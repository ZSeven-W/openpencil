//! Public entry points: `PenDocument` -> `DocPayload` conversion
//! (laid out, geometry-preserving and multi-document preview) plus the
//! variable-table build.

use super::*;

pub struct LoadedDoc {
    pub payload: DocPayload,
}

/// Convert a parsed `PenDocument` into the desktop's `DocPayload`,
/// running each page-root through jian-core's `LayoutEngine` so
/// flex sizes resolve to absolute scene-coord rects before paint.
pub fn pen_document_to_payload(doc: &PenDocument) -> LoadedDoc {
    pen_document_to_payload_with_measure(doc, layout_measure_backend())
}

/// Convert a document using an explicit text backend, shared with its runtime.
pub fn pen_document_to_payload_with_measure(
    doc: &PenDocument,
    measure: Rc<dyn MeasureBackend>,
) -> LoadedDoc {
    let pages: Vec<PagePayload> = if let Some(pages) = &doc.pages {
        pages
            .iter()
            .map(|p| build_page_with_measure(&p.id, &p.name, &p.children, Rc::clone(&measure)))
            .collect()
    } else if !doc.children.is_empty() {
        // Single-page fallback (TS shape: top-level `children`).
        vec![build_page_with_measure(
            "page-1",
            doc.name.as_deref().unwrap_or("Page 1"),
            &doc.children,
            Rc::clone(&measure),
        )]
    } else {
        vec![PagePayload {
            id: "n1".to_string(),
            name: "Page 1".into(),
            children: Vec::new(),
        }]
    };
    LoadedDoc {
        payload: DocPayload {
            version: 1,
            active_page_index: 0,
            pages,
            // Canonical-schema variables are harvested separately
            // by `build_var_table` and assigned after apply_payload;
            // this private-payload field stays empty for that path.
            var_table: crate::variables::VarTablePayload::default(),
        },
    }
}

/// Convert a document that already carries authored absolute/parent
/// geometry into payloads without running the flex/text layout pass.
///
/// Figma `.fig` import uses this after parsing in Preserve mode: all
/// nodes have numeric sizes and parent-local positions from Figma, so
/// re-running jian layout only burns time and can visibly freeze the
/// UI after the import worker finishes.
pub fn pen_document_to_payload_preserving_geometry(doc: &PenDocument) -> LoadedDoc {
    let pages: Vec<PagePayload> = if let Some(pages) = &doc.pages {
        pages
            .iter()
            .map(|p| build_page_preserving_geometry(&p.id, &p.name, &p.children))
            .collect()
    } else if !doc.children.is_empty() {
        vec![build_page_preserving_geometry(
            "page-1",
            doc.name.as_deref().unwrap_or("Page 1"),
            &doc.children,
        )]
    } else {
        vec![PagePayload {
            id: "n1".to_string(),
            name: "Page 1".into(),
            children: Vec::new(),
        }]
    };
    LoadedDoc {
        payload: DocPayload {
            version: 1,
            active_page_index: 0,
            pages,
            var_table: crate::variables::VarTablePayload::default(),
        },
    }
}

