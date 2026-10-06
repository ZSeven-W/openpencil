//! Test mounts for cleanup's split implementation modules.

use super::*;

#[path = "cleanup_image_slots_tests.rs"]
mod cleanup_image_slots_tests;
#[path = "cleanup_tests.rs"]
mod tests;
#[path = "cleanup_abandoned_duplicate_roots_tests.rs"]
mod tests_abandoned_duplicate_roots;
#[path = "cleanup_absolute_container_tests.rs"]
mod tests_absolute_container;
#[path = "cleanup_bottom_nav_tests.rs"]
mod tests_bottom_nav;
#[path = "cleanup_card_height_equalize_tests.rs"]
mod tests_card_height_equalize;
#[path = "cleanup_clip_row_stroke_tests.rs"]
mod tests_clip_row_stroke;
#[path = "cleanup_deck_geometry_tests.rs"]
mod tests_deck_geometry;
#[path = "cleanup_desktop_dashboard_tests.rs"]
mod tests_desktop_dashboard;
#[path = "cleanup_fill_container_content_tests.rs"]
mod tests_fill_container_content;
#[path = "cleanup_mobile_bottom_nav_dedup_tests.rs"]
mod tests_mobile_bottom_nav_dedup;
#[path = "cleanup_mobile_chrome_tests.rs"]
mod tests_mobile_chrome;
#[path = "cleanup_mobile_dense_tests.rs"]
mod tests_mobile_dense;
#[path = "cleanup_nested_horizontal_padding_tests.rs"]
mod tests_nested_horizontal_padding;
#[path = "cleanup_rail_wrapper_gutter_tests.rs"]
mod tests_rail_wrapper_gutter;
#[path = "cleanup_repair_summary_tests.rs"]
mod tests_repair_summary;
#[path = "cleanup_repair_tier_tests.rs"]
mod tests_repair_tier;
