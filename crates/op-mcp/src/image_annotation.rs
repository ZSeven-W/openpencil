//! Project confirmed source-image targets into screenshot annotations.
use crate::{McpTool, ToolErrorCode, ToolOutcome};
use jian_ops_schema::node::PenNode;
use jian_scene::layout_scene::SceneImageFit;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::json;
use std::collections::BTreeMap;

pub struct AnnotateImage {
    state: EditorState,
}
pub fn annotate_image_snapshot(state: &EditorState) -> AnnotateImage {
    AnnotateImage {
        state: state.clone(),
    }
}

fn parent_of<'a>(nodes: &'a [PenNode], id: &str) -> Option<&'a PenNode> {
    for node in nodes {
        if let Some(children) = node.children() {
            if children.iter().any(|n| n.id_str() == id) {
                return Some(node);
            }
            if let Some(parent) = parent_of(children, id) {
                return Some(parent);
            }
        }
    }
    None
}

impl McpTool for AnnotateImage {
    fn name(&self) -> &str {
        "annotate_image"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(image_id) = args.get("imageId") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "imageId is required".into());
        };
        let Some(label) = args.get("label").filter(|s| !s.trim().is_empty()) else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "Name the confirmed screenshot control in label".into(),
            );
        };
        let mut source_box = [0.0; 4];
        for (i, key) in ["x", "y", "width", "height"].iter().enumerate() {
            let Some(value) = args.get(*key).and_then(|s| s.parse::<f64>().ok()) else {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("{key} must be a normalized source-image number"),
                );
            };
            source_box[i] = value;
        }
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&self.state);
        let Some(page) = scene.active_page() else {
            return ToolOutcome::Err(ToolErrorCode::ToolFailed, "No active page".into());
        };
        let Some(image) = page.find(image_id) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "Screenshot image not found".into(),
            );
        };
        if image.image_transform.is_some() {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "Reset the custom image transform before annotation".into(),
            );
        }
        let Some(src) = image.image_src.as_deref() else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "The screenshot pixels are unavailable".into(),
            );
        };
        let Some(size) = op_image_enrich::pixels::embedded_size(src) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "Import an embedded screenshot before annotating".into(),
            );
        };
        let fit = match image.image_fit {
            SceneImageFit::Fill | SceneImageFit::Crop => "cover",
            SceneImageFit::Fit => "contain",
            SceneImageFit::Stretch => "stretch",
            _ => "unsupported",
        };
        let b = image.bounds;
        let Some(projected) = op_image_enrich::pixels::project_source_box(
            size,
            [
                b.origin.x as f64,
                b.origin.y as f64,
                b.size.x as f64,
                b.size.y as f64,
            ],
            source_box,
            fit,
        ) else {
            return ToolOutcome::Err(ToolErrorCode::InvalidArgument,"The target is outside the visible screenshot; use Fit to show the whole image and supply a confirmed 0..1 source rectangle".into());
        };
        let Some(parent) = parent_of(self.state.active_children(), image_id) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "Place the screenshot in a frame before annotating".into(),
            );
        };
        let Some(parent_box) = page.find(parent.id_str()).map(|n| n.bounds) else {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "Screenshot frame unavailable".into(),
            );
        };
        let children = parent.children().unwrap();
        let at = children
            .iter()
            .position(|n| n.id_str() == image_id)
            .unwrap();
        let color = args.get("color").map(String::as_str).unwrap_or("#F97316");
        let patch = json!({"x":projected[0]-parent_box.origin.x as f64,"y":projected[1]-parent_box.origin.y as f64,
            "width":projected[2],"height":projected[3],"name":format!("标注 · {label}"),"role":"image-annotation",
            "constraints":{"h":"left","v":"top"},"fill":[],"stroke":{"fill":[{"type":"solid","color":color}],"thickness":4.0}});
        let (annotation_id, mut commands) = if let Some(id) = args.get("annotationId") {
            let Some(node) = children
                .iter()
                .find(|n| n.id_str() == id && matches!(n, PenNode::Ellipse(_)))
            else {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "annotationId must be an ellipse in the screenshot's frame".into(),
                );
            };
            (
                node.id_str().to_owned(),
                vec![EditorCommand::PatchNodeData {
                    node_id: NodeId::new(id),
                    patch_json: patch.to_string(),
                    page_id: None,
                }],
            )
        } else {
            let mut id = format!("{image_id}-annotation");
            let mut suffix = 1;
            while op_editor_core::walkers::find_node(
                self.state.active_children(),
                &NodeId::new(&id),
            )
            .is_some()
            {
                id = format!("{image_id}-annotation-{suffix}");
                suffix += 1;
            }
            let mut node = patch;
            node["type"] = json!("ellipse");
            node["id"] = json!(id);
            let Ok(node) = serde_json::from_value(node) else {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "Invalid annotation style".into(),
                );
            };
            (
                id,
                vec![EditorCommand::InsertAuthoredSubtreePreservingRoots {
                    nodes: vec![node],
                    parent_id: NodeId::new(parent.id_str()),
                    page_id: None,
                }],
            )
        };
        let target_index = at.saturating_sub(usize::from(
            children
                .iter()
                .position(|n| n.id_str() == annotation_id)
                .is_some_and(|i| i < at),
        ));
        commands.push(EditorCommand::MoveNode {
            node_id: NodeId::new(&annotation_id),
            target_parent: NodeId::new(parent.id_str()),
            page_id: None,
            index: Some(target_index),
        });
        let command = EditorCommand::Batch { commands };
        let mut probe = self.state.clone();
        if !probe.apply(command.clone()) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                "The annotation cannot be applied in this editor scope".into(),
            );
        }
        ToolOutcome::OkJsonWithCommand(
            json!({"annotationId":annotation_id,"imageId":image_id,"label":label,
            "sourceBox":source_box,"documentBox":projected,"sourceSize":size})
            .to_string(),
            command,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    #[test]
    fn annotation_projects_source_coordinates_and_preserves_the_existing_tree() {
        let image = image::RgbaImage::from_pixel(20, 40, image::Rgba([255, 255, 255, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let src = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.into_inner())
        );
        let mut state=EditorState::from_document(serde_json::from_value(json!({"version":"1.0.0","children":[
            {"type":"frame","id":"root","width":1000,"height":700,"layout":"none","children":[
                {"type":"image","id":"photo","x":100,"y":100,"width":600,"height":400,"objectFit":"fit","src":src},
                {"type":"text","id":"title","content":"Original title","x":10,"y":10,"fontSize":20}
            ]}
        ]})).unwrap());
        let mut args: BTreeMap<_, _> = [
            ("imageId", "photo"),
            ("label", "Go"),
            ("x", "0.25"),
            ("y", "0.5"),
            ("width", "0.5"),
            ("height", "0.1"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
        let ToolOutcome::OkJsonWithCommand(result, command) =
            annotate_image_snapshot(&state).call(&args)
        else {
            panic!("annotation failed");
        };
        let result: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["documentBox"], json!([350.0, 300.0, 100.0, 40.0]));
        assert!(state.apply(command));
        let children = state.active_children()[0].children().unwrap();
        assert_eq!(children[0].id_str(), "photo-annotation");
        assert!(children.iter().any(|n| n.id_str() == "photo"));
        assert!(children.iter().any(|n| n.id_str() == "title"));
        args.insert("annotationId".into(), "photo-annotation".into());
        let ToolOutcome::OkJsonWithCommand(_, command) =
            annotate_image_snapshot(&state).call(&args)
        else {
            panic!("update failed");
        };
        assert!(state.apply(command));
        assert_eq!(state.active_children()[0].children().unwrap().len(), 3);
        assert_eq!(
            state.active_children()[0].children().unwrap()[0].id_str(),
            "photo-annotation"
        );
        args.insert("x".into(), "0.9".into());
        assert!(matches!(
            annotate_image_snapshot(&state).call(&args),
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
        ));
    }
}