/// Convert a document pair into payloads for the Canvas Preview: the
/// PAINT tree comes from `paint_doc` (the promoted document, so widget
/// leaves carry their `SceneWidget` props) while GEOMETRY comes from
/// `layout_doc` (the unpromoted document, laid out exactly as the
/// design canvas lays it out — or, for preserve-geometry documents,
/// its authored rects). Promotion keeps each frame's id, so the
/// rect-by-id lookup lands for promoted widgets; the children a
/// promotion dropped simply don't appear in the paint tree.
///
/// This is what makes Preview pixel-positions match the design canvas
/// BY CONSTRUCTION: the design canvas resolves geometry from the same
/// unpromoted tree through the same layout (or preserve) pass.
///
/// Both documents must be structurally parallel (the promoted document
/// is loaded from the serialized unpromoted one), so their pages line
/// up index-for-index.
pub fn pen_documents_to_payload_for_preview(
    paint_doc: &PenDocument,
    layout_doc: &PenDocument,
    preserve_authored_geometry: bool,
    measure: Rc<dyn MeasureBackend>,
) -> LoadedDoc {
    let rects_for = |roots: &[PenNode]| -> BTreeMap<String, [f32; 4]> {
        if preserve_authored_geometry {
            crate::authored_geometry::rects_for_roots(roots)
        } else {
            let mut rects = BTreeMap::new();
            for root in roots {
                compute_layout_with_measure(root, &mut rects, Rc::clone(&measure));
            }
            rects
        }
    };
    let build = |id: &str, name: &str, paint_roots: &[PenNode], layout_roots: &[PenNode]| {
        let rects = rects_for(layout_roots);
        let mut children: Vec<NodePayload> = paint_roots
            .iter()
            .map(|n| node_to_payload(n, &rects))
            .collect();
        mark_root_frame_clips(paint_roots, &mut children);
        PagePayload {
            id: id.to_string(),
            name: name.to_string(),
            children,
        }
    };
    let pages: Vec<PagePayload> = match (&paint_doc.pages, &layout_doc.pages) {
        (Some(paint_pages), Some(layout_pages)) => paint_pages
            .iter()
            .zip(layout_pages.iter())
            .map(|(pp, lp)| build(&pp.id, &pp.name, &pp.children, &lp.children))
            .collect(),
        _ if !paint_doc.children.is_empty() => vec![build(
            "page-1",
            paint_doc.name.as_deref().unwrap_or("Page 1"),
            &paint_doc.children,
            &layout_doc.children,
        )],
        _ => vec![PagePayload {
            id: "n1".to_string(),
            name: "Page 1".into(),
            children: Vec::new(),
        }],
    };
    LoadedDoc {
        payload: DocPayload {
            version: 1,
            active_page_index: 0,
            pages,
            var_table: crate::variables::VarTablePayload::default(),
        },
    }
}

/// Copy `PenDocument.variables` + `.themes` into a shell-core
/// `VariableTable`. Caller assigns the result to `Document.var_table`
/// AFTER `apply_payload` (which clears it via Default). Lossless on
/// the supported `VariableDefinition` variants; unknown future
/// `VariableKind`s round-trip via their `Color/Number/Boolean/String`
/// label since the enums are isomorphic.
pub fn build_var_table(doc: &PenDocument) -> op_editor_core::scene_vars::VariableTable {
    use op_editor_core::scene_vars::{
        ThemeAxis, ThemedValue, Variable, VariableKind, VariableTable, VariableValue,
    };
    let mut out = VariableTable::default();
    if let Some(themes) = &doc.themes {
        for (axis_name, values) in themes {
            out.themes.push(ThemeAxis {
                name: axis_name.clone(),
                values: values.clone(),
            });
        }
    }
    if let Some(vars) = &doc.variables {
        for (name, def) in vars {
            let kind = match def.kind {
                jian_ops_schema::variable::VariableKind::Color => VariableKind::Color,
                jian_ops_schema::variable::VariableKind::Number => VariableKind::Number,
                jian_ops_schema::variable::VariableKind::Boolean => VariableKind::Boolean,
                jian_ops_schema::variable::VariableKind::String => VariableKind::String,
            };
            let value = match &def.value {
                jian_ops_schema::variable::VariableValue::Scalar(s) => {
                    VariableValue::Scalar(map_scalar(s))
                }
                jian_ops_schema::variable::VariableValue::Themed(arr) => VariableValue::Themed(
                    arr.iter()
                        .map(|tv| ThemedValue {
                            value: map_scalar(&tv.value),
                            theme: tv.theme.clone(),
                        })
                        .collect(),
                ),
            };
            out.variables.push(Variable {
                name: name.clone(),
                kind,
                value,
            });
        }
    }
    out
}

fn map_scalar(
    s: &jian_ops_schema::variable::VariableScalar,
) -> op_editor_core::scene_vars::VariableScalar {
    use op_editor_core::scene_vars::VariableScalar;
    match s {
        jian_ops_schema::variable::VariableScalar::Bool(b) => VariableScalar::Bool(*b),
        jian_ops_schema::variable::VariableScalar::Num(n) => VariableScalar::Num(*n),
        jian_ops_schema::variable::VariableScalar::Str(s) => VariableScalar::Str(s.clone()),
    }
}
