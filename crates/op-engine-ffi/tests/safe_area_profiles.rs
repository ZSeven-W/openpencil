#![cfg(feature = "editor")]

//! Pixel-level safe-area contracts for the mobile editor surface.
//!
//! The platform shells report logical insets, while the Rust engine owns both
//! the full drawable backdrop and the safe-area-local editor chrome. These
//! profiles deliberately exercise phone/tablet size classes and asymmetric
//! Android cutouts without depending on an iOS or Android simulator.

use op_editor_ui::Theme;
use op_engine_ffi::{
    op_create, op_destroy, op_frame_cpu, op_get_pixel_size, op_set_keyboard, op_set_safe_area,
    OpCreateDesc, OpEngine, OpStatus,
};
use std::ptr;

const SAMPLE_DOC: &str =
    include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");
#[derive(Clone, Copy)]
struct Insets {
    top: usize,
    right: usize,
    bottom: usize,
    left: usize,
}

#[derive(Clone, Copy)]
struct Profile {
    name: &'static str,
    width: usize,
    height: usize,
    insets: Insets,
    compact: bool,
}

const PROFILES: [Profile; 4] = [
    Profile {
        name: "iPhone portrait notch + home indicator",
        width: 393,
        height: 852,
        insets: Insets {
            top: 59,
            right: 0,
            bottom: 34,
            left: 0,
        },
        compact: true,
    },
    Profile {
        name: "iPad portrait",
        width: 834,
        height: 1_194,
        insets: Insets {
            top: 24,
            right: 0,
            bottom: 20,
            left: 0,
        },
        compact: false,
    },
    Profile {
        name: "iPad landscape",
        width: 1_194,
        height: 834,
        insets: Insets {
            top: 24,
            right: 0,
            bottom: 20,
            left: 0,
        },
        compact: false,
    },
    Profile {
        name: "Android landscape cutout + gesture navigation",
        width: 915,
        height: 412,
        insets: Insets {
            top: 24,
            right: 8,
            bottom: 24,
            left: 44,
        },
        compact: true,
    },
];

struct Harness {
    engine: *mut OpEngine,
    width: usize,
    height: usize,
}

impl Harness {
    fn new(profile: Profile) -> Self {
        let doc = SAMPLE_DOC.as_bytes();
        let desc = OpCreateDesc {
            size: std::mem::size_of::<OpCreateDesc>(),
            doc_ptr: doc.as_ptr(),
            doc_len: doc.len(),
            width: profile.width as f32,
            height: profile.height as f32,
            dpr: 1.0,
            callbacks: ptr::null(),
            asset_base_ptr: ptr::null(),
            asset_base_len: 0,
            mode: 1,
        };
        let mut engine = ptr::null_mut();
        assert_eq!(unsafe { op_create(&desc, &mut engine) }, OpStatus::Ok);
        assert!(!engine.is_null());
        assert_eq!(
            unsafe {
                op_set_safe_area(
                    engine,
                    profile.insets.top as f32,
                    profile.insets.right as f32,
                    profile.insets.bottom as f32,
                    profile.insets.left as f32,
                )
            },
            OpStatus::Ok,
            "{} must accept its safe-area profile",
            profile.name
        );
        Self {
            engine,
            width: profile.width,
            height: profile.height,
        }
    }

    fn frame(&self) -> Vec<u8> {
        let mut width = 0_u32;
        let mut height = 0_u32;
        assert_eq!(
            unsafe { op_get_pixel_size(self.engine, &mut width, &mut height) },
            OpStatus::Ok
        );
        assert_eq!((width as usize, height as usize), (self.width, self.height));
        let stride = self.width * 4;
        let mut pixels = vec![0_u8; self.height * stride];
        assert_eq!(
            unsafe {
                op_frame_cpu(
                    self.engine,
                    1_000,
                    pixels.as_mut_ptr(),
                    pixels.len(),
                    stride,
                )
            },
            OpStatus::Ok
        );
        pixels
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        assert_eq!(unsafe { op_destroy(self.engine) }, OpStatus::Ok);
    }
}

#[test]
fn mobile_safe_area_profiles_paint_continuous_unobscured_chrome() {
    for profile in PROFILES {
        let harness = Harness::new(profile);
        // Warm paint/layout caches before making pixel assertions.
        let _ = harness.frame();
        let frame = harness.frame();
        assert_surface_is_opaque(profile, &frame);
        assert_safe_bands_use_theme_background(profile, &frame);
        assert_top_chrome_starts_at_safe_boundary(profile, &frame);
        assert_bottom_chrome_respects_safe_boundary(profile, &frame);
    }
}

