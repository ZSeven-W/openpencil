use super::*;
use serde_json::json;

/// Verbatim month-grid elements from ds41-official-1020 app-11/rep-1
/// smoke.log; the log line truncates after "，日期" and the paren is closed
/// here. The gate misread the 34×36 cell size as 1224 promised items and
/// retried a complete 6×7 calendar grid twice before failing the subtask.
const MONTH_GRID_ELEMENTS: &str = "ARCHETYPE: timeline — protagonist 月视图网格，首屏约 32%，安静排在头部之下，display 20px / body 13px，无装饰性大图。星期表头一行：一 二 三 四 五 六 日（12px #94A3B8，等宽 7 列居中）；6 行 × 7 列日期网格（单元格约 34×36，日期）";

fn task(label: &str, elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id": "month-grid", "label": label, "elements": elements,
        "region": {"width": 375, "height": 400}
    }))
    .unwrap()
}

#[test]
fn month_grid_text_promises_six_rows_by_seven_columns() {
    assert_eq!(
        grid_dimensions(&format!("月视图网格\n{MONTH_GRID_ELEMENTS}")),
        Some((6, 7))
    );
    assert_eq!(
        expected_item_count(&task("月视图网格", MONTH_GRID_ELEMENTS)),
        Some(42)
    );
}

#[test]
fn cell_pixel_sizes_are_not_grid_shapes() {
    assert_eq!(grid_dimensions("34×36px cell"), None);
    assert_eq!(grid_dimensions("34×36px cell grid"), None);
    assert_eq!(grid_dimensions("日期网格（单元格约 34×36）"), None);
    assert_eq!(grid_dimensions("日期网格（格子 44×44）"), None);
    assert_eq!(grid_dimensions("grid tiles 44×44 each"), None);
}

#[test]
fn bare_pairs_and_the_nine_cell_landmark_still_shape_grids() {
    assert_eq!(grid_dimensions("3×3 grid"), Some((3, 3)));
    assert_eq!(grid_dimensions("2×3 product grid"), Some((2, 3)));
    assert_eq!(grid_dimensions("首页 分类九宫格"), Some((3, 3)));
}

#[test]
fn explicit_rows_and_columns_wording_wins_over_bare_pairs() {
    assert_eq!(
        grid_dimensions("a 6 rows × 7 columns grid, cells 44×44"),
        Some((6, 7))
    );
    assert_eq!(grid_dimensions("7 columns × 6 rows grid"), Some((6, 7)));
    assert_eq!(
        grid_dimensions("a 6 rows × 7 columns grid at 44×44"),
        Some((6, 7))
    );
    assert_eq!(grid_dimensions("日期网格 6行7列"), Some((6, 7)));
    assert_eq!(grid_dimensions("7 列 × 6 行 网格"), Some((6, 7)));
}
