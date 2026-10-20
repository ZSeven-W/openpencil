//! End-to-end tests over the produced package.
//!
//! Every assertion here reads the actual zip rather than the emitter's
//! intermediate strings: a `.pptx` that is right in memory and wrong in
//! the container (a part nobody declared, a relationship pointing at a
//! file that is not there) opens as "PowerPoint found a problem with
//! this content", and that failure is invisible from the inside.

use std::io::Read as _;

use super::*;
use op_editor_core::scene_template_catalog::TemplateScene;

/// A 1×1 opaque PNG — the smallest thing the media path can embed and
/// the size reader can measure.
const ONE_PX_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn deck_state(source: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(source)
        .expect("fixture JSON parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    state
}

fn two_board_deck() -> EditorState {
    deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"封面","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#ff0000"}]},
            {"type":"frame","id":"f2","name":"步骤 1","x":2000,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#00ff00"}]}
        ]}"##,
    )
}

fn image_deck() -> EditorState {
    deck_state(&format!(
        r##"{{"version":"1.0.0","children":[
            {{"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{{"type":"solid","color":"#ffffff"}}],"children":[
               {{"type":"image","id":"i1","x":100,"y":100,"width":400,"height":300,
                "src":"{ONE_PX_PNG}"}}
             ]}}
        ]}}"##
    ))
}

/// Every part name in the package, in zip order.
fn part_names(bytes: &[u8]) -> Vec<String> {
    let zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("valid zip");
    (0..zip.len())
        .map(|i| {
            zip.by_index_data(i)
                .expect("entry")
                .name()
                .expect("valid entry name")
                .into_owned()
        })
        .collect()
}

/// One part's contents as text.
fn part(bytes: &[u8], name: &str) -> String {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).expect("valid zip");
    let mut entry = zip
        .by_name(name)
        .unwrap_or_else(|_| panic!("part {name} is missing"));
    let mut out = String::new();
    entry.read_to_string(&mut out).expect("text part");
    out
}

fn export(state: &EditorState) -> (Vec<u8>, DeckPptxExport) {
    build_deck_pptx(state).expect("deck exports")
}

#[test]
fn the_package_carries_every_part_a_reader_will_look_for() {
    let (bytes, summary) = export(&two_board_deck());

    assert_eq!(summary.slides, 2);
    let names = part_names(&bytes);
    for required in [
        "[Content_Types].xml",
        "_rels/.rels",
        "ppt/presentation.xml",
        "ppt/_rels/presentation.xml.rels",
        "ppt/slideMasters/slideMaster1.xml",
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        "ppt/slideLayouts/slideLayout1.xml",
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        "ppt/theme/theme1.xml",
        "ppt/slides/slide1.xml",
        "ppt/slides/_rels/slide1.xml.rels",
        "ppt/slides/slide2.xml",
        "ppt/slides/_rels/slide2.xml.rels",
    ] {
        assert!(names.contains(&required.to_string()), "missing {required}");
    }
}

#[test]
fn every_relationship_target_resolves_to_a_part_that_exists() {
    let (bytes, _) = export(&image_deck());
    let names = part_names(&bytes);

    for rels_part in names.iter().filter(|n| n.ends_with(".rels")) {
        let xml = part(&bytes, rels_part);
        let base = rels_part
            .rsplit_once("_rels/")
            .map(|(dir, _)| dir.to_string())
            .unwrap_or_default();
        for target in xml.split("Target=\"").skip(1) {
            let target = target.split('"').next().expect("closing quote");
            let resolved = normalize(&format!("{base}{target}"));
            assert!(
                names.contains(&resolved),
                "{rels_part} points at {resolved}, which is not in the package"
            );
        }
    }
}

/// Collapse the `../` segments a relationship target uses.
fn normalize(path: &str) -> String {
    let mut stack: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            other => stack.push(other),
        }
    }
    stack.join("/")
}

#[test]
fn one_slide_per_visible_board_in_document_order() {
    let (bytes, _) = export(&two_board_deck());

    let names = part_names(&bytes);
    assert_eq!(
        names
            .iter()
            .filter(|n| n.starts_with("ppt/slides/slide"))
            .count(),
        2,
        "two slide parts (their rels live under ppt/slides/_rels/)"
    );
    // Board names identify which board became which slide, so a swap
    // could not pass.
    assert!(part(&bytes, "ppt/slides/slide1.xml").contains("name=\"封面\""));
    assert!(part(&bytes, "ppt/slides/slide2.xml").contains("name=\"步骤 1\""));
    let presentation = part(&bytes, "ppt/presentation.xml");
    let first = presentation.find("r:id=\"rId2\"").expect("first slide id");
    let second = presentation.find("r:id=\"rId3\"").expect("second slide id");
    assert!(first < second, "{presentation}");
}

