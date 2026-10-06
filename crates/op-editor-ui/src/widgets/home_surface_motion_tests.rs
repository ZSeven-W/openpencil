//! Entrance-choreography unit tests for the Studio Home surface: the
//! paint-side phase functions (prototype values — welcome/tabs/panels/
//! explore/recent stagger 0/60/120/180/240 ms over 600 ms, rise 10 px;
//! art switch and hover lift over 300 ms) and the state-level deadline
//! contract.

use super::paint::{art_phase, enter_phase, hover_lift_dy, hover_lift_phase};
use super::HomeEnterBlock;
use op_editor_core::editor_ui_state::home::HOME_HOVER_LIFT_MS;
use op_editor_core::{HomeState, HOME_ART_SWITCH_MS, HOME_ENTER_WINDOW_MS};

const SHOWN: u64 = 10_000;

/// start / mid / end for one block: below the start the block sits at
/// `(rise, 0)`, inside the window it is partial, past the end settled.
fn assert_block_curve(block: HomeEnterBlock, start: u64) {
    let (duration, rise) = (600.0_f32, 10.0_f32);
    assert_eq!(
        enter_phase(block, SHOWN, SHOWN + start),
        (10.0, 0.0),
        "{block:?} not at rest before its window"
    );
    let (mid_dy, mid_alpha) = enter_phase(block, SHOWN, SHOWN + start + 300);
    assert!(
        mid_dy > 0.0 && mid_dy < rise,
        "{block:?} mid rise {mid_dy} not inside 0..{rise}"
    );
    assert!(
        (0.0..1.0).contains(&mid_alpha),
        "{block:?} mid alpha {mid_alpha} not inside 0..1"
    );
    assert_eq!(
        enter_phase(block, SHOWN, SHOWN + start + duration as u64),
        (0.0, 1.0),
        "{block:?} must settle exactly at its window end"
    );
}

#[test]
fn the_blocks_stagger_60ms_apart_over_600ms() {
    assert_block_curve(HomeEnterBlock::Welcome, 0);
    assert_block_curve(HomeEnterBlock::Tabs, 60);
    assert_block_curve(HomeEnterBlock::Panels, 120);
    assert_block_curve(HomeEnterBlock::Explore, 180);
    assert_block_curve(HomeEnterBlock::Recent, 240);
    // The panels start exactly when the tabs are 60 ms in; when the
    // tabs settle (660 ms), the panels are still mid-flight.
    let tabs_settled = SHOWN + 60 + 600;
    assert_eq!(
        enter_phase(HomeEnterBlock::Tabs, SHOWN, tabs_settled),
        (0.0, 1.0)
    );
    let (panels_dy, panels_alpha) = enter_phase(HomeEnterBlock::Panels, SHOWN, tabs_settled);
    assert!(panels_dy > 0.0 && panels_alpha < 1.0);
    // The last block (recent) settles at 240 + 600 = 840 ms.
    assert_eq!(
        enter_phase(HomeEnterBlock::Recent, SHOWN, SHOWN + 240 + 600),
        (0.0, 1.0)
    );
}

#[test]
fn an_unstamped_surface_paints_settled() {
    // shown_at_ms == 0 means "not started" (no host paint has stamped
    // the clock yet) — every block paints at t = 1, never frozen.
    for block in [
        HomeEnterBlock::Welcome,
        HomeEnterBlock::Tabs,
        HomeEnterBlock::Panels,
        HomeEnterBlock::Explore,
        HomeEnterBlock::Recent,
    ] {
        assert_eq!(
            enter_phase(block, 0, 1_234),
            (0.0, 1.0),
            "{block:?} must be static without a stamp"
        );
    }
    assert_eq!(art_phase(0, 1_234), 1.0);
    assert_eq!(hover_lift_phase(0, 1_234), 1.0);
}

#[test]
fn the_deadline_frames_while_the_entrance_runs_and_stops_after() {
    let home = HomeState {
        visible: true,
        shown_at_ms: SHOWN,
        ..HomeState::default()
    };
    assert_eq!(home.entrance_deadline_ms(SHOWN), Some(SHOWN + 16));
    assert_eq!(
        home.entrance_deadline_ms(SHOWN + HOME_ENTER_WINDOW_MS - 1),
        Some(SHOWN + HOME_ENTER_WINDOW_MS - 1 + 16)
    );
    assert_eq!(
        home.entrance_deadline_ms(SHOWN + HOME_ENTER_WINDOW_MS),
        None,
        "past the whole window nothing animates"
    );
    // The recent block's settle (240 ms stagger + 600 ms rise) is the
    // window the scheduler must cover.
    assert_eq!(HOME_ENTER_WINDOW_MS, 840, "240 stagger + 600 rise");
}

#[test]
fn the_art_switch_rises_over_300ms() {
    assert_eq!(art_phase(2_000, 2_000), 0.0);
    let mid = art_phase(2_000, 2_000 + HOME_ART_SWITCH_MS / 2);
    assert!((0.0..1.0).contains(&mid));
    assert_eq!(
        art_phase(2_000, 2_000 + HOME_ART_SWITCH_MS),
        1.0,
        "settled after the switch window"
    );
}

#[test]
fn the_hover_lift_plays_in_and_out_over_300ms() {
    assert_eq!(HOME_HOVER_LIFT_MS, 300);
    // Hover starts: the card rises from rest toward −4 px.
    assert_eq!(hover_lift_dy(true, 2_000, 2_000), 0.0);
    let mid_rise = hover_lift_dy(true, 2_000, 2_000 + HOME_HOVER_LIFT_MS / 2);
    assert!((-4.0..0.0).contains(&mid_rise), "mid rise {mid_rise}");
    assert_eq!(
        hover_lift_dy(true, 2_000, 2_000 + HOME_HOVER_LIFT_MS),
        -4.0,
        "settled hover sits at the full −4 px lift"
    );
    // Hover ends: the same stamp descends from −4 px back to rest.
    assert_eq!(hover_lift_dy(false, 5_000, 5_000), -4.0);
    let mid_fall = hover_lift_dy(false, 5_000, 5_000 + HOME_HOVER_LIFT_MS / 2);
    assert!((-4.0..0.0).contains(&mid_fall), "mid fall {mid_fall}");
    assert_eq!(
        hover_lift_dy(false, 5_000, 5_000 + HOME_HOVER_LIFT_MS),
        0.0,
        "the card rests after the lift-out window"
    );
    // An unstamped host degrades to the settled extremes, never a
    // frozen mid-lift.
    assert_eq!(hover_lift_dy(true, 0, 9_999), -4.0);
    assert_eq!(hover_lift_dy(false, 0, 9_999), 0.0);
}
