//! One-off smoke runner — read a `.fig` file from disk and drive it
//! through the same import path the desktop binary uses
//! (`parse_fig_binary` → `figma_all_pages_to_pen_document` →
//! `resolve_image_blobs`). Prints per-page node counts, the first
//! warnings, and a top-of-tree variant tally so the import behaves
//! visibly even without a GUI.
//!
//! Usage: `cargo run -p op-figma --example probe_fig -- <path.fig>`

use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;
use op_figma::{parse_fig_binary, FigLayoutMode};

fn variant_name(n: &PenNode) -> &'static str {
    match n {
        PenNode::Frame(_) => "Frame",
        PenNode::Group(_) => "Group",
        PenNode::Rectangle(_) => "Rectangle",
        PenNode::Ellipse(_) => "Ellipse",
        PenNode::Line(_) => "Line",
        PenNode::Polygon(_) => "Polygon",
        PenNode::Path(_) => "Path",
        PenNode::Text(_) => "Text",
        PenNode::TextInput(_) => "TextInput",
        PenNode::Image(_) => "Image",
        PenNode::IconFont(_) => "IconFont",
        PenNode::Ref(_) => "Ref",
    }
}

fn tally(nodes: &[PenNode]) -> std::collections::BTreeMap<&'static str, usize> {
    let mut out = std::collections::BTreeMap::new();
    for n in nodes {
        *out.entry(variant_name(n)).or_insert(0) += 1;
    }
    out
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_fig <path.fig>");
    let bytes = std::fs::read(&path).expect("read file");
    let file_name = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Figma Import");

    println!("input: {path} ({} bytes)", bytes.len());

    let import = match parse_fig_binary(&bytes, file_name, FigLayoutMode::OpenPencil) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("PARSE ERROR: {e}");
            std::process::exit(1);
        }
    };

    let pages = import.document.pages.as_deref().unwrap_or(&[]);
    println!("pages: {}", pages.len());
    for (i, page) in pages.iter().enumerate() {
        println!(
            "  [{}] name={:?} children={}",
            i,
            page.name,
            page.children.len()
        );
        let counts = tally(&page.children);
        if !counts.is_empty() {
            print!("        top-level tally:");
            for (k, v) in &counts {
                print!(" {k}={v}");
            }
            println!();
        }
    }

    if let Ok(out) = std::env::var("OP_DUMP_JSON") {
        let json = serde_json::to_string_pretty(&import.document).expect("serialize");
        std::fs::write(&out, json).expect("write json");
        eprintln!("dumped PenDocument JSON to {out}");
    }

    println!("warnings: {}", import.warnings.len());
    for w in import.warnings.iter().take(8) {
        println!("  - {w}");
    }
    if import.warnings.len() > 8 {
        println!("  … and {} more", import.warnings.len() - 8);
    }

    // Walk the first page's direct children: print the frame's own
    // (width, height) + auto-layout flags, then for each grandchild
    // print its (x, y, w, h) so overflow becomes visible.
    if let Some(page) = pages.first() {
        if let Some(root) = page.children.first() {
            print_overflow_audit(root);
            // Recursive drill: prints every descendant whose name
            // contains the needle, showing its own children. Useful for
            // tracking down a horizontal-row container by name.
            if let Ok(needle) = std::env::var("OP_PROBE_DRILL") {
                drill_all(root, &needle, 0);
            }
        }
    }

    // Deep digest of the first page — same shape as the TS probe so we
    // can diff `digest` / `deep_count` / `text_count` directly.
    if let Some(page) = pages.first() {
        let digest = digest_tree(&page.children);
        println!("first-page digest: {digest:x}");
        let total = count_deep(&page.children);
        println!("first-page deep node count: {total}");
        let texts = collect_text(&page.children);
        println!("first-page text node count: {}", texts.len());
        let mut clusters: std::collections::BTreeMap<(i64, i64), Vec<String>> =
            std::collections::BTreeMap::new();
        for t in &texts {
            clusters
                .entry((t.0.round() as i64, t.1.round() as i64))
                .or_default()
                .push(t.2.clone());
        }
        let overlaps: Vec<_> = clusters.iter().filter(|(_, v)| v.len() >= 2).collect();
        println!(
            "first-page co-located-text clusters (≥2 nodes sharing x,y): {}",
            overlaps.len()
        );
        for ((x, y), texts) in overlaps.iter().take(10) {
            print!("   - at ({x}, {y}):");
            for (i, t) in texts.iter().take(4).enumerate() {
                if i > 0 {
                    print!(" |");
                }
                print!(" {t:?}");
            }
            if texts.len() > 4 {
                print!(" …");
            }
            println!();
        }
    }
}

