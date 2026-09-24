//! The user-visible quality report for one AI generation run: what the
//! deterministic quality layer checked, what it repaired automatically,
//! and what still needs the user's attention — per topic and per board.
//!
//! **Facts only.** Every number here is a count of real records: an edit a
//! repair pass applied (`QualityChecked` progress), a fix the pre-validator
//! or the vision review applied (count-only, their progress carries no
//! itemized detail), or a finding a detector made on the FINAL document
//! (the host's end-of-run audit). Nothing is scored, estimated or assumed:
//!
//! - a topic is *checked* only when a check family that runs its detectors
//!   reported in, or the final audit covered it;
//! - "remaining" is only claimed after the final audit ran
//!   ([`QualityReport::audited`]); before that the report says nothing
//!   about leftover work rather than implying "none";
//! - an empty report (no check ever ran) renders as no report at all.
//!
//! This module is pure data + formatting (wasm32-clean). The progress
//! vocabulary lives in `op-orchestrator`, which depends on this crate, so
//! the orchestrator converts its records into [`QualityRepairRecord`]s and
//! hosts feed them in through the `ingest_*` methods.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::walkers::find_node;
use crate::{EditorState, NodeId, PenNodeExt};

mod topics;

pub use topics::{lint_topics, topic_for_family, topic_for_lint, topic_for_pass, QualityTopic};

/// One repair edit as the orchestrator reports it — the structured twin of
/// its rendered `RepairRecord::line`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityRepairRecord {
    /// Pass (or pass group) that applied the edit.
    pub pass: String,
    /// Check family the edit was credited to (`layout`, `overflow`, …).
    pub family: String,
    /// Target node id — empty for document-level edits.
    pub node_id: String,
    /// Target node's name at edit time.
    pub node_name: Option<String>,
    /// What changed, field level (`gap 24 → 16`).
    pub detail: String,
}

/// One line of the report: a fix that was applied or an issue that remains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityItem {
    pub topic: QualityTopic,
    /// Internal origin (pass name or lint category) — kept for diagnostics,
    /// never shown as the headline.
    pub source: String,
    /// The node the item is about, when it is about one.
    pub node_id: Option<String>,
    pub node_name: Option<String>,
    /// Top-level board containing the node (filled by
    /// [`QualityReport::attribute_boards`]).
    pub board_id: Option<String>,
    /// Short factual description (`gap 24 → 16`, a detector's reason).
    pub detail: String,
}

impl QualityItem {
    /// `Name · detail`, or whichever half exists.
    pub fn label(&self) -> String {
        let name = self
            .node_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        let detail = self.detail.trim();
        match (name, detail.is_empty()) {
            (Some(name), false) => format!("{name} · {detail}"),
            (Some(name), true) => name.to_string(),
            (None, false) => detail.to_string(),
            (None, true) => self.node_id.clone().unwrap_or_default(),
        }
    }
}

/// Everything the report knows about one topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityTopicReport {
    pub topic: QualityTopic,
    /// Detectors for this topic are proven to have run.
    pub checked: bool,
    /// Itemized fixes, in application order.
    pub fixed: Vec<QualityItem>,
    /// Fixes reported as a count with no itemized detail (pre-validation,
    /// vision review).
    pub fixed_without_detail: usize,
    /// Issues the final audit still found.
    pub remaining: Vec<QualityItem>,
}

impl QualityTopicReport {
    fn new(topic: QualityTopic) -> Self {
        Self {
            topic,
            checked: false,
            fixed: Vec::new(),
            fixed_without_detail: 0,
            remaining: Vec::new(),
        }
    }

    pub fn fixed_count(&self) -> usize {
        self.fixed.len() + self.fixed_without_detail
    }

    /// Issues found under this topic: the ones fixed plus the ones left.
    pub fn found_count(&self) -> usize {
        self.fixed_count() + self.remaining.len()
    }
}

/// Per-board totals, in the page's board order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityBoardSummary {
    pub board_id: String,
    pub board_name: String,
    pub fixed: usize,
    pub remaining: usize,
}

/// The whole report for one run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityReport {
    /// Topics in display order; only topics that were checked or carry an
    /// item are present.
    pub topics: Vec<QualityTopicReport>,
    pub boards: Vec<QualityBoardSummary>,
    /// Statements about the run that are not edits (e.g. a skipped tier).
    pub notes: Vec<String>,
    /// The end-of-run audit ran on the final document, so `remaining` is a
    /// fact rather than an unknown.
    pub audited: bool,
}

impl QualityReport {
    fn topic_mut(&mut self, topic: QualityTopic) -> &mut QualityTopicReport {
        let index = match self
            .topics
            .binary_search_by(|entry| entry.topic.cmp(&topic))
        {
            Ok(index) => index,
            Err(index) => {
                self.topics.insert(index, QualityTopicReport::new(topic));
                index
            }
        };
        &mut self.topics[index]
    }

    /// Fold one `QualityChecked` progress event in: the check families that
    /// ran, every applied edit, and the run notes. Several events (a
    /// concurrent run's per-worker cleanups) accumulate.
    pub fn ingest_repairs(
        &mut self,
        checked_families: &[String],
        records: &[QualityRepairRecord],
        notes: &[String],
    ) {
        for family in checked_families {
            for topic in QualityTopic::covered_by_family(family) {
                self.topic_mut(*topic).checked = true;
            }
        }
        for record in records {
            let topic = topic_for_pass(&record.pass, &record.family);
            let node_id = Some(record.node_id.trim())
                .filter(|id| !id.is_empty())
                .map(str::to_string);
            let entry = self.topic_mut(topic);
            entry.checked = true;
            entry.fixed.push(QualityItem {
                topic,
                source: record.pass.clone(),
                node_id,
                node_name: record.node_name.clone(),
                board_id: None,
                detail: record.detail.clone(),
            });
        }
        for note in notes {
            if !self.notes.contains(note) {
                self.notes.push(note.clone());
            }
        }
    }

