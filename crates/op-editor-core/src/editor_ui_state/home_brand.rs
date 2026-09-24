//! Studio Home's "use as brand" state — the 加链接 tool.
//!
//! Pressing the tool reads a brand from what the brief already holds: the
//! first `http(s)://` link typed into the composer, else the newest staged
//! screenshot. The host drains the request ([`HomeBrandState::take_request`]),
//! extracts off the UI thread, and reports back ([`HomeBrandState::finish`]).
//! The resulting kit stays STAGED here, not applied: a Home send starts a
//! fresh document, so the kit is applied to that document right before
//! the run (see the host's send path), and keeps applying to later briefs
//! until the user removes it.

use crate::brand_kit::BrandKitPayload;

/// How long a hint / failure line stays up after the press.
pub const BRAND_HINT_MS: u64 = 4_000;

/// What to extract from.
#[derive(Clone, PartialEq, Eq)]
pub enum BrandSourceRequest {
    Url(String),
    Image {
        name: String,
        media_type: String,
        data: Vec<u8>,
    },
}

impl std::fmt::Debug for BrandSourceRequest {
    // Screenshots are megabytes; never dump them into a debug print.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Url(url) => f.debug_tuple("Url").field(url).finish(),
            Self::Image {
                name,
                media_type,
                data,
            } => f
                .debug_struct("Image")
                .field("name", name)
                .field("media_type", media_type)
                .field("bytes", &data.len())
                .finish(),
        }
    }
}

impl BrandSourceRequest {
    /// Short label for the chip while extracting (host / file name).
    pub fn label(&self) -> String {
        match self {
            Self::Url(url) => {
                let rest = url.split("://").nth(1).unwrap_or(url);
                let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
                host.strip_prefix("www.").unwrap_or(host).to_string()
            }
            Self::Image { name, .. } => name.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum HomeBrandStatus {
    #[default]
    Idle,
    /// A request is queued or running.
    Extracting { label: String },
    /// A kit is staged for the next send.
    Ready(Box<BrandKitPayload>),
    /// The last extraction failed (hint shown for [`BRAND_HINT_MS`]).
    Failed { detail: String },
    /// Pressed with no link in the brief and no screenshot staged.
    NeedsSource,
}

#[derive(Debug, Clone, Default)]
pub struct HomeBrandState {
    /// Host capability: this host can run an extraction (desktop). Off,
    /// the tool paints disabled with the "coming soon" hint.
    pub available: bool,
    pub status: HomeBrandStatus,
    /// Wall-clock ms the status last changed (drives hint timeout).
    pub status_at_ms: u64,
    /// Queued request, drained by the host.
    pending: Option<(u64, BrandSourceRequest)>,
    /// Request generation — a result for an older request (the user
    /// removed the chip or pressed again) is dropped.
    generation: u64,
}

impl HomeBrandState {
    /// The 加链接 press. `draft` is the composer text, `image` the newest
    /// staged screenshot `(name, media type, bytes)`. Returns whether a
    /// request was queued.
    pub fn press(&mut self, draft: &str, image: Option<(&str, &str, &[u8])>, now_ms: u64) -> bool {
        if matches!(self.status, HomeBrandStatus::Extracting { .. }) {
            return false;
        }
        let request = match (first_url(draft), image) {
            (Some(url), _) => BrandSourceRequest::Url(url),
            (None, Some((name, media_type, data))) => BrandSourceRequest::Image {
                name: name.to_string(),
                media_type: media_type.to_string(),
                data: data.to_vec(),
            },
            (None, None) => {
                self.set_status(HomeBrandStatus::NeedsSource, now_ms);
                return false;
            }
        };
        self.generation += 1;
        self.set_status(
            HomeBrandStatus::Extracting {
                label: request.label(),
            },
            now_ms,
        );
        self.pending = Some((self.generation, request));
        true
    }

    /// Host: take the queued request `(generation, source)`.
    pub fn take_request(&mut self) -> Option<(u64, BrandSourceRequest)> {
        self.pending.take()
    }

    /// Host: report a finished extraction. Stale generations are ignored.
    pub fn finish(
        &mut self,
        generation: u64,
        result: Result<BrandKitPayload, String>,
        now_ms: u64,
    ) -> bool {
        if generation != self.generation
            || !matches!(self.status, HomeBrandStatus::Extracting { .. })
        {
            return false;
        }
        let status = match result {
            Ok(kit) => HomeBrandStatus::Ready(Box::new(kit)),
            Err(detail) => HomeBrandStatus::Failed { detail },
        };
        self.set_status(status, now_ms);
        true
    }

    /// Remove the staged kit (or cancel a running extraction).
    pub fn clear(&mut self) {
        self.generation += 1;
        self.pending = None;
        self.status = HomeBrandStatus::Idle;
        self.status_at_ms = 0;
    }

    /// The kit the next send applies.
    pub fn staged(&self) -> Option<&BrandKitPayload> {
        match &self.status {
            HomeBrandStatus::Ready(kit) => Some(kit),
            _ => None,
        }
    }

    pub fn is_extracting(&self) -> bool {
        matches!(self.status, HomeBrandStatus::Extracting { .. })
    }

    /// Whether the transient hint (needs-source / failure) is still up.
    pub fn hint_visible(&self, now_ms: u64) -> bool {
        matches!(
            self.status,
            HomeBrandStatus::NeedsSource | HomeBrandStatus::Failed { .. }
        ) && now_ms.saturating_sub(self.status_at_ms) < BRAND_HINT_MS
    }

    /// When the transient hint expires (the host wakes to erase it).
    pub fn hint_deadline_ms(&self) -> Option<u64> {
        matches!(
            self.status,
            HomeBrandStatus::NeedsSource | HomeBrandStatus::Failed { .. }
        )
        .then(|| self.status_at_ms.saturating_add(BRAND_HINT_MS))
    }

    /// The chip is painted while extracting or when a kit is staged.
    pub fn chip_visible(&self) -> bool {
        matches!(
            self.status,
            HomeBrandStatus::Extracting { .. } | HomeBrandStatus::Ready(_)
        )
    }

    fn set_status(&mut self, status: HomeBrandStatus, now_ms: u64) {
        self.status = status;
        self.status_at_ms = now_ms.max(1);
    }
}

/// The first `http://` / `https://` link in `text`, trailing punctuation
/// (ASCII and CJK) trimmed.
pub fn first_url(text: &str) -> Option<String> {
    let start = ["https://", "http://"]
        .iter()
        .filter_map(|scheme| text.find(scheme))
        .min()?;
    let tail = &text[start..];
    let end = tail
        .find(|c: char| c.is_whitespace() || "<>\"'`，。、；）】」".contains(c))
        .unwrap_or(tail.len());
    let url = tail[..end].trim_end_matches(['.', ',', ';', ':', ')', ']', '!', '?']);
    let host = url.split("://").nth(1).unwrap_or("");
    (!host.is_empty()).then(|| url.to_string())
}

#[cfg(test)]
#[path = "home_brand_tests.rs"]
mod tests;
