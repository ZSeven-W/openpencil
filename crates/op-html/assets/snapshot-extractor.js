(function (options) {
  "use strict";

  // `options.root` narrows the capture to one element's subtree instead of
  // the whole page; anything else (including no argument at all, which is how
  // this file is pasted into a console) captures `document.body`. The emitted
  // JSON has the same shape either way: `snapshot.root` is a node object with
  // page-absolute `rect` coordinates, and the importer places whatever it
  // finds there at the document origin with its descendants relative to it.
  var requestedRoot =
    options && options.root && options.root.nodeType === 1 ? options.root : null;

  var MAX_NODES = 40000;
  var MAX_IMAGE_EDGE = 2048;
  var MAX_IMAGE_DATA_BYTES = 24 * 1024 * 1024;
  var GRAY_PLACEHOLDER =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
  // Keys are lowercase: SVG elements report a lowercase `tagName` (they are
  // not HTML elements), so every tag comparison in this file goes through
  // `tagOf` and matches lowercase or inline `<svg>` slips through as a plain
  // container and its `<path>` children paint as empty boxes.
  var SKIP_TAGS = {
    script: true,
    style: true,
    noscript: true,
    template: true,
    meta: true,
    link: true,
    head: true,
  };
  var MEDIA_TAGS = { img: true, svg: true, canvas: true, video: true };
  var ELEMENT_STYLE_KEYS = [
    "background-color",
    "background-image",
    // Placement of a `url()` background layer. Without these three a sprite
    // sheet imports as the whole sheet stretched over the box, which is how a
    // logo turns into the wrong (and squashed) mark.
    "background-size",
    "background-position",
    "background-repeat",
    // Per-corner radii: the `border-radius` shorthand serializes as up to
    // four lengths ("6px 6px 0px 0px"), which the importer cannot parse as a
    // single length, so it dropped every non-uniform radius.
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "box-shadow",
    "border",
    "opacity",
    "overflow",
    // The gradient-text idiom: `background-clip: text` paints the background
    // THROUGH the glyphs instead of behind them. Without this key the
    // importer painted the gradient as a solid bar over invisible text.
    "background-clip",
    "transform",
    "object-fit",
    "object-position",
    "mix-blend-mode",
    // Stacking inputs. CSS paints later siblings on top and lifts positioned
    // / z-indexed boxes out of document order; the canonical child array is
    // front-to-back. Without these the importer cannot reconstruct either,
    // and a full-bleed hero background buries everything it should sit under.
    "position",
    "z-index",
    // The parent's `display` is the third stacking input: CSS gives `z-index`
    // effect on a flex / grid *item* even while it stays `position: static`,
    // and that is exactly the negative-margin + z-index overlay idiom. Without
    // it the importer reads such an item as ordinary flow and inverts the
    // overlap. Pruned when it equals the initial value, so the common block
    // element still costs nothing.
    "display",
  ];
  var TEXT_STYLE_KEYS = [
    "color",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "line-height",
    "letter-spacing",
    "text-align",
    "white-space",
  ];
  // Per-run overrides carried on a folded inline block's `segments` (see
  // `buildInlineText`). These are the properties an inline `<a>` / `<code>` /
  // `<span>` changes against its block: colour, the monospace family and
  // smaller size of code, bold / italic, and link underlines.
  var SEGMENT_STYLE_KEYS = [
    "color",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "text-decoration-line",
  ];
  // Computed values that carry no information: the importer's own defaults
  // are identical, so emitting them only inflates the payload (they are the
  // majority of every element's style block on a real page).
  var STYLE_DEFAULTS = {
    "background-color": "rgba(0, 0, 0, 0)",
    "background-image": "none",
    "background-size": "auto",
    "background-position": "0% 0%",
    "background-repeat": "repeat",
    "border-top-left-radius": "0px",
    "border-top-right-radius": "0px",
    "border-bottom-right-radius": "0px",
    "border-bottom-left-radius": "0px",
    "box-shadow": "none",
    opacity: "1",
    overflow: "visible",
    transform: "none",
    "object-fit": "fill",
    "object-position": "50% 50%",
    "mix-blend-mode": "normal",
    "background-clip": "border-box",
    position: "static",
    "z-index": "auto",
    "white-space": "normal",
    "text-decoration-line": "none",
    display: "block",
  };

  var nodeCount = 0;
  var embeddedImageBytes = 0;
  var remoteImageCount = 0;
  var truncated = false;

  function round(value) {
    return Math.round(value * 100) / 100;
  }

  // Depth of `position: fixed` ancestry for the node being built. A fixed
  // box is positioned against the VIEWPORT: adding the scroll offset (right
  // for everything else) drops a scrolled-down page's navbar into the middle
  // of the document. Inside a fixed subtree, viewport coordinates ARE the
  // resting page coordinates.
  var fixedDepth = 0;

  function pageRect(rect) {
    var scrollLeft = fixedDepth > 0 ? 0 : window.scrollX;
    var scrollTop = fixedDepth > 0 ? 0 : window.scrollY;
    return {
      x: round(rect.left + scrollLeft),
      y: round(rect.top + scrollTop),
      w: round(rect.width),
      h: round(rect.height),
    };
  }

  function takeNode() {
    if (nodeCount >= MAX_NODES) {
      truncated = true;
      return false;
    }
    nodeCount += 1;
    return true;
  }

  function tagOf(element) {
    return (element.tagName || "").toLowerCase();
  }

  function copyStyles(computed, keys) {
    var result = {};
    keys.forEach(function (key) {
      var value = computed.getPropertyValue(key);
      if (value === "" || value === undefined) return;
      // `border` serializes as "<width> <style> <color>"; a zero width means
      // no border whatever the (always present) colour is.
      if (key === "border" && value.indexOf("0px") === 0) return;
      if (STYLE_DEFAULTS[key] === value) return;
      result[key] = value;
    });
    return result;
  }

  // Text styles with the colour that actually paints the glyphs.
  // `-webkit-text-fill-color` wins over `color` when they differ — the
  // gradient-text idiom sets it to `transparent` while `color` keeps its
  // value, which otherwise imports as solid-colour glyphs over the gradient
  // bar the background produced.
  function textPaintStyles(computed, keys) {
    var styles = copyStyles(computed, keys);
    var fill = computed.getPropertyValue("-webkit-text-fill-color");
    if (fill && fill !== computed.getPropertyValue("color")) {
      styles.color = fill;
    }
    return styles;
  }

  var BORDER_SIDES = ["top", "right", "bottom", "left"];

  // Element styles plus the per-side border longhands for the sides that
  // actually draw. The `border` shorthand serializes to "" whenever the four
  // sides differ, so a lone `border-bottom` divider (every table row on a
  // real page) reached the importer as "no border at all".
  function elementStyles(computed) {
    var result = copyStyles(computed, ELEMENT_STYLE_KEYS);
    BORDER_SIDES.forEach(function (side) {
      var width = computed.getPropertyValue("border-" + side + "-width");
      if (!width || parseFloat(width) <= 0) return;
      var style = computed.getPropertyValue("border-" + side + "-style");
      if (!style || style === "none" || style === "hidden") return;
      result["border-" + side + "-width"] = width;
      result["border-" + side + "-style"] = style;
      result["border-" + side + "-color"] =
        computed.getPropertyValue("border-" + side + "-color");
    });
    return result;
  }

  function childHasVisibleBox(element) {
    return Array.prototype.some.call(element.children, function (child) {
      var style = window.getComputedStyle(child);
      var rect = child.getBoundingClientRect();
      return (
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        Number(style.opacity) !== 0 &&
        (rect.width >= 0.5 || rect.height >= 0.5)
      );
    });
  }

  function visibleElement(element, computed, rect) {
    if (
      computed.display === "none" ||
      computed.visibility === "hidden" ||
      Number(computed.opacity) === 0
    ) {
      return false;
    }
    if (rect.width < 0.5 || rect.height < 0.5) {
      return childHasVisibleBox(element);
    }
    return true;
  }

  function buildText(textNode, parentComputed) {
    // Collapse runs of whitespace the way CSS does, but keep a single space at
    // either boundary: the gap between two inline runs lives at the end of one
    // text node, and trimming it welded neighbouring runs together
    // ("252 points by gyan" imported as "252 pointsby gyan"). A node that is
    // nothing but whitespace still drops out.
    //
    // A boundary space is only real when there is a sibling on that side. The
    // leading and trailing whitespace of an *only* child is the source file's
    // pretty-printing indentation, which CSS discards and which otherwise
    // imports as a phantom space — " Hello world " in a box measured for
    // "Hello world", i.e. every indented `<p>` starts one space too far right.
    //
    // A WRAPPED bare text run cannot be captured as one box: the union rect a
    // range reports anchors at the block's left edge and spans every line, and
    // a node re-wrapped there paints over whatever else shares those lines —
    // including a first line that actually started mid-line after an inline
    // sibling. Wrapped runs split into one single-line node per line box
    // instead (`buildTextLines`); a run the browser kept on one line stays a
    // single node. Returns an ARRAY of text nodes.
    var text = (textNode.textContent || "").replace(/\s+/g, " ");
    if (!text.trim()) return [];
    if (!textNode.previousSibling) text = text.replace(/^ /, "");
    if (!textNode.nextSibling) text = text.replace(/ $/, "");
    var range = document.createRange();
    range.selectNodeContents(textNode);
    var rect = range.getBoundingClientRect();
    // TRUE line count via vertical bands, not raw `rects.length` — a bidi
    // direction change fragments the rects of a single line, and reporting
    // that as "wrapped" cost the run its hug sizing and the importer's
    // single-line line-height clamp.
    var bands = lineBands(range.getClientRects());
    if (typeof range.detach === "function") range.detach();
    if (rect.width < 0.5 || rect.height < 0.5) return [];
    if (bands > 1) {
      var split = buildTextLines(textNode, parentComputed, bands);
      if (split.length) return split;
      // Splitting can fail (node budget, zero-progress guard); the union
      // box is still better than losing the text.
    }
    if (!takeNode()) return [];
    return [
      {
        kind: "text",
        rect: pageRect(rect),
        text: text,
        lines: bands || 1,
        styles: textPaintStyles(parentComputed, TEXT_STYLE_KEYS),
      },
    ];
  }

  // Count TRUE line boxes among a range's client rects. Nested inline boxes
  // and bidi runs contribute one rect each WITHIN a line, so raw
  // `rects.length` over-reports; rects arrive in content (= line) order and
  // are merged into vertical bands.
  //
  // Same-band means MOSTLY overlapping, not merely touching: a display
  // heading's tight leading (`line-height: 1.05` on 44px CJK) makes
  // consecutive glyph boxes overlap by a few pixels, and "any overlap"
  // merged real lines into one — the heading then imported as a single
  // overflowing line. Fragments that genuinely share a line (nested spans,
  // baseline-aligned size mixes, sub/sup) overlap by most of the smaller
  // height; consecutive tight lines only graze.
  function lineBands(rects) {
    var count = 0;
    var top = 0;
    var bottom = 0;
    for (var index = 0; index < rects.length; index += 1) {
      var rect = rects[index];
      if (rect.width < 0.5 || rect.height < 0.5) continue;
      var overlap = Math.min(rect.bottom, bottom) - Math.max(rect.top, top);
      var least = Math.min(rect.height, bottom - top);
      if (count > 0 && overlap > 0.6 * least) {
        if (rect.top < top) top = rect.top;
        if (rect.bottom > bottom) bottom = rect.bottom;
      } else {
        count += 1;
        top = rect.top;
        bottom = rect.bottom;
      }
    }
    return count;
  }

  // Largest `end` for which [start, end) still renders inside one line box,
  // by binary search over character offsets.
  function lineEndAfter(textNode, range, start, length) {
    var low = start + 1;
    var high = length;
    var best = start + 1;
    while (low <= high) {
      var mid = (low + high) >> 1;
      range.setStart(textNode, start);
      range.setEnd(textNode, mid);
      if (lineBands(range.getClientRects()) <= 1) {
        best = mid;
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }
    // Never leave half a surrogate pair on a line — a lone half cannot
    // survive the JSON round-trip into the importer.
    if (best > start && best < length) {
      var lead = textNode.textContent.charCodeAt(best - 1);
      var trail = textNode.textContent.charCodeAt(best);
      if (lead >= 0xd800 && lead <= 0xdbff && trail >= 0xdc00 && trail <= 0xdfff) {
        best = best - 1 > start ? best - 1 : best + 1;
      }
    }
    return best;
  }

  // One single-line text node per line box of a wrapped bare text run. Each
  // line's rect comes from its own subrange, so a line that starts mid-line
  // (after an atomic inline sibling) keeps its true x instead of smearing
  // from the block's left edge.
  function buildTextLines(textNode, parentComputed, lineCount) {
    var content = textNode.textContent || "";
    var styles = textPaintStyles(parentComputed, TEXT_STYLE_KEYS);
    var range = document.createRange();
    var out = [];
    var start = 0;
    var guard = lineCount + 8;
    while (start < content.length && guard > 0) {
      guard -= 1;
      var end = lineEndAfter(textNode, range, start, content.length);
      if (end <= start) break;
      range.setStart(textNode, start);
      range.setEnd(textNode, end);
      var rect = range.getBoundingClientRect();
      var text = content.slice(start, end).replace(/\s+/g, " ");
      // Interior line edges are collapsed wrap points and always trim; the
      // run's outer boundaries follow the same sibling rule as the
      // single-line path.
      if (start > 0 || !textNode.previousSibling) text = text.replace(/^ /, "");
      if (end < content.length || !textNode.nextSibling) text = text.replace(/ $/, "");
      start = end;
      if (!text.trim() || rect.width < 0.5 || rect.height < 0.5) continue;
      if (!takeNode()) return [];
      out.push({
        kind: "text",
        rect: pageRect(rect),
        text: text,
        lines: 1,
        styles: styles,
      });
    }
    if (typeof range.detach === "function") range.detach();
    // All-or-nothing: a partial split would silently drop the tail text.
    return start >= content.length ? out : [];
  }

  function scaledCanvas(width, height) {
    var longest = Math.max(width, height, 1);
    var scale = Math.min(1, MAX_IMAGE_EDGE / longest);
    var canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(width * scale));
    canvas.height = Math.max(1, Math.round(height * scale));
    return canvas;
  }

  function keepDataUrl(dataUrl, fallbackUrl) {
    if (!dataUrl || embeddedImageBytes + dataUrl.length > MAX_IMAGE_DATA_BYTES) {
      remoteImageCount += 1;
      return { src: fallbackUrl || GRAY_PLACEHOLDER, tainted: true };
    }
    embeddedImageBytes += dataUrl.length;
    return { src: dataUrl, tainted: false };
  }

  function rasterizeImage(image) {
    var fallback = image.currentSrc || image.src || GRAY_PLACEHOLDER;
    if (!image.complete || !image.naturalWidth || !image.naturalHeight) {
      remoteImageCount += 1;
      return { src: fallback, tainted: true };
    }
    try {
      var canvas = scaledCanvas(image.naturalWidth, image.naturalHeight);
      canvas
        .getContext("2d")
        .drawImage(image, 0, 0, canvas.width, canvas.height);
      return keepDataUrl(canvas.toDataURL("image/png"), fallback);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: fallback, tainted: true };
    }
  }

  var SVG_SHAPE_TAGS = {
    path: true,
    rect: true,
    circle: true,
    ellipse: true,
    line: true,
    polygon: true,
    polyline: true,
  };

  var NUMBER_PATTERN = /-?\d*\.?\d+(?:e[-+]?\d+)?/gi;

  // Twice the shoelace area of a closed point ring. Positive means clockwise
  // in SVG's y-down space.
  function signedArea(points) {
    var total = 0;
    for (var index = 0; index < points.length; index += 1) {
      var current = points[index];
      var next = points[(index + 1) % points.length];
      total += current[0] * next[1] - next[0] * current[1];
    }
    return total;
  }

  // `points` list → path data with a normalized (clockwise) winding.
  //
  // WINDING IS LOAD-BEARING. Every subpath merged into the single emitted `d`
  // has to wind the same way: the browser fills each shape element
  // independently — so winding never interacts there — but the merged path
  // under the `nonzero` rule *cancels* a contained subpath that winds the
  // other way, punching a hole where the page paints solid. Every primitive
  // this file generates therefore winds clockwise: `rect` (right / down /
  // left) and the rounded-rect arcs (`sweep 1`) already did, the ellipse arcs
  // are emitted with `sweep 1` for the same reason, and a `points` ring is
  // reversed here when the author wrote it counter-clockwise.
  function pointsToPathData(element, closed) {
    var raw = (element.getAttribute("points") || "").match(NUMBER_PATTERN);
    if (!raw || raw.length < 4) return "";
    var points = [];
    for (var index = 0; index + 1 < raw.length; index += 2) {
      var x = parseFloat(raw[index]);
      var y = parseFloat(raw[index + 1]);
      if (!isFinite(x) || !isFinite(y)) return "";
      points.push([x, y]);
    }
    if (signedArea(points) < 0) points.reverse();
    var data = "";
    for (var at = 0; at < points.length; at += 1) {
      data += (at ? " " : "M") + points[at][0] + " " + points[at][1];
    }
    return data + (closed ? "Z" : "");
  }

  // Shape element → SVG path data in the element's own user units.
  function shapeToPathData(element, tag) {
    var attr = function (name, fallback) {
      var value = parseFloat(element.getAttribute(name));
      return isFinite(value) ? value : fallback;
    };
    if (tag === "path") return element.getAttribute("d") || "";
    if (tag === "rect") {
      var x = attr("x", 0);
      var y = attr("y", 0);
      var w = attr("width", 0);
      var h = attr("height", 0);
      if (w <= 0 || h <= 0) return "";
      var rx = Math.min(attr("rx", attr("ry", 0)), w / 2);
      var ry = Math.min(attr("ry", attr("rx", 0)), h / 2);
      if (rx > 0 && ry > 0) {
        return (
          "M" + (x + rx) + " " + y + "H" + (x + w - rx) +
          "A" + rx + " " + ry + " 0 0 1 " + (x + w) + " " + (y + ry) +
          "V" + (y + h - ry) +
          "A" + rx + " " + ry + " 0 0 1 " + (x + w - rx) + " " + (y + h) +
          "H" + (x + rx) +
          "A" + rx + " " + ry + " 0 0 1 " + x + " " + (y + h - ry) +
          "V" + (y + ry) +
          "A" + rx + " " + ry + " 0 0 1 " + (x + rx) + " " + y + "Z"
        );
      }
      return "M" + x + " " + y + "h" + w + "v" + h + "h" + -w + "Z";
    }
    if (tag === "circle" || tag === "ellipse") {
      var cx = attr("cx", 0);
      var cy = attr("cy", 0);
      var radiusX = tag === "circle" ? attr("r", 0) : attr("rx", 0);
      var radiusY = tag === "circle" ? attr("r", 0) : attr("ry", 0);
      if (radiusX <= 0 || radiusY <= 0) return "";
      // `sweep 1` — clockwise, matching the `rect` branch. See the winding
      // note on `pointsToPathData`: an ellipse drawn the other way around
      // erases a rect it sits inside once the two are merged into one path.
      return (
        "M" + (cx - radiusX) + " " + cy +
        "a" + radiusX + " " + radiusY + " 0 1 1 " + radiusX * 2 + " 0" +
        "a" + radiusX + " " + radiusY + " 0 1 1 " + -radiusX * 2 + " 0Z"
      );
    }
    if (tag === "line") {
      return (
        "M" + attr("x1", 0) + " " + attr("y1", 0) +
        "L" + attr("x2", 0) + " " + attr("y2", 0)
      );
    }
    return pointsToPathData(element, tag === "polygon");
  }

  // A CTM entry below this is rounding noise rather than a real skew.
  var CTM_SKEW_EPSILON = 1e-6;

  // Union of two user-unit boxes; either may be null.
  function unionBox(left, right) {
    if (!left) return right;
    if (!right) return left;
    var x = Math.min(left.x, right.x);
    var y = Math.min(left.y, right.y);
    return {
      x: x,
      y: y,
      width: Math.max(left.x + left.width, right.x + right.width) - x,
      height: Math.max(left.y + left.height, right.y + right.height) - y,
    };
  }

  function shapeBox(element) {
    if (typeof element.getBBox !== "function") return null;
    try {
      var box = element.getBBox();
      if (!box || !isFinite(box.x) || !isFinite(box.y)) return null;
      if (!isFinite(box.width) || !isFinite(box.height)) return null;
      return box;
    } catch (_error) {
      return null;
    }
  }

  // Vectorize an inline `<svg>` into one path.
  //
  // Skia decodes PNG / JPEG / GIF / WebP — not SVG — so an SVG data URI
  // reaches the canvas as an undecodable image and paints as a placeholder.
  // Icon sets are the overwhelming majority of inline SVG on a real page, and
  // they are plain filled paths in a single colour, which the canonical
  // schema paints natively as a `path` node.
  //
  // Everything is emitted in user units and the reported rect is the art's own
  // page-space bounding box, because the renderer fits path data to the node
  // box by its bounds: taking the box from the shapes' own bounds keeps the
  // artwork's aspect and position exact instead of stretching it to the
  // element rect. Anything that carries a per-element transform or
  // references external paint (`<use>`, gradients, masks, nested images) is
  // left to the image path.
  //
  // Returns an array of `{ d, fill, fillRule?, rect }` fragments, one per
  // consecutive same-fill shape group — data `buildImage` merges into the
  // image node (single group) or expands into per-colour path children,
  // never a node of its own.
  function vectorizeSvg(svg) {
    if (typeof svg.getScreenCTM !== "function") return null;
    var rootMatrix = svg.getScreenCTM();
    if (!rootMatrix) return null;
    // The rect this emits is an axis-aligned box, so a rotated / skewed CTM
    // cannot be expressed by it — mapping a box through one and keeping only
    // the diagonal produces dimensions that belong to no shape on the page.
    // Those SVGs go down the image path, which carries the transform in the
    // element rect the browser already resolved.
    if (
      Math.abs(rootMatrix.b) > CTM_SKEW_EPSILON ||
      Math.abs(rootMatrix.c) > CTM_SKEW_EPSILON ||
      !(rootMatrix.a > 0) ||
      !(rootMatrix.d > 0)
    ) {
      return null;
    }
    var nodes = svg.querySelectorAll("*");
    // Shapes grouped by CONSECUTIVE fill + fill-rule. A single-colour icon
    // still merges into one path exactly as before; multi-colour flat art (a
    // brand logo) becomes one path per colour group. Only consecutive shapes
    // merge because the groups paint in document order — merging same-fill
    // shapes across an intervening colour would hoist them over it.
    var groups = [];
    var current = null;
    for (var index = 0; index < nodes.length; index += 1) {
      var node = nodes[index];
      var tag = tagOf(node);
      if (tag === "title" || tag === "desc" || tag === "metadata" || tag === "g") {
        // A `<g>` may only carry grouping; a transform on it moves its
        // children out of the shared user space this path assumes.
        if (tag === "g" && node.getAttribute("transform")) return null;
        continue;
      }
      if (!SVG_SHAPE_TAGS[tag]) return null;
      if (node.getAttribute("transform")) return null;
      var style = window.getComputedStyle(node);
      if (style.display === "none" || style.visibility === "hidden") continue;
      // Stroked icon sets (lucide et al.) need stroke geometry the schema's
      // single fill cannot express.
      if (style.stroke && style.stroke !== "none") return null;
      // An unstroked `<line>` paints nothing at all (a line encloses no area,
      // so it cannot be filled); a stroked one already bailed above.
      if (tag === "line") continue;
      var shapeFill = style.fill;
      if (!shapeFill || shapeFill === "none") continue;
      if (shapeFill.indexOf("url(") === 0) return null;
      var segment = shapeToPathData(node, tag);
      if (!segment) return null;
      // Per-shape bounds — NOT `svg.getBBox()`, which measures every shape
      // in the tree including the ones skipped above. Material's icon set
      // opens each glyph with `<path fill="none" d="M0 0h24v24H0z"/>`: a
      // full-viewBox sizing rect that paints nothing but doubles the root
      // bbox, so sizing from it scaled and shifted every glyph by exactly
      // that factor.
      var segmentBox = shapeBox(node);
      if (!segmentBox) return null;
      var shapeRule = style.getPropertyValue("fill-rule") || "nonzero";
      if (!current || current.fill !== shapeFill || current.rule !== shapeRule) {
        current = { fill: shapeFill, rule: shapeRule, data: [], box: null };
        groups.push(current);
      }
      current.data.push(segment);
      current.box = unionBox(current.box, segmentBox);
    }
    if (!groups.length) return null;
    var fragments = [];
    for (var at = 0; at < groups.length; at += 1) {
      var group = groups[at];
      if (!group.box) return null;
      if (!(group.box.width > 0) || !(group.box.height > 0)) return null;
      // User units → page pixels through the root CTM, so viewBox scaling
      // and `preserveAspectRatio` are already accounted for. The CTM is
      // known axis-aligned by the guard above, so the off-diagonal terms
      // are zero.
      var fragment = {
        rect: pageRect({
          left: rootMatrix.a * group.box.x + rootMatrix.e,
          top: rootMatrix.d * group.box.y + rootMatrix.f,
          width: rootMatrix.a * group.box.width,
          height: rootMatrix.d * group.box.height,
        }),
        d: group.data.join(" "),
        fill: group.fill,
      };
      if (!(fragment.rect.w > 0) || !(fragment.rect.h > 0)) return null;
      if (group.rule === "evenodd") fragment.fillRule = "evenodd";
      fragments.push(fragment);
    }
    return fragments;
  }

  // Paint properties inlined onto every shape of a serialized `<svg>` clone.
  var SVG_PAINT_KEYS = [
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-dasharray",
    "fill-rule",
    "opacity",
    "fill-opacity",
    "stroke-opacity",
  ];

  // Inline each element's COMPUTED paint into the clone as presentation
  // attributes. A standalone data URI loses every stylesheet the page used —
  // a `.card svg { stroke: #fff }` rule outranks the markup's own
  // `stroke="currentColor"`, so without this the serialized icon fell back
  // to the inherited text colour (a theme accent) instead of the colour the
  // page actually painted with.
  function inlineComputedPaint(source, clone) {
    var sourceNodes = source.querySelectorAll("*");
    var cloneNodes = clone.querySelectorAll("*");
    var limit = Math.min(sourceNodes.length, cloneNodes.length);
    for (var index = 0; index < limit; index += 1) {
      var computed = window.getComputedStyle(sourceNodes[index]);
      for (var at = 0; at < SVG_PAINT_KEYS.length; at += 1) {
        var key = SVG_PAINT_KEYS[at];
        var value = computed.getPropertyValue(key);
        if (!value) continue;
        // Computed url() refs serialize with quotes; the attribute grammar
        // wants them bare.
        cloneNodes[index].setAttribute(key, value.replace(/url\("([^"]*)"\)/g, "url($1)"));
      }
    }
  }

  function encodeSvg(svg, computed, rect) {
    try {
      var clone = svg.cloneNode(true);
      if (!clone.getAttribute("xmlns")) {
        clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      }
      // Icon sets (GitHub's octicons, lucide, heroicons) paint with
      // `fill="currentColor"`, which resolves against the inherited text
      // colour. A standalone data URI inherits nothing, so bake the computed
      // colour in or every icon rasterizes black — or, on a dark page,
      // invisible.
      var color = computed.getPropertyValue("color");
      if (color) clone.style.color = color;
      inlineComputedPaint(svg, clone);
      // Serialize at the used box size so the rasterizer scales the viewBox
      // exactly the way the page did; CSS-sized SVGs carry no useful
      // width/height attributes.
      if (rect.width >= 1) clone.setAttribute("width", round(rect.width));
      if (rect.height >= 1) clone.setAttribute("height", round(rect.height));
      var xml = new XMLSerializer().serializeToString(clone);
      var encoded = btoa(unescape(encodeURIComponent(xml)));
      return keepDataUrl("data:image/svg+xml;base64," + encoded, GRAY_PLACEHOLDER);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: GRAY_PLACEHOLDER, tainted: true };
    }
  }

  function captureCanvas(canvas) {
    try {
      return keepDataUrl(canvas.toDataURL("image/png"), GRAY_PLACEHOLDER);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: GRAY_PLACEHOLDER, tainted: true };
    }
  }

  function captureVideo(video) {
    // A hero background video is usually cross-origin (tainting the canvas)
    // and often has no decoded frame yet, so the poster is the honest
    // fallback: a remote poster URL still shows the intended artwork, a gray
    // 1×1 placeholder stretched full-bleed shows nothing.
    var poster = video.poster || GRAY_PLACEHOLDER;
    try {
      var width = video.videoWidth || video.clientWidth;
      var height = video.videoHeight || video.clientHeight;
      if (!video.videoWidth || video.readyState < 2) {
        remoteImageCount += 1;
        return { src: poster, tainted: true };
      }
      var canvas = scaledCanvas(width, height);
      canvas
        .getContext("2d")
        .drawImage(video, 0, 0, canvas.width, canvas.height);
      return keepDataUrl(canvas.toDataURL("image/png"), poster);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: poster, tainted: true };
    }
  }

  function buildImage(element, computed, rect) {
    if (!takeNode()) return null;
    var tag = tagOf(element);
    var fragments = tag === "svg" ? vectorizeSvg(element) : null;
    if (fragments && fragments.length > 1) {
      // Multi-colour flat art (a brand logo): one path node per colour
      // group, wrapped in a plain container at the element's box and painted
      // in document order. This replaces the raster fallback entirely — an
      // SVG data URI is undecodable to the skia importer anyway, so the
      // native path paint is the only rendering these ever get. Each child
      // still carries a placeholder `src` so an importer that predates `d`
      // degrades to its usual unknown-image box instead of crashing.
      var children = [];
      for (var at = 0; at < fragments.length; at += 1) {
        if (!takeNode()) break;
        var piece = {
          kind: "image",
          tag: "svg",
          rect: fragments[at].rect,
          src: GRAY_PLACEHOLDER,
          styles: {},
          d: fragments[at].d,
          fill: fragments[at].fill,
          vectorRect: fragments[at].rect,
        };
        if (fragments[at].fillRule) piece.fillRule = fragments[at].fillRule;
        children.push(piece);
      }
      return {
        kind: "element",
        tag: "svg",
        rect: pageRect(rect),
        styles: elementStyles(computed),
        children: children,
      };
    }
    var captured;
    if (tag === "img") captured = rasterizeImage(element);
    else if (tag === "svg") captured = encodeSvg(element, computed, rect);
    else if (tag === "canvas") captured = captureCanvas(element);
    else captured = captureVideo(element);
    var result = {
      kind: "image",
      tag: tag,
      rect: pageRect(rect),
      src: captured.src,
      styles: elementStyles(computed),
    };
    if (captured.tainted) result.tainted = true;
    // Single-colour vector geometry rides ALONGSIDE the image serialization
    // rather than replacing it, and the node keeps `kind: "image"`. An
    // importer that predates the vector fields reads exactly the node it
    // always read (it dispatches on `kind`, and an unknown one is dropped
    // with a warning — the icon would vanish rather than degrade); one that
    // understands them prefers `d` and ignores `src`. `vectorRect` is the
    // artwork's own bounds, which is what the path node has to be sized to;
    // `rect` stays the element's used box so the image fallback keeps
    // landing where it did.
    if (fragments && fragments.length === 1) {
      result.d = fragments[0].d;
      result.fill = fragments[0].fill;
      result.vectorRect = fragments[0].rect;
      if (fragments[0].fillRule) result.fillRule = fragments[0].fillRule;
    }
    return result;
  }

  // Inline elements that fold into their block's text flow rather than
  // becoming a positioned node of their own. Anything with its own box
  // (inline-block, media, a shadow root) is deliberately absent.
  var INLINE_FLOW_TAGS = {
    a: true, code: true, span: true, b: true, strong: true, i: true,
    em: true, u: true, s: true, small: true, mark: true, sub: true,
    sup: true, abbr: true, cite: true, q: true, kbd: true, samp: true,
    time: true, label: true, bdi: true, bdo: true, del: true, ins: true,
    strike: true, big: true, tt: true, nobr: true,
  };

  // Text, an ignorable node, or an inline element whose subtree is inline all
  // the way down — i.e. a node that joins a normal inline flow.
  function isInlineFlow(node) {
    if (node.nodeType === Node.TEXT_NODE) return true;
    if (node.nodeType !== Node.ELEMENT_NODE) return true;
    var tag = tagOf(node);
    if (SKIP_TAGS[tag]) return true;
    if (MEDIA_TAGS[tag] || node.shadowRoot || !INLINE_FLOW_TAGS[tag]) return false;
    var computed = window.getComputedStyle(node);
    // An undisplayed inline element renders nothing at all, so it neither
    // joins nor breaks the flow around it — splitting a run on one would
    // leave the second half starting mid-line for no visible reason. Its
    // text is excluded by the segment walker.
    if (computed.display === "none") return true;
    if (computed.display !== "inline") return isFoldableTextChip(node, computed);
    return Array.prototype.every.call(node.childNodes, isInlineFlow);
  }

  // All of the range's fragment rects sit in one line box. A range reports
  // one client rect per inline box fragment, so several rects on a single
  // line are normal (each nested span contributes its own) and "one line"
  // cannot be `rects.length === 1` — the band walk decides, with the same
  // mostly-overlapping criterion that keeps tight-leading lines apart.
  function contentsOnOneLine(element) {
    var range = document.createRange();
    range.selectNodeContents(element);
    var bands = lineBands(range.getClientRects());
    if (typeof range.detach === "function") range.detach();
    return bands <= 1;
  }

  // An atomic inline (`display: inline-block`) still reads as part of its
  // parent's text flow when it is nothing but undecorated text on one line —
  // the idiom behind search-result date chips ("2026年5月8日 — ") and nowrap
  // ellipsis spans, which otherwise block the fold and smear the paragraph
  // around them. Anything with its own visible box (a background, a border)
  // or with internally wrapped text genuinely is a box, and stays one.
  function isFoldableTextChip(element, computed) {
    if (computed.display !== "inline-block") return false;
    if (
      computed.backgroundColor !== "rgba(0, 0, 0, 0)" ||
      computed.backgroundImage !== "none"
    ) {
      return false;
    }
    for (var index = 0; index < BORDER_SIDES.length; index += 1) {
      var side = BORDER_SIDES[index];
      var width = computed.getPropertyValue("border-" + side + "-width");
      var style = computed.getPropertyValue("border-" + side + "-style");
      if (parseFloat(width) > 0 && style !== "none" && style !== "hidden") {
        return false;
      }
    }
    if (!Array.prototype.every.call(element.childNodes, isInlineFlow)) {
      return false;
    }
    return contentsOnOneLine(element);
  }

  // Does this element lay inline children out as wrapping text lines, the way
  // a block does? Only under these displays may consecutive inline children
  // fold into one flowing run; a flex / grid parent turns each child into its
  // own *item* (side by side, with gaps), which a folded run cannot express.
  function blockLikeDisplay(computed) {
    var display = computed.display;
    return (
      display === "block" ||
      display === "inline-block" ||
      display === "list-item" ||
      // A block formatting context lays inline children out exactly like a
      // block. Chrome computes the `-webkit-line-clamp` truncation idiom
      // (`display: -webkit-box` + vertical orient — every search-result
      // snippet) as `flow-root`, which is how those paragraphs escaped the
      // fold and smeared.
      display === "flow-root" ||
      display.indexOf("table-cell") !== -1 ||
      // Engines that still compute the clamp idiom as `-webkit-box`
      // (console-pasted captures on Safari). Only the *vertical* orient is
      // block-like; its horizontal cousin is old flexbox — children side by
      // side as items — which a folded run cannot express.
      (display === "-webkit-box" &&
        computed.getPropertyValue("-webkit-box-orient") === "vertical")
    );
  }

  // The `href` of the nearest enclosing `<a>` up to (and including) `root`.
  function nearestHref(node, root) {
    for (var element = node; element; element = element.parentElement) {
      if (tagOf(element) === "a") {
        var href = element.getAttribute("href");
        if (href) return href;
      }
      if (element === root) break;
    }
    return null;
  }

  // Whether a text node's owner (walked up to the run's `stop` parent) is
  // invisible plumbing rather than rendered text: script / style sources and
  // anything inside a `display: none` subtree. The folding TreeWalker visits
  // those text nodes; the browser's text flow does not.
  function segmentTextHidden(owner, stop) {
    for (var element = owner; element; element = element.parentElement) {
      if (SKIP_TAGS[tagOf(element)]) return true;
      if (window.getComputedStyle(element).display === "none") return true;
      if (element === stop) break;
    }
    return false;
  }

  // Walk the inline text of `nodes` (consecutive siblings under `parent`) in
  // document order into styled runs, collapsing whitespace across inline
  // boundaries as CSS does: `\s+` -> one space, a straddling boundary space
  // -> one, run-edge space dropped.
  function inlineSegments(parent, nodes) {
    var segments = [];
    var spaceOpen = true;
    function addText(textNode) {
      var owner = textNode.parentElement || parent;
      if (owner !== parent && segmentTextHidden(owner, parent)) return;
      var text = (textNode.textContent || "").replace(/\s+/g, " ");
      if (spaceOpen && text.charAt(0) === " ") text = text.slice(1);
      if (!text) return;
      spaceOpen = text.charAt(text.length - 1) === " ";
      var href = nearestHref(owner, parent);
      var styles = textPaintStyles(window.getComputedStyle(owner), SEGMENT_STYLE_KEYS);
      var key = JSON.stringify(styles) + " " + (href || "");
      var previous = segments[segments.length - 1];
      if (previous && previous.key === key) {
        previous.text += text;
      } else {
        segments.push({ text: text, styles: styles, href: href, key: key });
      }
    }
    for (var index = 0; index < nodes.length; index += 1) {
      var node = nodes[index];
      if (node.nodeType === Node.TEXT_NODE) {
        addText(node);
      } else if (node.nodeType === Node.ELEMENT_NODE) {
        var walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT, null);
        var textNode;
        while ((textNode = walker.nextNode())) {
          addText(textNode);
        }
      }
    }
    if (segments.length) {
      var last = segments[segments.length - 1];
      last.text = last.text.replace(/ $/, "");
      if (!last.text) segments.pop();
    }
    return segments;
  }

  // One folded text node for one inline formatting run. `nodes` is either
  // every child of a fully-inline block or a maximal run of consecutive
  // inline-flow siblings between block-level children; either way the run
  // starts and ends at a line-box boundary (block siblings force breaks), so
  // the range's box is an honest wrap box: position the run there once and
  // wrap it there — never one node per inline child stacked at the block
  // origin. Inline box decorations (a `<code>` pill background, a badge
  // border) are the one thing it cannot carry and are dropped.
  function buildInlineText(parent, nodes, blockComputed) {
    var range = document.createRange();
    range.setStartBefore(nodes[0]);
    range.setEndAfter(nodes[nodes.length - 1]);
    var rect = range.getBoundingClientRect();
    // TRUE line count (vertical bands): raw `getClientRects().length` counts
    // one rect per inline fragment, which classified one-line footers with
    // several styled spans as wrapped — costing them both the hug sizing and
    // the importer's single-line line-height clamp.
    var lines = lineBands(range.getClientRects());
    if (typeof range.detach === "function") range.detach();
    if (rect.width < 0.5 || rect.height < 0.5 || !takeNode()) return null;
    var segments = inlineSegments(parent, nodes);
    var text = "";
    for (var index = 0; index < segments.length; index += 1) {
      text += segments[index].text;
    }
    if (!text.trim()) return null;
    var emitted = segments.map(function (segment) {
      var out = { text: segment.text, styles: segment.styles };
      if (segment.href) out.href = segment.href;
      return out;
    });
    return {
      kind: "text",
      rect: pageRect(rect),
      text: text,
      lines: lines || 1,
      styles: textPaintStyles(blockComputed, TEXT_STYLE_KEYS),
      segments: emitted,
    };
  }

  // Emit one run of consecutive inline-flow siblings: mixed content (bare
  // text plus inline elements) folds into a single flowing text node, and
  // everything else keeps the per-child capture — a lone bare text keeps the
  // plain `buildText` shape, and a lone element keeps its own box, which is
  // what preserves a single `<code>` pill's background.
  function buildInlineRun(parent, nodes, parentComputed) {
    var sawElement = false;
    var sawText = false;
    for (var index = 0; index < nodes.length; index += 1) {
      var node = nodes[index];
      if (node.nodeType === Node.ELEMENT_NODE && !SKIP_TAGS[tagOf(node)]) {
        sawElement = true;
      } else if (node.nodeType === Node.TEXT_NODE && node.textContent.trim()) {
        sawText = true;
      }
    }
    if (sawElement && (sawText || nodes.length > 1)) {
      var folded = buildInlineText(parent, nodes, parentComputed);
      return folded ? [folded] : [];
    }
    var out = [];
    for (var at = 0; at < nodes.length; at += 1) {
      var child = nodes[at];
      if (child.nodeType === Node.TEXT_NODE) {
        var texts = buildText(child, parentComputed);
        for (var t = 0; t < texts.length; t += 1) out.push(texts[t]);
      } else if (child.nodeType === Node.ELEMENT_NODE) {
        var mapped = buildElement(child);
        if (mapped) out.push(mapped);
      }
    }
    return out;
  }

  function buildElement(element) {
    var tag = tagOf(element);
    if (SKIP_TAGS[tag]) return null;
    // Capture chrome injected by the OpenPencil extension (the element-picker
    // overlay, its highlight box and labels) marks itself with this
    // attribute. It is never page content, however it ends up still mounted
    // at capture time.
    if (element.hasAttribute && element.hasAttribute("data-openpencil-ui")) {
      return null;
    }
    var computed = window.getComputedStyle(element);
    var rect = element.getBoundingClientRect();
    if (!visibleElement(element, computed, rect)) return null;
    if (computed.position === "fixed") {
      fixedDepth += 1;
      try {
        return buildElementBody(element, tag, computed, rect);
      } finally {
        fixedDepth -= 1;
      }
    }
    return buildElementBody(element, tag, computed, rect);
  }

  function buildElementBody(element, tag, computed, rect) {
    if (MEDIA_TAGS[tag]) {
      return buildImage(element, computed, rect);
    }
    if (!takeNode()) return null;
    // Consecutive inline-flow children of a block-like parent fold into one
    // flowing text node per run (see `buildInlineText`); block-level children
    // break runs apart, so a paragraph interrupted by a list or a figure
    // still folds the text around it. Non-block parents (flex / grid rows)
    // keep the per-child capture: their children are items, not a text flow.
    var children = [];
    var foldRuns = blockLikeDisplay(computed);
    var run = [];
    var flushRun = function () {
      if (!run.length) return;
      var emitted = buildInlineRun(element, run, computed);
      for (var at = 0; at < emitted.length; at += 1) children.push(emitted[at]);
      run = [];
    };
    var collect = function (child) {
      if (truncated) return;
      if (foldRuns && isInlineFlow(child)) {
        run.push(child);
        return;
      }
      flushRun();
      if (child.nodeType === Node.TEXT_NODE) {
        var texts = buildText(child, computed);
        for (var t = 0; t < texts.length; t += 1) children.push(texts[t]);
      } else if (child.nodeType === Node.ELEMENT_NODE) {
        var mapped = buildElement(child);
        if (mapped) children.push(mapped);
      }
    };
    // An open shadow root holds everything a web component actually renders.
    // Walking only `childNodes` captured such an element as an empty box —
    // GitHub's `<relative-time>` timestamps, and every design system built on
    // custom elements, vanished that way. Shadow and light children are
    // separate subtrees, so a run never spans the boundary.
    if (element.shadowRoot) {
      Array.prototype.forEach.call(element.shadowRoot.childNodes, collect);
      flushRun();
    }
    Array.prototype.forEach.call(element.childNodes, collect);
    flushRun();
    return {
      kind: "element",
      tag: tag,
      rect: pageRect(rect),
      styles: elementStyles(computed),
      children: children,
    };
  }

  // A picked element can itself be an <img>/<svg>/<canvas>/<video>, which
  // maps to an image node. The importer reads `snapshot.root` as a container,
  // so wrap that one case in a synthetic element of the same size rather than
  // emitting an image at the top level.
  function buildRoot(element) {
    var node = buildElement(element);
    if (!node || node.kind === "element") return node;
    return {
      kind: "element",
      tag: "div",
      rect: node.rect,
      styles: {},
      children: [node],
    };
  }

  var target = requestedRoot || document.body;
  if (!target) {
    console.error("OpenPencil snapshot: document.body is not available");
    return;
  }
  var root = buildRoot(target);
  if (!root) {
    console.error(
      requestedRoot
        ? "OpenPencil snapshot: the selected element is not visible"
        : "OpenPencil snapshot: the page has no visible body",
    );
    return;
  }
  // The page's own backdrop. A subtree capture (element pick) usually starts
  // below the element that carries it, and a dark-theme page rendered onto
  // the importer's white default reads as washed-out text on the wrong
  // canvas.
  function pageBackground() {
    var nodes = [document.body, document.documentElement];
    for (var index = 0; index < nodes.length; index += 1) {
      if (!nodes[index]) continue;
      var color = window.getComputedStyle(nodes[index]).backgroundColor;
      if (color && color !== "transparent" && !/, *0\)$/.test(color)) return color;
    }
    return "rgb(255, 255, 255)";
  }

  var snapshot = {
    version: 1,
    source: window.location.href,
    title: document.title,
    background: pageBackground(),
    viewport: {
      width: round(window.innerWidth),
      height: round(window.innerHeight),
    },
    root: root,
  };
  if (truncated) snapshot.truncated = true;
  var output = JSON.stringify(snapshot, null, 2);

  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(output).catch(function (error) {
      console.warn("OpenPencil snapshot: clipboard copy failed", error);
    });
  }
  var blobUrl = URL.createObjectURL(
    new Blob([output], { type: "application/json" }),
  );
  var download = document.createElement("a");
  download.href = blobUrl;
  download.download = "snapshot.json";
  download.click();
  setTimeout(function () {
    URL.revokeObjectURL(blobUrl);
  }, 0);
  console.log("OpenPencil snapshot ready", {
    nodes: nodeCount,
    bytes: output.length,
    embeddedImageBytes: embeddedImageBytes,
    remoteImages: remoteImageCount,
    truncated: truncated,
  });
  // The argument is optional on purpose. Pasted into a devtools console the
  // global is undefined and this captures the whole page, exactly as before;
  // a host that wants a subtree sets `{ root: <element> }` on the global
  // first (the Chrome extension does, in its own isolated world).
})(typeof globalThis === "undefined" ? null : globalThis.openpencilSnapshotOptions);