#[test]
fn keyboard_does_not_double_subtract_safe_area_or_move_unfocused_chrome() {
    for profile in PROFILES {
        let harness = Harness::new(profile);
        let _ = harness.frame();
        let before = harness.frame();
        let keyboard_height = (profile.insets.bottom + 300) as f32;
        assert_eq!(
            unsafe { op_set_keyboard(harness.engine, keyboard_height) },
            OpStatus::Ok,
            "{} must accept keyboard occlusion",
            profile.name
        );
        let after = harness.frame();
        assert_eq!(
            after, before,
            "{} keyboard occlusion must stay local: the safe-area backdrop, app bar, canvas, and dock cannot move",
            profile.name
        );
    }
}

fn assert_surface_is_opaque(profile: Profile, frame: &[u8]) {
    assert!(
        frame.chunks_exact(4).all(|pixel| pixel[3] == 0xff),
        "{} must paint every drawable pixel, including cutout and gesture bands",
        profile.name
    );
}

fn assert_safe_bands_use_theme_background(profile: Profile, frame: &[u8]) {
    let theme_background = dark_theme_background();
    let Insets {
        top,
        right,
        bottom,
        left,
    } = profile.insets;
    let content_mid_x = left + (profile.width - left - right) / 2;
    let content_mid_y = top + (profile.height - top - bottom) / 2;

    if top > 0 {
        assert_eq!(
            pixel_at(frame, profile.width, content_mid_x, top / 2),
            theme_background,
            "{} top safe band should extend the app surface, not introduce a black strip",
            profile.name
        );
    }
    if bottom > 0 {
        assert_eq!(
            pixel_at(
                frame,
                profile.width,
                content_mid_x,
                profile.height - (bottom / 2).max(1),
            ),
            theme_background,
            "{} bottom safe band should extend the app surface behind system gestures",
            profile.name
        );
    }
    if left > 0 {
        assert_eq!(
            pixel_at(frame, profile.width, left / 2, content_mid_y),
            theme_background,
            "{} left cutout band should use the root app surface",
            profile.name
        );
    }
    if right > 0 {
        assert_eq!(
            pixel_at(
                frame,
                profile.width,
                profile.width - (right / 2).max(1),
                content_mid_y,
            ),
            theme_background,
            "{} right gesture/cutout band should use the root app surface",
            profile.name
        );
    }
}

fn assert_top_chrome_starts_at_safe_boundary(profile: Profile, frame: &[u8]) {
    let theme_background = dark_theme_background();
    let usable_width = profile.width - profile.insets.left - profile.insets.right;
    let x = profile.insets.left + usable_width / 2;
    let safe_pixel = pixel_at(
        frame,
        profile.width,
        x,
        profile.insets.top.saturating_sub(2),
    );
    let app_bar_pixel = pixel_at(frame, profile.width, x, profile.insets.top + 2);

    assert_ne!(
        app_bar_pixel, theme_background,
        "{} app bar should begin immediately inside the safe boundary without an excessive blank margin",
        profile.name
    );
    assert_natural_transition(
        profile,
        "top safe band to app bar",
        safe_pixel,
        app_bar_pixel,
    );
}

fn assert_bottom_chrome_respects_safe_boundary(profile: Profile, frame: &[u8]) {
    let theme_background = dark_theme_background();
    let boundary_y = profile.height - profile.insets.bottom;
    let usable_width = profile.width - profile.insets.left - profile.insets.right;
    let x = if profile.compact {
        profile.insets.left + 2
    } else {
        // Expanded tablets keep a persistent layer rail at the left edge.
        // Sample the center below the floating dock so this assertion tests
        // the intended root backdrop rather than the rail's card surface.
        profile.insets.left + usable_width / 2
    };
    let content_pixel = pixel_at(frame, profile.width, x, boundary_y - 2);
    let safe_pixel = pixel_at(
        frame,
        profile.width,
        x,
        (boundary_y + 2).min(profile.height - 1),
    );

    if profile.compact {
        assert_ne!(
            content_pixel, theme_background,
            "{} compact dock should reach the safe boundary instead of being obscured or over-inset",
            profile.name
        );
        assert_natural_transition(
            profile,
            "compact dock to bottom safe band",
            content_pixel,
            safe_pixel,
        );
    } else {
        assert_eq!(
            content_pixel, theme_background,
            "{} tablet dock floats above a continuous root background",
            profile.name
        );
        assert_eq!(
            safe_pixel, content_pixel,
            "{} tablet bottom safe band must continue the root background without a seam",
            profile.name
        );
    }
}

fn assert_natural_transition(profile: Profile, edge: &str, first: [u8; 4], second: [u8; 4]) {
    let max_channel_delta = first[..3]
        .iter()
        .zip(&second[..3])
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    assert!(
        max_channel_delta <= 12,
        "{} {edge} should be a restrained surface transition, got {first:?} -> {second:?}",
        profile.name
    );
}

fn pixel_at(buffer: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let offset = (y * width + x) * 4;
    [
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ]
}

fn dark_theme_background() -> [u8; 4] {
    let color = Theme::dark().background;
    [
        (color.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
