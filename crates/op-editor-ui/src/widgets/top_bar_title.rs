//! Bounded file-title layout for the `TopBar`.
//!
//! The title lives in the center slot between the Home button and the
//! agent chip. The visible file-name / edited-marker / Git-button group is
//! centered against the full window whenever the slot has room; left/right
//! chrome only clamps it in genuinely narrow layouts. Long file names are
//! middle-elided while the edited marker and Git button keep their own space.

use crate::widgets::top_bar::*;
use crate::{Point2D, Rect};

pub(super) const TITLE_SIDE_GAP: f32 = 8.0;
const TITLE_TEXT_MAX_WIDTH: f32 = 360.0;
const EDITED_GAP: f32 = 6.0;
const GIT_GAP: f32 = 10.0;

#[derive(Debug, Clone)]
pub(super) struct TopBarTitleLayout {
    pub file_name: String,
    pub file_x: f32,
    pub edited_x: Option<f32>,
    pub git_rect: Option<Rect>,
    pub slot: Rect,
}

impl TopBar {
    /// Compute the title layout using one caller-supplied text metric.
    ///
    /// Paint supplies exact backend metrics; native hit-test and popup
    /// anchoring pass the same family-aware metric, while backend-less callers
    /// use a conservative fallback. The edited marker and Git button are
    /// reserved before the file name is elided, so neither can be pushed under
    /// the agent chip by a long path basename.
    pub(super) fn title_layout(
        &self,
        top_bar_rect: Rect,
        mut measure: impl FnMut(&str, f32) -> f32,
    ) -> TopBarTitleLayout {
        let chip_text = self.chip_text();
        let chip_text_w = measure(&chip_text, 11.0);
        let chip_rect = self.agent_chip_rect(top_bar_rect, chip_text_w);
        let home_rect = self.home_button_rect(top_bar_rect);
        let slot_left = home_rect.origin.x + home_rect.size.x + TITLE_SIDE_GAP;
        let slot_right = chip_rect.origin.x - TITLE_SIDE_GAP;
        let slot_width = (slot_right - slot_left).max(0.0);
        let slot = Rect {
            origin: Point2D::new(slot_left, top_bar_rect.origin.y),
            size: Point2D::new(slot_width, top_bar_rect.size.y),
        };

        let edited_w = if self.edited {
            measure(self.label_edited, 11.0)
        } else {
            0.0
        };
        let edited_span = if self.edited {
            EDITED_GAP + edited_w
        } else {
            0.0
        };

        let measured_git_w = if GIT_BUTTON_AVAILABLE {
            let branch_w = self
                .git_branch
                .as_deref()
                .map(|branch| GIT_GAP - 4.0 + measure(branch, 11.0))
                .unwrap_or(0.0);
            GIT_BUTTON_PAD_X * 2.0 + ICON_SIZE + branch_w
        } else {
            0.0
        };
        // In an exceptionally narrow window the edited marker and a minimal
        // recognizable file label take precedence over the optional Git
        // affordance.
        let minimum_file = minimum_elided_filename(&self.file_name);
        let minimum_file_w = measure(&minimum_file, 13.0);
        let show_git = measured_git_w > 0.0
            && slot_width >= edited_span + minimum_file_w + GIT_GAP + measured_git_w;
        let git_span = if show_git {
            GIT_GAP + measured_git_w
        } else {
            0.0
        };
        let max_title_w = (slot_width - git_span).clamp(0.0, TITLE_TEXT_MAX_WIDTH);
        let max_file_w = (max_title_w - edited_span).max(0.0);
        let file_name = elide_filename_to_width(&self.file_name, max_file_w, |candidate| {
            measure(candidate, 13.0)
        });
        let file_w = measure(&file_name, 13.0);
        let actual_edited_gap = if self.edited && !file_name.is_empty() {
            EDITED_GAP
        } else {
            0.0
        };
        let title_w = file_w + actual_edited_gap + edited_w;
        let group_w = title_w + git_span;
        let desired_group_left = top_bar_rect.origin.x + (top_bar_rect.size.x - group_w) / 2.0;
        let max_group_left = (slot_right - group_w).max(slot_left);
        let group_left = desired_group_left.clamp(slot_left, max_group_left);
        let file_x = group_left;
        let edited_x = self.edited.then_some(file_x + file_w + actual_edited_gap);
        let git_rect = show_git.then_some(Rect {
            origin: Point2D::new(
                group_left + title_w + GIT_GAP,
                top_bar_rect.origin.y + (top_bar_rect.size.y - ICON_BUTTON) / 2.0,
            ),
            size: Point2D::new(measured_git_w, ICON_BUTTON),
        });

        TopBarTitleLayout {
            file_name,
            file_x,
            edited_x,
            git_rect,
            slot,
        }
    }
}

fn minimum_elided_filename(file_name: &str) -> String {
    let chars: Vec<char> = file_name.chars().collect();
    chars
        .iter()
        .rposition(|&c| c == '.')
        .filter(|&index| index > 0 && index + 1 < chars.len())
        .map(|extension_start| {
            let suffix: String = chars[extension_start..].iter().collect();
            format!("…{suffix}")
        })
        .unwrap_or_else(|| "…".to_string())
}

/// Middle-elide a basename while retaining its final extension when possible.
pub(super) fn elide_filename_to_width(
    file_name: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> String {
    if file_name.is_empty() || max_width <= 0.0 {
        return String::new();
    }
    if measure(file_name) <= max_width {
        return file_name.to_string();
    }

    const ELLIPSIS: char = '…';
    let chars: Vec<char> = file_name.chars().collect();
    let extension_start = chars
        .iter()
        .rposition(|&c| c == '.')
        .filter(|&index| index > 0 && index + 1 < chars.len());

    if let Some(extension_start) = extension_start {
        let suffix: String = chars[extension_start..].iter().collect();
        let minimum = format!("{ELLIPSIS}{suffix}");
        if measure(&minimum) <= max_width {
            return widest_candidate(0, extension_start, max_width, &mut measure, |count| {
                let prefix: String = chars[..count].iter().collect();
                format!("{prefix}{ELLIPSIS}{suffix}")
            });
        }
    }

    let ellipsis = ELLIPSIS.to_string();
    if measure(&ellipsis) > max_width {
        return String::new();
    }
    widest_candidate(0, chars.len(), max_width, &mut measure, |count| {
        let prefix: String = chars[..count].iter().collect();
        format!("{prefix}{ELLIPSIS}")
    })
}

fn widest_candidate(
    mut low: usize,
    mut high: usize,
    max_width: f32,
    measure: &mut impl FnMut(&str) -> f32,
    candidate: impl Fn(usize) -> String,
) -> String {
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        let text = candidate(mid);
        if measure(&text) <= max_width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    candidate(low)
}
