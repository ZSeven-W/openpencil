//! Figma vector geometry decoder — ports `figma-vector-decoder.ts`.
//! Decodes the two Figma path blob formats — the opcode command
//! stream (`fillGeometry` / `strokeGeometry`) and the vertex/segment
//! vector-network table — into SVG path `d` strings.

use crate::figma_types::BlobOrString;
use crate::kiwi::FigValue;
use std::collections::HashMap;

/// Approximate path bounding box (control points included).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// Format a coordinate: snap near-zero to `0`, else 4-decimal round
/// with trailing zeros stripped.
fn r(n: f64) -> String {
    if n.abs() < 5e-5 {
        return "0".to_string();
    }
    let rounded: f64 = format!("{n:.4}").parse().unwrap_or(n);
    format!("{rounded}")
}

fn f32_le(blob: &[u8], off: usize) -> Option<f64> {
    let s = blob.get(off..off + 4)?;
    Some(f32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f64)
}

fn u32_le(blob: &[u8], off: usize) -> Option<u32> {
    let s = blob.get(off..off + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn join_parts(parts: &[String]) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// Decode the opcode command stream — `0x00`=Z, `0x01`=M, `0x02`=L,
/// `0x03`=Q (quadratic), `0x04`=C (cubic); operands are f32-LE. A
/// truncated operand buffer or unknown opcode returns the prefix
/// decoded so far.
pub fn decode_figma_path_blob(blob: &[u8]) -> Option<String> {
    if blob.len() < 9 {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut off = 0usize;
    while off < blob.len() {
        let cmd = blob[off];
        off += 1;
        match cmd {
            0x00 => parts.push("Z".to_string()),
            0x01 | 0x02 => {
                let (Some(x), Some(y)) = (f32_le(blob, off), f32_le(blob, off + 4)) else {
                    return join_parts(&parts);
                };
                off += 8;
                if x.is_finite() && y.is_finite() {
                    let letter = if cmd == 0x01 { "M" } else { "L" };
                    parts.push(format!("{letter}{} {}", r(x), r(y)));
                }
            }
            0x03 => {
                let coords: Option<Vec<f64>> = (0..4).map(|i| f32_le(blob, off + i * 4)).collect();
                let Some(c) = coords else {
                    return join_parts(&parts);
                };
                off += 16;
                if c.iter().all(|v| v.is_finite()) {
                    parts.push(format!("Q{} {} {} {}", r(c[0]), r(c[1]), r(c[2]), r(c[3])));
                }
            }
            0x04 => {
                let coords: Option<Vec<f64>> = (0..6).map(|i| f32_le(blob, off + i * 4)).collect();
                let Some(c) = coords else {
                    return join_parts(&parts);
                };
                off += 24;
                if c.iter().all(|v| v.is_finite()) {
                    parts.push(format!(
                        "C{} {} {} {} {} {}",
                        r(c[0]),
                        r(c[1]),
                        r(c[2]),
                        r(c[3]),
                        r(c[4]),
                        r(c[5])
                    ));
                }
            }
            _ => return join_parts(&parts),
        }
    }
    join_parts(&parts)
}

/// Scan signed decimal numbers (`-?\d+\.?\d*`) out of a path-command
/// body. Lone `-` and exponents are not matched — `r()` never emits
/// either.
fn scan_numbers(body: &str) -> Vec<f64> {
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'-' || c.is_ascii_digit() {
            let start = i;
            if c == b'-' {
                i += 1;
            }
            let digit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i > digit_start && i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i > digit_start {
                if let Ok(v) = body[start..i].parse::<f64>() {
                    out.push(v);
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Approximate the bounding box of an SVG path `d` string using its
/// raw coordinate pairs (control points included; no extrema math).
pub fn compute_svg_path_bounds(d: &str) -> Option<PathBounds> {
    let is_cmd = |c: char| matches!(c, 'M' | 'L' | 'C' | 'Q' | 'Z' | 'm' | 'l' | 'c' | 'q' | 'z');
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    let mut letter: Option<char> = None;
    let mut body = String::new();
    let flush = |letter: Option<char>,
                 body: &str,
                 mnx: &mut f64,
                 mny: &mut f64,
                 mxx: &mut f64,
                 mxy: &mut f64| {
        let Some(l) = letter else { return };
        if l.eq_ignore_ascii_case(&'Z') {
            return;
        }
        let nums = scan_numbers(body);
        let mut i = 0;
        while i + 1 < nums.len() {
            let (x, y) = (nums[i], nums[i + 1]);
            if x.is_finite() && y.is_finite() {
                *mnx = mnx.min(x);
                *mny = mny.min(y);
                *mxx = mxx.max(x);
                *mxy = mxy.max(y);
            }
            i += 2;
        }
    };

    for c in d.chars() {
        if is_cmd(c) {
            flush(
                letter, &body, &mut min_x, &mut min_y, &mut max_x, &mut max_y,
            );
            letter = Some(c);
            body.clear();
        } else {
            body.push(c);
        }
    }
    flush(
        letter, &body, &mut min_x, &mut min_y, &mut max_x, &mut max_y,
    );

    if min_x.is_finite() {
        Some(PathBounds {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    } else {
        None
    }
}

/// Whether any paint in the array is visible (`visible != false`).
fn any_visible(paints: Option<&[FigValue]>) -> bool {
    paints
        .map(|p| p.iter().any(|x| x.get_bool("visible") != Some(false)))
        .unwrap_or(false)
}

/// Decode a Figma vector node into an SVG path string. Prefers
/// geometry blobs (stroke centerline for stroke-only shapes), falling
/// back to the vector-network table.
pub fn decode_figma_vector_path(node: &FigValue, blobs: &[BlobOrString]) -> Option<String> {
    let has_fills = any_visible(node.get_array("fillPaints"));
    let has_strokes = any_visible(node.get_array("strokePaints"));

    let geometries = if !has_fills && has_strokes {
        node.get_array("strokeGeometry")
            .or_else(|| node.get_array("fillGeometry"))
    } else {
        node.get_array("fillGeometry")
            .or_else(|| node.get_array("strokeGeometry"))
    };

    let Some(geometries) = geometries.filter(|g| !g.is_empty()) else {
        return decode_vector_network_blob(node, blobs);
    };

    let mut path_parts: Vec<String> = Vec::new();
    for geom in geometries {
        let Some(idx) = geom.get_f64("commandsBlob") else {
            continue;
        };
        if let Some(BlobOrString::Bytes(bytes)) = blobs.get(idx as usize) {
            if let Some(decoded) = decode_figma_path_blob(bytes) {
                path_parts.push(decoded);
            }
        }
    }

    if path_parts.is_empty() {
        return decode_vector_network_blob(node, blobs);
    }
    // Geometry coords are already node-local — no scaling.
    Some(path_parts.join(" "))
}

struct VnSegment {
    start: usize,
    end: usize,
    ts: (f64, f64),
    te: (f64, f64),
}

/// Decode the vertex/segment vector-network blob — the fallback when
/// no geometry blob is present. Coordinates are scaled by
/// `nodeSize / normalizedSize`; tangents are start/end-relative.
pub fn decode_vector_network_blob(node: &FigValue, blobs: &[BlobOrString]) -> Option<String> {
    let vector_data = node.get("vectorData")?;
    let blob_idx = vector_data.get_f64("vectorNetworkBlob")? as usize;
    let BlobOrString::Bytes(blob) = blobs.get(blob_idx)? else {
        return None;
    };
    if blob.len() < 8 {
        return None;
    }

    let mut off = 0usize;
    let vertex_count = u32_le(blob, off)? as usize;
    off += 4;
    if vertex_count > 100_000 || off + vertex_count * 8 > blob.len() {
        return None;
    }
    let mut vertices: Vec<(f64, f64)> = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let x = f32_le(blob, off)?;
        let y = f32_le(blob, off + 4)?;
        off += 8;
        vertices.push((x, y));
    }

    let segment_count = u32_le(blob, off)? as usize;
    off += 4;
    if segment_count > 100_000 {
        return None;
    }
    let mut segments: Vec<VnSegment> = Vec::new();
    for _ in 0..segment_count {
        if off + 24 > blob.len() {
            break;
        }
        let start = u32_le(blob, off)? as usize;
        let end = u32_le(blob, off + 4)? as usize;
        let ts = (f32_le(blob, off + 8)?, f32_le(blob, off + 12)?);
        let te = (f32_le(blob, off + 16)?, f32_le(blob, off + 20)?);
        off += 24;
        if start < vertex_count && end < vertex_count {
            segments.push(VnSegment { start, end, ts, te });
        }
    }
    if segments.is_empty() || vertices.is_empty() {
        return None;
    }

    let norm = vector_data.get("normalizedSize");
    let norm_w = norm.and_then(|n| n.get_f64("x")).unwrap_or(1.0);
    let norm_h = norm.and_then(|n| n.get_f64("y")).unwrap_or(1.0);
    let size = node.get("size");
    let node_w = size.and_then(|s| s.get_f64("x")).unwrap_or(norm_w);
    let node_h = size.and_then(|s| s.get_f64("y")).unwrap_or(norm_h);
    let sx = if norm_w > 0.001 { node_w / norm_w } else { 1.0 };
    let sy = if norm_h > 0.001 { node_h / norm_h } else { 1.0 };

    // Adjacency: segment indices keyed by their start vertex.
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, seg) in segments.iter().enumerate() {
        adj.entry(seg.start).or_default().push(i);
    }

    let mut parts: Vec<String> = Vec::new();
    let mut used = vec![false; segments.len()];
    for i in 0..segments.len() {
        if used[i] {
            continue;
        }
        let seg = &segments[i];
        let sv = vertices[seg.start];
        parts.push(format!("M{} {}", r(sv.0 * sx), r(sv.1 * sy)));
        used[i] = true;
        emit_segment(seg, &vertices, sx, sy, &mut parts);
        let chain_start = seg.start;
        let mut current = seg.end;
        loop {
            let mut found = false;
            if let Some(nexts) = adj.get(&current) {
                for &ni in nexts {
                    if used[ni] {
                        continue;
                    }
                    used[ni] = true;
                    emit_segment(&segments[ni], &vertices, sx, sy, &mut parts);
                    current = segments[ni].end;
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
        if current == chain_start {
            parts.push("Z".to_string());
        }
    }

    let result = parts.join(" ");
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn emit_segment(
    seg: &VnSegment,
    vertices: &[(f64, f64)],
    sx: f64,
    sy: f64,
    parts: &mut Vec<String>,
) {
    let sv = vertices[seg.start];
    let ev = vertices[seg.end];
    let straight = seg.ts.0.abs() < 1e-4
        && seg.ts.1.abs() < 1e-4
        && seg.te.0.abs() < 1e-4
        && seg.te.1.abs() < 1e-4;
    if straight {
        parts.push(format!("L{} {}", r(ev.0 * sx), r(ev.1 * sy)));
    } else {
        let cp1x = (sv.0 + seg.ts.0) * sx;
        let cp1y = (sv.1 + seg.ts.1) * sy;
        let cp2x = (ev.0 + seg.te.0) * sx;
        let cp2y = (ev.1 + seg.te.1) * sy;
        parts.push(format!(
            "C{} {} {} {} {} {}",
            r(cp1x),
            r(cp1y),
            r(cp2x),
            r(cp2y),
            r(ev.0 * sx),
            r(ev.1 * sy)
        ));
    }
}

#[cfg(test)]
mod tests;
