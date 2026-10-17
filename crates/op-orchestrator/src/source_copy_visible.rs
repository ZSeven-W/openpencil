//! Preserve literal copy while recognizing an annotated product/price row.
use super::compact;
use jian_ops_schema::node::{LayoutMode, PenNode, TextContent};
use op_design_lint::node_util::{is_node_visible, opacity};
use op_editor_core::PenNodeExt;
use regex::Regex;
use std::{ops::Range, sync::LazyLock};

static PRICE_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?)\s*([0-9]+(?:\.[0-9]+)?)\s*元$").unwrap());
static PRICE_FIELD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]+(?:\.[0-9]+)?元$").unwrap());

#[derive(Default)]
pub(super) struct VisibleCopy {
    text: String,
    fields: Vec<Range<usize>>,
    price_rows: Vec<Vec<Range<usize>>>,
}

impl VisibleCopy {
    pub(super) fn from_nodes(nodes: &[PenNode]) -> Self {
        let mut copy = Self::default();
        for node in nodes {
            copy.collect(node);
        }
        copy
    }

    pub(super) fn take(&mut self, line: &str) -> bool {
        let needle = compact(line);
        let price = PRICE_LINE.captures(line);
        let offset = self.text.match_indices(&needle).find_map(|(offset, _)| {
            // A short product name inside another name is not that product.
            let starts_field = self.fields.iter().any(|field| field.start == offset);
            let boundary = starts_field
                || self.text[..offset]
                    .chars()
                    .next_back()
                    .is_none_or(|c| !c.is_alphanumeric());
            (price.is_none() || boundary).then_some(offset)
        });
        if let Some(offset) = offset {
            self.consume(offset..offset + needle.len());
            return true;
        }
        let Some(price) = price else {
            return false;
        };
        let product = compact(&price[1]);
        let amount = format!("{}元", &price[2]);
        let pair = self.price_rows.iter().find_map(|fields| {
            let name = fields.iter().find(|r| self.text[(*r).clone()] == product)?;
            let value = fields.iter().find(|r| self.text[(*r).clone()] == amount)?;
            Some((name.clone(), value.clone()))
        });
        if let Some((name, value)) = pair {
            self.consume(name);
            self.consume(value);
            return true;
        }
        false
    }

    fn consume(&mut self, range: Range<usize>) {
        // Keep byte offsets stable and consume each visible occurrence once.
        let tombstone = "\0".repeat(range.len());
        self.text.replace_range(range, &tombstone);
    }

    fn collect(&mut self, node: &PenNode) -> Vec<Range<usize>> {
        if !is_node_visible(node) || opacity(node) <= 0.0 {
            return Vec::new();
        }
        let mut fields = Vec::new();
        if let PenNode::Text(text) = node {
            let content = match &text.content {
                TextContent::Plain(text) => text.clone(),
                TextContent::Styled(runs) => runs.iter().map(|run| run.text.as_str()).collect(),
            };
            let content = compact(&content);
            if !content.is_empty() {
                let start = self.text.len();
                self.text.push_str(&content);
                let field = start..self.text.len();
                self.fields.push(field.clone());
                fields.push(field);
            }
        }
        if let Some(children) = node.children() {
            for child in children {
                fields.extend(self.collect(child));
            }
        }
        let horizontal = matches!(node, PenNode::Frame(frame)
            if frame.container.layout == Some(LayoutMode::Horizontal));
        // A whole menu or a row of multiple product cards is ambiguous. Only
        // one-price rows can pair fields across an intervening secondary note.
        if horizontal
            && fields
                .iter()
                .filter(|r| PRICE_FIELD.is_match(&self.text[(*r).clone()]))
                .count()
                == 1
        {
            self.price_rows.push(fields.clone());
        }
        fields
    }
}
