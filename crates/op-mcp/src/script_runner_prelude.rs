//! The embedded JavaScript prelude evaluated before every sandboxed script
//! (split out of `script_runner.rs` at the 800-line cap). It defines the
//! `I(parent, obj)` / `K(...)` / `U(nodeId, patch)` recorders plus their
//! normalization helpers (centered-frame expansion, text defaults, icon-name
//! canonicalization, divider re-parenting, foreground-contrast repair), and
//! stubs the unsupported ops + `console`.

pub(super) const PRELUDE: &str = r##"
var __cjkText = /(?:\p{Script=Han}|\p{Script=Hiragana}|\p{Script=Katakana}|\p{Script=Hangul})/u;
function __hasCjkContent(content) {
  if (typeof content === "string") return __cjkText.test(content);
  if (!Array.isArray(content)) return false;
  var text = "";
  for (var i = 0; i < content.length; i++) {
    var segment = content[i];
    if (segment == null || typeof segment !== "object" || typeof segment.text !== "string") return false;
    text += segment.text;
  }
  return __cjkText.test(text);
}
var __insertBindings = Object.create(null);
function __hasOwn(obj, key) {
  return Object.prototype.hasOwnProperty.call(obj, key);
}
function __parseOpaqueHex(value) {
  if (typeof value !== "string") return null;
  var hex = value.trim();
  var digits;
  if (/^#[0-9a-fA-F]{3}$/.test(hex)) {
    digits = hex.slice(1).replace(/./g, function (ch) { return ch + ch; });
  } else if (/^#[0-9a-fA-F]{6}$/.test(hex)) {
    digits = hex.slice(1);
  } else if (/^#[0-9a-fA-F]{8}$/.test(hex) && hex.slice(7).toLowerCase() === "ff") {
    digits = hex.slice(1, 7);
  } else {
    return null;
  }
  return {
    color: hex,
    rgb: [
      parseInt(digits.slice(0, 2), 16),
      parseInt(digits.slice(2, 4), 16),
      parseInt(digits.slice(4, 6), 16)
    ]
  };
}
function __opaqueSolidFill(fill, nodeOpacity) {
  if (typeof nodeOpacity === "number" && nodeOpacity !== 1) return null;
  var paint = fill;
  var shape = "object";
  if (typeof fill === "string") {
    var direct = __parseOpaqueHex(fill);
    return direct == null ? null : { color: direct.color, rgb: direct.rgb, shape: "string" };
  }
  if (Array.isArray(fill)) {
    if (fill.length !== 1) return null;
    paint = fill[0];
    shape = "array";
  }
  if (paint == null || typeof paint !== "object" || Array.isArray(paint)) return null;
  if (typeof paint.type !== "string" || paint.type.toLowerCase() !== "solid") return null;
  if (paint.visible === false || paint.enabled === false) return null;
  if (typeof paint.opacity === "number" && paint.opacity !== 1) return null;
  var parsed = __parseOpaqueHex(paint.color);
  return parsed == null ? null : { color: parsed.color, rgb: parsed.rgb, shape: shape };
}
function __relativeLuminance(rgb) {
  function channel(value) {
    var s = value / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  }
  return 0.2126 * channel(rgb[0]) + 0.7152 * channel(rgb[1]) + 0.0722 * channel(rgb[2]);
}
function __contrastRatio(a, b) {
  var left = __parseOpaqueHex(a);
  var right = __parseOpaqueHex(b);
  if (left == null || right == null) return null;
  var l1 = __relativeLuminance(left.rgb);
  var l2 = __relativeLuminance(right.rgb);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}
