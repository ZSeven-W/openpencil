//! Keyboard focus for the desktop Studio Home: the Tab order, where each
//! target sits, and the visible focus ring.
//!
//! The order is the reading order a keyboard user needs — the composer
//! first (it is the page's primary action), then the task chips, the
//! example, the explore cards, recent projects, and the top bar — and it
//! is VALIDATED against the real hit-test: a target is only in the order
//! when a press at its centre would land on it. That keeps keyboard
//! activation (a press at the centre, see the host) and the pointer path
//! the same code, and it makes modal layers trap focus for free: while the
//! connect card is open every other target hit-tests as `ConnectClose`,
//! so only the card's rows remain.

use super::layout::HomeLayout;
use super::{HomeSurface, StudioPalette, EXPLORE_FAMILIES};
use crate::widgets::studio_focus_ring::paint_focus_ring;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit};

/// Corner radius of Home's focus ring (matches its buttons and chips).
const RING_RADIUS: f32 = 10.0;

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

impl HomeSurface<'_> {
    /// Where `hit` sits in `layout`, when it is a focusable target.
    pub fn focus_rect(&self, layout: &HomeLayout, hit: HomeHit) -> Option<Rect> {
        let rect = match hit {
            HomeHit::Sheet => layout.input_box,
            HomeHit::Segment(index) => *layout.segment_options.get(index as usize)?,
            HomeHit::Attachment => layout.screenshot,
            HomeHit::ModelChip => layout.model_chip,
            HomeHit::Send => layout.send,
            HomeHit::Tab(family) => {
                let index = HomeFamily::ALL.iter().position(|each| *each == family)?;
                layout.tabs[index]
            }
            HomeHit::More => layout.more_button,
            HomeHit::MoreItem(family) => {
                let hidden = HomeLayout::hidden_tasks(layout.tabs_row.size.x, self.state.task);
                let index = hidden.iter().position(|each| *each == family)?;
                *layout.more_rows.get(index)?
            }
            HomeHit::UseExample | HomeHit::BackToWorkspace => layout.use_example,
            HomeHit::ReplaceConfirm => layout.replace_use,
            HomeHit::ReplaceKeep => layout.replace_keep,
            HomeHit::ExploreCard(family) => {
                let index = EXPLORE_FAMILIES.iter().position(|each| *each == family)?;
                *layout.explore_cards.get(index)?
            }
            HomeHit::Recent(index) => *layout.recent_chips.get(index)?,
            HomeHit::NewCanvas => layout.new_canvas,
            HomeHit::OpenFile => layout.open_file,
            HomeHit::Account => layout.account,
            HomeHit::Professional => layout.professional,
            HomeHit::ConnectFreeTier => layout.connect_rows[0],
            HomeHit::ConnectApiKey => layout.connect_rows[1],
            HomeHit::ConnectCli => layout.connect_rows[2],
            _ => return None,
        };
        (rect.size.x > 0.0 && rect.size.y > 0.0).then_some(rect)
    }

    /// Every candidate target, in Tab order, before validation.
    fn focus_candidates(&self, layout: &HomeLayout) -> Vec<HomeHit> {
        if self.state.connect_card_open {
            return vec![
                HomeHit::ConnectFreeTier,
                HomeHit::ConnectApiKey,
                HomeHit::ConnectCli,
            ];
        }
        let mut order = Vec::with_capacity(32);
        if self.state.replace_pending {
            order.extend([HomeHit::ReplaceConfirm, HomeHit::ReplaceKeep]);
        }
        order.push(HomeHit::Sheet);
        order.extend((0..3).map(HomeHit::Segment));
        order.extend([HomeHit::Attachment, HomeHit::ModelChip, HomeHit::Send]);
        order.extend(HomeFamily::ALL.iter().copied().map(HomeHit::Tab));
        order.push(HomeHit::More);
        if self.state.more_open {
            let hidden = HomeLayout::hidden_tasks(layout.tabs_row.size.x, self.state.task);
            order.extend(hidden.into_iter().map(HomeHit::MoreItem));
        }
        order.push(if self.ui.workspace.active {
            HomeHit::BackToWorkspace
        } else {
            HomeHit::UseExample
        });
        order.extend(EXPLORE_FAMILIES.iter().copied().map(HomeHit::ExploreCard));
        order.extend((0..self.recent_files.len()).map(HomeHit::Recent));
        order.extend([
            HomeHit::NewCanvas,
            HomeHit::OpenFile,
            HomeHit::Account,
            HomeHit::Professional,
        ]);
        order
    }

    /// The Tab order: the targets a press at their centre would reach, on
    /// the page scrolled to the top (a target scrolled under the pinned
    /// top bar is still part of the page).
    pub fn focus_order(&self, viewport_width: f32, viewport_height: f32) -> Vec<HomeHit> {
        let layout = self.layout_at_scroll(viewport_width, viewport_height, 0.0);
        self.focus_candidates(&layout)
            .into_iter()
            .filter(|hit| {
                self.focus_rect(&layout, *hit).is_some_and(|rect| {
                    self.hit_test_layout(viewport_width, viewport_height, &layout, centre(rect))
                        == Some(*hit)
                })
            })
            .collect()
    }

    /// The scroll that brings `hit` fully into view below the pinned top
    /// bar, or `None` when it already is (or does not scroll at all).
    pub fn focus_scroll_for(
        &self,
        viewport_width: f32,
        viewport_height: f32,
        hit: HomeHit,
        top_inset: f32,
    ) -> Option<f32> {
        if matches!(
            hit,
            HomeHit::OpenFile | HomeHit::Account | HomeHit::Professional
        ) {
            return None;
        }
        let page = self.layout_at_scroll(viewport_width, viewport_height, 0.0);
        let rect = self.focus_rect(&page, hit)?;
        const MARGIN: f32 = 16.0;
        let scroll = self.state.scroll_y;
        let top = rect.origin.y - scroll;
        let bottom = top + rect.size.y;
        if top < top_inset + MARGIN {
            Some((rect.origin.y - top_inset - MARGIN).max(0.0))
        } else if bottom > viewport_height - MARGIN {
            Some(rect.origin.y + rect.size.y + MARGIN - viewport_height)
        } else {
            None
        }
    }
}

/// Paint the keyboard focus ring around Home's focused target, if any.
pub(super) fn paint_home_focus_ring(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    palette: StudioPalette,
) {
    let Some(hit) = surface.state.key_focus else {
        return;
    };
    if let Some(rect) = surface.focus_rect(layout, hit) {
        paint_focus_ring(cx.backend, rect, RING_RADIUS, palette.blue);
    }
}