#[test]
fn the_slide_size_is_the_board_size_in_emu() {
    let (bytes, _) = export(&two_board_deck());

    let presentation = part(&bytes, "ppt/presentation.xml");
    // 1920 x 1080 px at 9525 EMU per px — a real 16:9 slide, not a
    // rescaled one.
    assert!(
        presentation.contains("<p:sldSz cx=\"18288000\" cy=\"10287000\"/>"),
        "{presentation}"
    );
}

#[test]
fn no_emitted_number_reaches_scientific_notation() {
    let (bytes, _) = export(&two_board_deck());

    for name in part_names(&bytes) {
        if !name.ends_with(".xml") && !name.ends_with(".rels") {
            continue;
        }
        let xml = part(&bytes, &name);
        for attr in ["x=\"", "y=\"", "cx=\"", "cy=\"", "sz=\"", "w=\""] {
            for value in xml.split(attr).skip(1) {
                let value = value.split('"').next().unwrap_or("");
                assert!(
                    !value.contains('e') && !value.contains('.'),
                    "{name} has a non-integer {attr}{value}"
                );
            }
        }
    }
}

#[test]
fn slide_text_lands_as_a_real_text_box_not_as_pixels() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#ffffff"}],"children":[
               {"type":"text","id":"t1","x":100,"y":100,"width":800,"height":60,
                "content":"Quarterly Review","fontSize":32,"fontWeight":"700",
                "fontFamily":"Noto Sans SC","fill":[{"type":"solid","color":"#101828"}]}
             ]}
        ]}"##,
    );

    let (bytes, summary) = export(&state);

    assert_eq!(summary.raster_fallbacks, 0);
    let slide = part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide.contains("<a:t>Quarterly Review</a:t>"), "{slide}");
    assert!(slide.contains("sz=\"2400\""), "32 px is 24 pt: {slide}");
    assert!(slide.contains("b=\"1\""), "{slide}");
    assert!(slide.contains("<a:srgbClr val=\"101828\"/>"), "{slide}");
    assert!(
        slide.contains("<a:latin typeface=\"Noto Sans SC\"/>"),
        "{slide}"
    );
    assert!(slide.contains("txBox=\"1\""), "{slide}");
    assert!(!slide.contains("data:image"), "{slide}");
}

#[test]
fn markup_in_authored_text_and_names_is_escaped() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"<script> & \"quotes\"","x":0,"y":0,
             "width":1920,"height":1080,"fill":[{"type":"solid","color":"#ffffff"}],
             "children":[
               {"type":"text","id":"t1","x":10,"y":10,"width":900,"height":60,
                "content":"a < b & c","fontSize":24,
                "fill":[{"type":"solid","color":"#000000"}]}
             ]}
        ]}"##,
    );

    let (bytes, _) = export(&state);

    let slide = part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide.contains("<a:t>a &lt; b &amp; c</a:t>"), "{slide}");
    assert!(
        slide.contains("name=\"&lt;script&gt; &amp; &quot;quotes&quot;\""),
        "{slide}"
    );
    assert!(
        !slide.contains("<script>"),
        "raw board name leaked: {slide}"
    );
}

#[test]
fn a_hidden_board_is_skipped_rather_than_failing_the_export() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"one","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#ff0000"}]},
            {"type":"frame","id":"f2","name":"skipped","x":2000,"y":0,"width":1920,"height":1080,
             "visible":false,"fill":[{"type":"solid","color":"#00ff00"}]},
            {"type":"frame","id":"f3","name":"two","x":4000,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#0000ff"}]}
        ]}"##,
    );

    let (bytes, summary) = export(&state);

    assert_eq!(
        summary.slides, 2,
        "the hidden board must not become a slide"
    );
    let names = part_names(&bytes);
    assert!(!names.contains(&"ppt/slides/slide3.xml".to_string()));
    assert!(!part(&bytes, "ppt/slides/slide2.xml").contains("name=\"skipped\""));
    assert!(part(&bytes, "ppt/slides/slide2.xml").contains("name=\"two\""));
}

#[test]
fn a_deck_with_no_visible_board_refuses_to_write_a_file() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"gone","x":0,"y":0,"width":1920,"height":1080,
             "visible":false,"fill":[{"type":"solid","color":"#ff0000"}]}
        ]}"##,
    );
    let mut path = std::env::temp_dir();
    path.push(format!("openpencil-pptx-empty-{}.pptx", std::process::id()));

    let result = export_deck_pptx(&state, &path);

    assert_eq!(result, Err(ExportError::NothingToExport));
    assert!(!path.exists(), "a refused export must leave no file behind");
}

#[test]
fn an_embedded_image_becomes_a_media_part_the_content_types_declares() {
    let (bytes, summary) = export(&image_deck());

    assert_eq!(summary.raster_fallbacks, 0, "a data URL needs no raster");
    let names = part_names(&bytes);
    assert!(
        names.contains(&"ppt/media/image1.png".to_string()),
        "{names:?}"
    );
    let types = part(&bytes, "[Content_Types].xml");
    assert!(
        types.contains("<Default Extension=\"png\" ContentType=\"image/png\"/>"),
        "{types}"
    );
    let slide = part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide.contains("<p:pic>"), "{slide}");
    assert!(slide.contains("r:embed=\"rId2\""), "{slide}");
}