function __replaceSolidFill(fill, color) {
  if (typeof fill === "string") return color;
  if (Array.isArray(fill)) {
    var items = fill.slice();
    items[0] = Object.assign({}, items[0], {color: color});
    return items;
  }
  return Object.assign({}, fill, {color: color});
}
function __normalizeForegroundContrast(obj, background) {
  if (background == null || obj == null || typeof obj !== "object") return obj;
  var threshold = obj.type === "text" ? 4.5 : (obj.type === "icon_font" ? 3.0 : null);
  if (threshold == null || !__hasOwn(obj, "fill")) return obj;
  var foreground = __opaqueSolidFill(obj.fill, obj.opacity);
  if (foreground == null) return obj;
  var current = __contrastRatio(foreground.color, background);
  if (current == null || current >= threshold) return obj;
  var dark = __contrastRatio("#17191D", background);
  var light = __contrastRatio("#FAF8F3", background);
  var replacement = dark >= light ? "#17191D" : "#FAF8F3";
  return Object.assign({}, obj, {fill: __replaceSolidFill(obj.fill, replacement)});
}
function __isDivider(obj) {
  if (obj == null || typeof obj !== "object" || typeof obj.name !== "string") return false;
  var name = obj.name.toLowerCase();
  var explicitName = name.indexOf("divider") !== -1
    || name.indexOf("separator") !== -1
    || name.indexOf("分隔") !== -1
    || /rule(?:$|[^a-z])/.test(name);
  return explicitName
    && obj.width === "fill_container"
    && typeof obj.height === "number"
    && obj.height >= 0
    && obj.height <= 2;
}
function __canonicalIconFontName(name) {
  switch (name) {
    case "magnifying-glass": return "search";
    case "snow": return "snowflake";
    case "drop": return "droplet";
    case "cup": return "coffee";
    case "table-lamp": return "lamp-desk";
    default: return name;
  }
}
globalThis.I = function (parent, obj) {
  var recorded = obj;
  if (obj != null && typeof obj === "object" && obj.type === "frame" && obj.layout === "center") {
    var centeredFrame = {layout: "vertical"};
    if (!__hasOwn(obj, "alignItems")) centeredFrame.alignItems = "center";
    if (!__hasOwn(obj, "justifyContent")) centeredFrame.justifyContent = "center";
    recorded = Object.assign({}, obj, centeredFrame);
  } else if (obj != null && typeof obj === "object" && obj.type === "text") {
    var defaults = {};
    if (!Object.prototype.hasOwnProperty.call(obj, "fontFamily")) defaults.fontFamily = "Inter";
    if (!Object.prototype.hasOwnProperty.call(obj, "fontSize")) defaults.fontSize = 16;
    if (!Object.prototype.hasOwnProperty.call(obj, "lineHeight")) {
      defaults.lineHeight = 1.5;
    } else if (typeof obj.lineHeight === "number" && obj.lineHeight < 1.3 && __hasCjkContent(obj.content)) {
      defaults.lineHeight = 1.5;
    }
    if (typeof obj.height === "number" && obj.textGrowth !== "fixed-width-height") {
      defaults.height = "fit_content";
    }
    if (Object.keys(defaults).length > 0) recorded = Object.assign({}, obj, defaults);
  } else if (obj != null && typeof obj === "object" && obj.type === "icon_font" && typeof obj.iconFontName === "string") {
    var canonicalIconFontName = __canonicalIconFontName(obj.iconFontName);
    if (canonicalIconFontName !== obj.iconFontName) {
      recorded = Object.assign({}, obj, {iconFontName: canonicalIconFontName});
    }
  }
  var effectiveParent = parent == null ? "null" : String(parent);
  var directParent = __insertBindings[effectiveParent];
  if (__isDivider(recorded) && directParent != null && directParent.layout === "horizontal") {
    effectiveParent = directParent.parent;
  }
  var effectiveParentMeta = __insertBindings[effectiveParent];
  var inheritedBackground = effectiveParentMeta == null ? null : effectiveParentMeta.background;
  recorded = __normalizeForegroundContrast(recorded, inheritedBackground);
  var binding = __record(effectiveParent, JSON.stringify(recorded));
  var background = inheritedBackground;
  var isForegroundNode = recorded != null
    && typeof recorded === "object"
    && (recorded.type === "text" || recorded.type === "icon_font");
  if (!isForegroundNode && recorded != null && typeof recorded === "object" && __hasOwn(recorded, "fill")) {
    var ownBackground = __opaqueSolidFill(recorded.fill, recorded.opacity);
    background = ownBackground == null ? null : ownBackground.color;
  }
  var layout = recorded != null && typeof recorded === "object" && typeof recorded.layout === "string"
    ? recorded.layout.toLowerCase()
    : null;
  __insertBindings[binding] = {parent: effectiveParent, background: background, layout: layout};
  return binding;
};
globalThis.K = function (kitComponentId, parent, overrides) {
  return __recordK(JSON.stringify(String(kitComponentId)), parent == null ? "null" : String(parent), JSON.stringify(overrides == null ? {} : overrides));
};
globalThis.U = function (nodeId, patch) {
  __recordU(JSON.stringify(String(nodeId)), JSON.stringify(patch));
  return nodeId;
};
function __unsupported(op) {
  return function () {
    throw new Error("OP_SCRIPT_MODE_UNSUPPORTED: " + op + "() is unavailable in direct QuickJS; use only I(), K(), and authorized U() calls");
  };
}
globalThis.C = __unsupported("C");
globalThis.D = __unsupported("D");
globalThis.M = __unsupported("M");
globalThis.R = __unsupported("R");
globalThis.G = __unsupported("G");
var __noop = function () {};
globalThis.console = { log: __noop, warn: __noop, error: __noop, info: __noop, debug: __noop };
"##;
