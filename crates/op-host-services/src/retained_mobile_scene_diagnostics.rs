//! Test-only visibility into route projection and framed-root disagreements.
//! Records the existing scene without changing its active page or assertions.

use super::*;

fn observed_texts(root: &SceneNode, current: &SceneNode, values: &mut Vec<Value>) {
    if let Some(text) = &current.text {
        values.push(json!({
            "id":current.id,"text":text,"bounds":rect_value(current.bounds),
            "fully_visible_through_root_clip":visible_rect(root, &current.id, root.bounds)
                .is_some_and(|clip| inside(current.bounds,clip)),
        }));
    }
    for child in &current.children {
        observed_texts(root, child, values);
    }
}

pub(super) fn snapshot(session: &PreviewSession, scene: &LayoutScene) -> Value {
    let pages: Vec<_> = scene
        .pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            let roots: Vec<_> = page
                .children
                .iter()
                .map(|root| {
                    let clip = visible_rect(root, &root.id, root.bounds);
                    let mut texts = Vec::new();
                    observed_texts(root, root, &mut texts);
                    json!({
                        "id":root.id,"bounds":rect_value(root.bounds),
                        "bounds_valid":valid(root.bounds),"hidden":root.hidden,
                        "opacity":root.opacity,"composite_opacity":root.composite_opacity,
                        "clip_content":root.clip_content,
                        "fully_visible_through_own_clip":clip.is_some_and(|c| inside(root.bounds,c)),
                        "child_ids":root.children.iter().map(|child| &child.id).collect::<Vec<_>>(),
                        "observed_texts":texts,
                    })
                })
                .collect();
            json!({"index":index,"id":page.id,"name":page.name,
                "active":index == scene.active_page_index,"roots":roots})
        })
        .collect();
    json!({
        "scope":"diagnostic scene inventory; presence alone does not prove presented screen",
        "active_page_index":scene.active_page_index,
        "pages":pages,
        "current_screen_scene_rect":session.current_screen_scene_rect().map(rect_value),
        "screen_switcher_entries":session.screen_switcher_entries(),
        "framed_root":session.framed_root().map(|(id,bounds)| json!({"id":id,"bounds":rect_value(bounds)})),
    })
}

pub(super) fn capture_other_active_roots(
    scene: &LayoutScene,
    framed_root: &str,
    output: &Path,
    name: &str,
    receipt: &mut Value,
) {
    let Some(page) = scene.active_page() else {
        return;
    };
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("capture");
    for (index, root) in page.children.iter().enumerate() {
        if root.id == framed_root || !valid(root.bounds) {
            continue;
        }
        // An alternate-root export is evidence about scene contents only.
        // Keep the real framed-root capture and failing assertion unchanged.
        let result = crate::export::render_node_raster_bytes(
            scene,
            &root.id,
            crate::export::RasterFormat::Png,
            1.0,
        );
        let mut capture = json!({
            "root":root.id,"bounds":rect_value(root.bounds),
            "kind":"diagnostic-other-active-root-export-not-presentation-or-navigation-proof",
        });
        match result {
            Ok(png) => {
                let path = output.join(format!("{stem}-diagnostic-root-{index}.png"));
                match std::fs::write(&path, &png) {
                    Ok(()) => {
                        capture["file"] = json!(path);
                        capture["sha256"] = json!(sha(&png));
                    }
                    Err(error) => capture["error"] = json!(error.to_string()),
                }
            }
            Err(error) => capture["error"] = json!(error.to_string()),
        }
        if !receipt["diagnostic_root_captures"].is_array() {
            receipt["diagnostic_root_captures"] = json!([]);
        }
        receipt["diagnostic_root_captures"]
            .as_array_mut()
            .unwrap()
            .push(capture);
    }
}