    /// Fold in the pre-validator's per-lint-category fix counts
    /// (`ValidationPreCheckDone`). Categories the report does not show a
    /// topic for are code-shape repairs and land under Structure — they are
    /// real edits, so they are still counted.
    pub fn ingest_lint_fixes(&mut self, by_category: &BTreeMap<String, usize>) {
        for (category, count) in by_category {
            if *count == 0 {
                continue;
            }
            let topic = topic_for_lint(category).unwrap_or(QualityTopic::Structure);
            let entry = self.topic_mut(topic);
            entry.checked = true;
            entry.fixed_without_detail += count;
        }
    }

    /// Fold in one vision-review round's applied fixes.
    pub fn ingest_visual_review(&mut self, applied: usize) {
        let entry = self.topic_mut(QualityTopic::VisualReview);
        entry.checked = true;
        entry.fixed_without_detail += applied;
    }

    /// Record the end-of-run audit: `audited_topics` are the topics its
    /// detectors covered (marked checked even when clean) and `remaining`
    /// every finding on the final document. Replaces any earlier audit.
    pub fn ingest_audit(&mut self, audited_topics: &[QualityTopic], remaining: Vec<QualityItem>) {
        for entry in &mut self.topics {
            entry.remaining.clear();
        }
        for topic in audited_topics {
            self.topic_mut(*topic).checked = true;
        }
        for item in remaining {
            let entry = self.topic_mut(item.topic);
            entry.checked = true;
            entry.remaining.push(item);
        }
        self.audited = true;
    }

    /// Resolve every item's board on the active page and rebuild the
    /// per-board totals for `run_boards` (the boards this run produced, in
    /// page order). Items on nodes that no longer exist keep `board_id =
    /// None` and count toward no board.
    pub fn attribute_boards(&mut self, state: &EditorState, run_boards: &[String]) {
        let boards: Vec<_> = state
            .active_children()
            .iter()
            .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
            .collect();
        let board_of = |id: &str| -> Option<String> {
            let id = NodeId::new(id);
            boards
                .iter()
                .find(|board| find_node(std::slice::from_ref(**board), &id).is_some())
                .map(|board| board.id_str().to_string())
        };
        for entry in &mut self.topics {
            for item in entry.fixed.iter_mut().chain(entry.remaining.iter_mut()) {
                item.board_id = item.node_id.as_deref().and_then(board_of);
            }
        }
        self.boards = run_boards
            .iter()
            .filter_map(|board_id| {
                let board = boards.iter().find(|board| board.id_str() == board_id)?;
                let name = board
                    .base()
                    .name
                    .clone()
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| board_id.clone());
                let count = |items: &dyn Fn(&QualityTopicReport) -> &Vec<QualityItem>| {
                    self.topics
                        .iter()
                        .flat_map(|entry| items(entry).iter())
                        .filter(|item| item.board_id.as_deref() == Some(board_id.as_str()))
                        .count()
                };
                Some(QualityBoardSummary {
                    board_id: board_id.clone(),
                    board_name: name,
                    fixed: count(&|entry| &entry.fixed),
                    remaining: count(&|entry| &entry.remaining),
                })
            })
            .collect();
    }

    /// Nothing was ever checked — render no report at all.
    pub fn is_empty(&self) -> bool {
        !self.topics.iter().any(|entry| entry.checked)
    }

    /// Topics whose detectors ran.
    pub fn checked_count(&self) -> usize {
        self.topics.iter().filter(|entry| entry.checked).count()
    }

    /// Every fix applied, itemized or not.
    pub fn total_fixed(&self) -> usize {
        self.topics
            .iter()
            .map(QualityTopicReport::fixed_count)
            .sum()
    }

    /// Issues the final audit still found (0 before the audit ran — check
    /// [`Self::audited`] before calling that "clean").
    pub fn total_remaining(&self) -> usize {
        self.topics.iter().map(|entry| entry.remaining.len()).sum()
    }

    /// The compact header chip: `质检：修复 N 处 · 待关注 M 处`.
    pub fn chip_text(&self, locale: crate::Locale) -> String {
        let key = if self.total_remaining() == 0 {
            "workspace.quality.chipClean"
        } else {
            "workspace.quality.chip"
        };
        op_i18n::translate(locale, key)
            .replace("{{fixed}}", &self.total_fixed().to_string())
            .replace("{{remaining}}", &self.total_remaining().to_string())
    }

    /// The panel's one-line summary under its title.
    pub fn summary_text(&self, locale: crate::Locale) -> String {
        op_i18n::translate(locale, "workspace.quality.summary")
            .replace("{{checked}}", &self.checked_count().to_string())
            .replace("{{fixed}}", &self.total_fixed().to_string())
            .replace("{{remaining}}", &self.total_remaining().to_string())
    }

    /// The chat transcript line appended when the run ends.
    pub fn transcript_line(&self, locale: crate::Locale) -> String {
        op_i18n::translate(locale, "workspace.quality.transcript")
            .replace("{{checked}}", &self.checked_count().to_string())
            .replace("{{fixed}}", &self.total_fixed().to_string())
            .replace("{{remaining}}", &self.total_remaining().to_string())
    }

    /// The remaining item at `(topic_index, item_index)` of [`Self::topics`].
    pub fn remaining_item(&self, topic_index: usize, item_index: usize) -> Option<&QualityItem> {
        self.topics.get(topic_index)?.remaining.get(item_index)
    }
}

#[cfg(test)]
#[path = "quality_report_tests.rs"]
mod tests;