#[test]
fn every_media_extension_present_is_declared() {
    let (bytes, _) = export(&image_deck());

    let types = part(&bytes, "[Content_Types].xml");
    for name in part_names(&bytes) {
        let Some(ext) = name
            .strip_prefix("ppt/media/")
            .and_then(|file| file.rsplit('.').next())
        else {
            continue;
        };
        assert!(
            types.contains(&format!("<Default Extension=\"{ext}\"")),
            "{ext} is in the package but not in [Content_Types]: {types}"
        );
    }
}

#[test]
fn a_remote_image_is_rasterised_so_the_package_stays_self_contained() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#ffffff"}],"children":[
               {"type":"image","id":"i1","x":100,"y":100,"width":400,"height":300,
                "src":"https://example.com/hero.png"}
             ]}
        ]}"##,
    );

    let (bytes, summary) = export(&state);

    assert_eq!(summary.raster_fallbacks, 1, "the remote image must raster");
    assert!(
        part_names(&bytes).contains(&"ppt/media/image1.png".to_string()),
        "the raster must land in the package"
    );
    // Nothing anywhere in the package may still point at the network.
    for name in part_names(&bytes) {
        if !name.ends_with(".xml") && !name.ends_with(".rels") {
            continue;
        }
        let xml = part(&bytes, &name);
        assert!(!xml.contains("http://example"), "{name} leaks a URL");
        assert!(!xml.contains("https://example"), "{name} leaks a URL");
    }
}

#[test]
fn a_vector_path_rasters_while_its_siblings_stay_editable() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#ffffff"}],"children":[
               {"type":"text","id":"t1","x":10,"y":10,"width":900,"height":60,
                "content":"Still text","fontSize":24,
                "fill":[{"type":"solid","color":"#000000"}]},
               {"type":"polygon","id":"p1","x":600,"y":300,"width":200,"height":200,
                "polygonCount":5,"fill":[{"type":"solid","color":"#3366ff"}]}
             ]}
        ]}"##,
    );

    let (bytes, summary) = export(&state);

    assert_eq!(summary.raster_fallbacks, 1);
    let slide = part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide.contains("<a:t>Still text</a:t>"), "{slide}");
    assert!(slide.contains("<p:pic>"), "the polygon rastered: {slide}");
}

#[test]
fn a_gradient_board_states_its_angle_in_drawingml_units() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"linear_gradient","angle":90.0,
                      "stops":[{"offset":0.0,"color":"#000000"},
                               {"offset":1.0,"color":"#ffffff"}]}]}
        ]}"##,
    );

    let (bytes, summary) = export(&state);

    assert_eq!(
        summary.raster_fallbacks, 0,
        "a linear gradient is expressible"
    );
    let slide = part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide.contains("<a:gradFill"), "{slide}");
    // CSS 90deg runs left→right, which is DrawingML 0.
    assert!(slide.contains("<a:lin ang=\"0\" scaled=\"0\"/>"), "{slide}");
    assert!(slide.contains("pos=\"0\""), "{slide}");
    assert!(slide.contains("pos=\"100000\""), "{slide}");
}

#[test]
fn a_rounded_card_keeps_its_radius_as_a_preset_adjustment() {
    let state = deck_state(
        r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"cover","x":0,"y":0,"width":1920,"height":1080,
             "fill":[{"type":"solid","color":"#ffffff"}],"children":[
               {"type":"rectangle","id":"r1","x":100,"y":100,"width":400,"height":200,
                "cornerRadius":20,"fill":[{"type":"solid","color":"#3366ff"}]}
             ]}
        ]}"##,
    );

    let (bytes, summary) = export(&state);

    assert_eq!(summary.raster_fallbacks, 0);
    let slide = part(&bytes, "ppt/slides/slide1.xml");
    assert!(slide.contains("prst=\"roundRect\""), "{slide}");
    // 20 / min(400, 200) = 10% of the shorter side.
    assert!(slide.contains("val 10000"), "{slide}");
}

#[test]
fn the_written_file_is_a_zip_holding_the_same_parts() {
    let state = two_board_deck();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "openpencil-pptx-write-{}-{}.pptx",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let summary = export_deck_pptx(&state, &path).expect("deck exports");

    let written = std::fs::read(&path).expect("file exists");
    assert_eq!(summary.slides, 2);
    assert_eq!(part_names(&written), part_names(&export(&state).0));
    // The zip local-file-header magic — how every reader identifies the
    // format before it looks at a single part.
    assert_eq!(&written[..2], b"PK");
    let _ = std::fs::remove_file(&path);
}