/// Same `(type | x | y | width-json | height-json)` per-node tuple
/// hashed exactly the way the TS probe in `scripts/probe-fig-ts.ts`
/// does it — bitwise i32 multiply-by-31, walking the tree in
/// pre-order. Lets us prove the Rust + TS pipelines produced
/// structurally identical PenDocuments.
fn digest_tree(nodes: &[PenNode]) -> u32 {
    let mut h: i32 = 0;
    fn go(arr: &[PenNode], h: &mut i32) {
        for c in arr {
            let (x, y) = (base_of(c).x.unwrap_or(0.0), base_of(c).y.unwrap_or(0.0));
            let w = width_json(c);
            let height = height_json(c);
            let sig = format!(
                "{}|{}|{}|{}|{}",
                type_str(c),
                fmt_num(x),
                fmt_num(y),
                w,
                height
            );
            for byte in sig.chars() {
                *h = h.wrapping_mul(31).wrapping_add(byte as i32);
            }
            go(children_of(c), h);
        }
    }
    go(nodes, &mut h);
    h as u32
}

/// Match JS's `n ?? 0` then default `.toString()` — for 0.5 etc. we
/// want "0.5", for 100 we want "100" (no trailing zero, no '.0').
fn fmt_num(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    if n.fract() == 0.0 && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        // Drop trailing zeros and trailing dot for clean repr.
        let s = format!("{n}");
        s
    }
}

fn type_str(n: &PenNode) -> &'static str {
    // Snake-case to mirror the JSON tags ("rectangle", "text", ...).
    match n {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}

fn base_of(n: &PenNode) -> &jian_ops_schema::node::base::PenNodeBase {
    match n {
        PenNode::Frame(x) => &x.base,
        PenNode::Group(x) => &x.base,
        PenNode::Rectangle(x) => &x.base,
        PenNode::Ellipse(x) => &x.base,
        PenNode::Line(x) => &x.base,
        PenNode::Polygon(x) => &x.base,
        PenNode::Path(x) => &x.base,
        PenNode::Text(x) => &x.base,
        PenNode::TextInput(x) => &x.base,
        PenNode::Image(x) => &x.base,
        PenNode::IconFont(x) => &x.base,
        PenNode::Ref(x) => &x.base,
    }
}

fn width_json(n: &PenNode) -> String {
    let w = match n {
        PenNode::Frame(x) => x.container.width.as_ref(),
        PenNode::Group(x) => x.container.width.as_ref(),
        PenNode::Rectangle(x) => x.container.width.as_ref(),
        PenNode::Ellipse(x) => x.width.as_ref(),
        PenNode::Polygon(x) => x.width.as_ref(),
        PenNode::Path(x) => x.width.as_ref(),
        PenNode::Text(x) => x.width.as_ref(),
        PenNode::TextInput(x) => x.width.as_ref(),
        PenNode::Image(x) => x.width.as_ref(),
        _ => None,
    };
    w.map(|s| serde_json::to_string(s).unwrap_or_default())
        .unwrap_or_default()
}

fn height_json(n: &PenNode) -> String {
    let h = match n {
        PenNode::Frame(x) => x.container.height.as_ref(),
        PenNode::Group(x) => x.container.height.as_ref(),
        PenNode::Rectangle(x) => x.container.height.as_ref(),
        PenNode::Ellipse(x) => x.height.as_ref(),
        PenNode::Polygon(x) => x.height.as_ref(),
        PenNode::Path(x) => x.height.as_ref(),
        PenNode::Text(x) => x.height.as_ref(),
        PenNode::TextInput(x) => x.height.as_ref(),
        PenNode::Image(x) => x.height.as_ref(),
        _ => None,
    };
    h.map(|s| serde_json::to_string(s).unwrap_or_default())
        .unwrap_or_default()
}

