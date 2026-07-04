//! TEMPORARY debug probe — fill-override targeting investigation.

use crate::figma_types::parse_fig_file;
use crate::kiwi::FigValue;
use crate::tree::{build_tree, TreeNode};

fn walk_find<'a>(n: &'a TreeNode, pred: &dyn Fn(&TreeNode) -> bool, out: &mut Vec<&'a TreeNode>) {
    if pred(n) {
        out.push(n);
    }
    for c in &n.children {
        walk_find(c, pred, out);
    }
}

fn lid(n: &TreeNode) -> u64 {
    n.figma
        .get("guid")
        .and_then(|g| g.get_f64("localID"))
        .map(|l| l as u64)
        .unwrap_or(0)
}

fn fill_desc(f: &FigValue) -> String {
    f.get_array("fillPaints")
        .map(|a| {
            a.iter()
                .map(|p| {
                    let t = p.get_str("type").unwrap_or("?");
                    if t == "IMAGE" {
                        "IMAGE".to_string()
                    } else if let Some(c) = p.get("color") {
                        format!(
                            "{}({:.2},{:.2},{:.2})@{:.2}",
                            t,
                            c.get_f64("r").unwrap_or(-1.0),
                            c.get_f64("g").unwrap_or(-1.0),
                            c.get_f64("b").unwrap_or(-1.0),
                            p.get_f64("opacity").unwrap_or(1.0),
                        )
                    } else {
                        t.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

fn dump(n: &TreeNode, depth: usize) {
    let f = &n.figma;
    println!(
        "{}[lid {}] {} {:?} sz={:?}x{:?} fill=[{}]",
        "  ".repeat(depth),
        lid(n),
        f.get_str("type").unwrap_or("?"),
        f.get_str("name").unwrap_or(""),
        f.get("size").and_then(|s| s.get_f64("x")).map(|v| v as i64),
        f.get("size").and_then(|s| s.get_f64("y")).map(|v| v as i64),
        fill_desc(f),
    );
    for c in &n.children {
        dump(c, depth + 1);
    }
}

#[test]
#[ignore]
fn probe_fill_targets() {
    let path = std::env::var("OP_FIG_PATH").expect("set OP_FIG_PATH");
    let bytes = std::fs::read(&path).expect("read fig");
    let decoded = parse_fig_file(&bytes).expect("parse");
    let tree = build_tree(&decoded.node_changes).expect("tree");

    // 1) summary card symbol subtree with localIDs + fills.
    let mut syms = Vec::new();
    walk_find(
        &tree,
        &|n| {
            n.figma.get_str("type") == Some("SYMBOL")
                && n.figma.get_str("name") == Some("Type=Double")
        },
        &mut syms,
    );
    if let Some(sym) = syms.first() {
        println!("=== SYMBOL Type=Double (localID order) ===");
        dump(sym, 0);
    }

    // 2) orderCard symbol Rect3 fill + one instance's 8519 override.
    let mut order_syms = Vec::new();
    walk_find(
        &tree,
        &|n| {
            n.figma.get_str("type") == Some("SYMBOL")
                && n.figma.get_str("name") == Some("orderCard")
        },
        &mut order_syms,
    );
    if let Some(sym) = order_syms.first() {
        println!("\n=== SYMBOL orderCard ===");
        dump(sym, 0);
    }
    // Find row instances (Frame 23-30) and dump their 8519 overrides.
    let mut rows = Vec::new();
    walk_find(
        &tree,
        &|n| {
            n.figma.get_str("type") == Some("INSTANCE")
                && n.figma
                    .get("symbolData")
                    .and_then(|s| s.get("symbolID"))
                    .and_then(|g| g.get_f64("localID"))
                    .map(|l| l as u64 == 4977)
                    .unwrap_or(false)
        },
        &mut rows,
    );
    println!(
        "\n=== orderCard instances: {} — 8519/8527/8529 overrides ===",
        rows.len()
    );
    for r in rows.iter().take(9) {
        let name = r.figma.get_str("name").unwrap_or("?");
        if let Some(entries) = r
            .figma
            .get("symbolData")
            .and_then(|s| s.get_array("symbolOverrides"))
        {
            for e in entries {
                let l = e
                    .get("guidPath")
                    .and_then(|p| p.get_array("guids"))
                    .and_then(|g| g.first())
                    .and_then(|g| g.get_f64("localID"))
                    .map(|v| v as u64)
                    .unwrap_or(0);
                if [8519u64, 8527, 8529].contains(&l) {
                    println!("  {name}: OV {l} fill=[{}]", fill_desc(e));
                }
            }
        }
    }
}
