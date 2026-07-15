//! Regression for jian `layout::resolve` fixed-size handling. The fix: a
//! node whose width AND height are both `Number` gets `flex_shrink=0` (in
//! both `node_to_style` leaves and `container_to_style` frames), matching
//! TS (fixed sizes are summed; only fill_container shares the remainder).
//!  - a fixed square (image / pin / filter background) keeps its size in a
//!    tight row instead of taffy's default `flex_shrink:1` squeezing it;
//!  - a `width=fill, height=Number` tile (quick-action cards) is NOT pinned
//!    (the rule is AND, not OR), so N fill tiles still share the row.
use op_pen_loader::payload::NodePayload;

fn find<'a>(nodes: &'a [NodePayload], name: &str) -> Option<&'a NodePayload> {
    for n in nodes {
        if n.name == name {
            return Some(n);
        }
        if let Some(f) = find(&n.children, name) {
            return Some(f);
        }
    }
    None
}

#[test]
fn fixed_square_image_not_shrunk_in_tight_row() {
    // 160-wide horizontal row holding a fill image + a fixed 80x80 image.
    // The fixed one must stay 80x80 (the fill image absorbs the squeeze).
    let doc = r#"{"version":"1.0.0","children":[{"type":"frame","id":"row","width":160,"height":100,"layout":"horizontal","gap":8,"children":[{"type":"image","id":"fill","name":"Fill","width":"fill_container","height":"fill_container","src":""},{"type":"image","id":"sq","name":"Sq","width":80,"height":80,"src":""}]}]}"#;
    let loaded = op_pen_loader::load_canonical(doc).expect("loads");
    let d = op_pen_loader::pen_document_to_payload(&loaded.value);
    let sq = d
        .payload
        .pages
        .iter()
        .find_map(|p| find(&p.children, "Sq"))
        .expect("fixed image present");
    assert!(
        (sq.w - 80.0).abs() < 1.0 && (sq.h - 80.0).abs() < 1.0,
        "fixed 80x80 image must stay square in a tight row, got {}x{}",
        sq.w,
        sq.h
    );
}

#[test]
fn fill_width_tiles_with_fixed_height_still_share_the_row() {
    // Quick-action tiles are `width=fill_container, height=78` (only the
    // cross axis is fixed). They must still flex-share the row. A `width OR
    // height is Number` rule wrongly pinned them (flex_shrink=0) and the
    // fill tiles overflowed, showing only the first; the `AND` rule frees
    // them to share the row again.
    let doc = r#"{"version":"1.0.0","children":[{"type":"frame","id":"row","width":300,"height":100,"layout":"horizontal","gap":0,"children":[{"type":"frame","id":"a","name":"A","width":"fill_container","height":78},{"type":"frame","id":"b","name":"B","width":"fill_container","height":78},{"type":"frame","id":"c","name":"C","width":"fill_container","height":78}]}]}"#;
    let loaded = op_pen_loader::load_canonical(doc).expect("loads");
    let d = op_pen_loader::pen_document_to_payload(&loaded.value);
    let a = d
        .payload
        .pages
        .iter()
        .find_map(|p| find(&p.children, "A"))
        .expect("tile A present");
    assert!(
        a.w < 150.0,
        "fill+fixed-height tiles must share the 300px row (~100 each), not occupy it whole; got w={}",
        a.w
    );
}

#[test]
fn fixed_sibling_stays_inside_card_when_fill_column_has_wide_content() {
    // Banner-shaped: a horizontal card holding [fill column whose content is
    // wider than the remaining space, fixed 80x80 image]. The fill column
    // must shrink below its content's min-width (TS gives fill the remaining
    // space, content wraps) so the fixed image stays inside the card instead
    // of being pushed out (needs `min_size: 0` on the fill axis).
    let doc = r#"{"version":"1.0.0","children":[{"type":"frame","id":"card","width":200,"height":100,"layout":"horizontal","gap":0,"children":[{"type":"frame","id":"col","name":"Col","width":"fill_container","height":"fill_container","layout":"vertical","children":[{"type":"frame","id":"wide","width":150,"height":20}]},{"type":"image","id":"img","name":"Img","width":80,"height":80,"src":""}]}]}"#;
    let loaded = op_pen_loader::load_canonical(doc).expect("loads");
    let d = op_pen_loader::pen_document_to_payload(&loaded.value);
    let img = d
        .payload
        .pages
        .iter()
        .find_map(|p| find(&p.children, "Img"))
        .expect("img present");
    assert!(
        img.x + img.w <= 201.0,
        "fixed image must stay inside the 200px card (x+w<=200), got x={} w={}",
        img.x,
        img.w
    );
}