/// Dump the page-root frame's (w, h) + auto-layout flags, plus each
/// direct child's (x, y, w, h). Lets us see overflowed children
/// quickly — anything whose `(x + w)` or `(y + h)` exceeds the parent
/// box is overflowing it.
fn print_overflow_audit(root: &PenNode) {
    let (rw, rh, layout) = match root {
        PenNode::Frame(f) => (
            sizing_str(f.container.width.as_ref()),
            sizing_str(f.container.height.as_ref()),
            format!(
                "layout={:?} gap={:?} align_items={:?} clip_content={:?} padding={:?}",
                f.container.layout,
                f.container.gap,
                f.container.align_items,
                f.container.clip_content,
                f.container.padding
            ),
        ),
        other => (
            format!("(not Frame: {})", type_str(other)),
            "-".to_string(),
            "-".to_string(),
        ),
    };
    println!("root frame: width={rw} height={rh}");
    println!("            {layout}");
    let kids = children_of(root);
    println!("            direct children: {}", kids.len());
    let parent_w = number_or_zero(width_json_raw(root));
    let parent_h = number_or_zero(height_json_raw(root));
    for (i, c) in kids.iter().enumerate() {
        let nm = base_of(c).name.as_deref().unwrap_or("(unnamed)");
        let x = base_of(c).x.unwrap_or(0.0);
        let y = base_of(c).y.unwrap_or(0.0);
        let w = number_or_zero(width_json_raw(c));
        let h = number_or_zero(height_json_raw(c));
        let mut tags = String::new();
        if parent_w > 0.0 && x + w > parent_w + 0.5 {
            tags.push_str(" [OVERFLOWS-X]");
        }
        if parent_h > 0.0 && y + h > parent_h + 0.5 {
            tags.push_str(" [OVERFLOWS-Y]");
        }
        let wj = width_json_raw(c);
        let hj = height_json_raw(c);
        println!(
            "   [{i:2}] {nm:<24} {ty} x={x:>7.1} y={y:>7.1} w_json={wj:<18} h_json={hj:<18}{tags}",
            ty = type_str(c),
            nm = truncate(nm, 24)
        );
        let _ = (w, h);
    }
}

fn drill_all(node: &PenNode, needle: &str, depth: u32) {
    if depth > 20 {
        return;
    }
    if base_of(node)
        .name
        .as_deref()
        .map(|n| n.contains(needle))
        .unwrap_or(false)
    {
        println!();
        let pad = "  ".repeat(depth as usize);
        println!(
            "{pad}── {:?} ── (depth {depth})",
            base_of(node).name.as_deref().unwrap_or("?")
        );
        print_overflow_audit(node);
    }
    for c in children_of(node) {
        drill_all(c, needle, depth + 1);
    }
}

fn truncate(s: &str, n: usize) -> String {
    let mut acc = String::new();
    for (i, c) in s.chars().enumerate() {
        if i >= n {
            acc.push('…');
            break;
        }
        acc.push(c);
    }
    acc
}

fn width_json_raw(n: &PenNode) -> String {
    width_json(n)
}

fn height_json_raw(n: &PenNode) -> String {
    height_json(n)
}

fn sizing_str(s: Option<&SizingBehavior>) -> String {
    s.map(|v| serde_json::to_string(v).unwrap_or_default())
        .unwrap_or_else(|| "-".to_string())
}

fn number_or_zero(s: String) -> f64 {
    // serde_json output for SizingBehavior::Number(N) is just `N`.
    s.trim().parse::<f64>().unwrap_or(0.0)
}

fn count_deep(nodes: &[PenNode]) -> usize {
    let mut n = nodes.len();
    for c in nodes {
        n += count_deep(children_of(c));
    }
    n
}

fn children_of(n: &PenNode) -> &[PenNode] {
    use jian_ops_schema::node::PenNode::*;
    match n {
        Frame(f) => f.children.as_deref().unwrap_or(&[]),
        Group(g) => g.children.as_deref().unwrap_or(&[]),
        Rectangle(r) => r.children.as_deref().unwrap_or(&[]),
        Ref(r) => r.children.as_deref().unwrap_or(&[]),
        _ => &[],
    }
}

fn collect_text(nodes: &[PenNode]) -> Vec<(f64, f64, String)> {
    let mut out = Vec::new();
    fn go(arr: &[PenNode], out: &mut Vec<(f64, f64, String)>) {
        for c in arr {
            if let PenNode::Text(t) = c {
                let s = match &t.content {
                    jian_ops_schema::node::text::TextContent::Plain(p) => p.clone(),
                    jian_ops_schema::node::text::TextContent::Styled(segs) => {
                        segs.iter().map(|s| s.text.as_str()).collect::<String>()
                    }
                };
                out.push((t.base.x.unwrap_or(0.0), t.base.y.unwrap_or(0.0), s));
            }
            go(children_of(c), out);
        }
    }
    go(nodes, &mut out);
    out
}
