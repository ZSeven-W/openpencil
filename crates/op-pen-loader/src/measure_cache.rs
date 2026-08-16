//! Memoizing wrapper around a [`MeasureBackend`].
//!
//! `editor_state_to_layout_scene` rebuilds the layout scene whenever the
//! document is dirty, re-running the taffy flex solve — which calls
//! `MeasureBackend::measure` once per text leaf. Under the real skia shaper
//! (`SkiaMeasure`) each call builds and lays out a fresh `Paragraph`, the single
//! most expensive op in the layout pass. Text rarely changes between
//! reconversions (a drag / resize / colour edit leaves every string identical),
//! so memoizing on the full shaping input turns repeat measurements into hash
//! lookups. The backend lives in a thread-local that outlives individual passes,
//! so the cache spans reconversions.
//!
//! Measurement is a pure function of its inputs *for a fixed font set*, so
//! cached entries only go stale when the process-global font registry changes
//! (a runtime import/removal bumps [`jian_skia::font_generation`]). The cache
//! watches that generation and drops every entry when it advances, so an
//! already-open document re-measures with the new faces. Within a generation it
//! only grows with the set of distinct (text, font, width) tuples in play and
//! is bounded by [`MAX_ENTRIES`].

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use jian_core::layout::measure::{FontStyleKind, MeasureBackend, MeasureRequest, MeasureResult};

/// Cap on distinct cached measurements. Sized above the snapshot importer's
/// 40,000-node budget (a text-heavy web capture can carry tens of thousands
/// of distinct text leaves — at the old 8,192 such an import cleared the
/// cache mid-pass and re-measured at a near-0% hit rate); past this, drop the
/// whole cache rather than track LRU order — a full refill is one layout
/// pass, far cheaper than per-measure bookkeeping.
const MAX_ENTRIES: usize = 49_152;

/// Owned, hashable mirror of a `StyledRun`. `f32` fields are keyed by their bit
/// pattern (`to_bits`) since `f32` is not `Hash`/`Eq`; font sizes / spacings are
/// never `NaN` in practice, so distinct bit patterns map to distinct keys.
#[derive(PartialEq, Eq, Hash)]
struct RunKey {
    text: String,
    font_family: Option<String>,
    font_size_bits: u32,
    font_weight: u16,
    italic: bool,
    letter_spacing_bits: u32,
}

#[derive(PartialEq, Eq, Hash)]
struct MeasureKey {
    runs: Vec<RunKey>,
    line_height_bits: u32,
    max_width_bits: Option<u32>,
}

impl MeasureKey {
    fn from_request(req: &MeasureRequest<'_>) -> Self {
        let runs = req
            .runs
            .iter()
            .map(|run| RunKey {
                text: run.text.to_string(),
                font_family: run.font_family.map(str::to_string),
                font_size_bits: run.font_size.to_bits(),
                font_weight: run.font_weight,
                italic: matches!(run.font_style, FontStyleKind::Italic),
                letter_spacing_bits: run.letter_spacing.to_bits(),
            })
            .collect();
        MeasureKey {
            runs,
            line_height_bits: req.line_height.to_bits(),
            max_width_bits: req.max_width.map(f32::to_bits),
        }
    }
}

/// Wraps a `MeasureBackend`, memoizing results keyed on the full shaping input.
/// `&self` measurement (Taffy hands an immutable context) uses interior
/// mutability via `RefCell`, the single-threaded pattern the trait documents.
pub struct CachingMeasureBackend {
    inner: Rc<dyn MeasureBackend>,
    cache: RefCell<HashMap<MeasureKey, MeasureResult>>,
    built_generation: Cell<u64>,
}

impl CachingMeasureBackend {
    pub fn new(inner: Rc<dyn MeasureBackend>) -> Self {
        Self {
            inner,
            cache: RefCell::new(HashMap::new()),
            built_generation: Cell::new(crate::current_font_generation()),
        }
    }
}

impl MeasureBackend for CachingMeasureBackend {
    fn measure(&self, req: &MeasureRequest<'_>) -> MeasureResult {
        // A runtime font import/removal invalidates every memoized width.
        let generation = crate::current_font_generation();
        if generation != self.built_generation.get() {
            self.cache.borrow_mut().clear();
            self.built_generation.set(generation);
        }
        let key = MeasureKey::from_request(req);
        if let Some(hit) = self.cache.borrow().get(&key) {
            return *hit;
        }
        let result = self.inner.measure(req);
        let mut cache = self.cache.borrow_mut();
        // Cheap bound: a runaway distinct-key set just resets (a full refill is
        // one layout pass) rather than paying LRU bookkeeping on every measure.
        if cache.len() >= MAX_ENTRIES {
            cache.clear();
        }
        cache.insert(key, result);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    use jian_core::layout::measure::StyledRun;

    /// Records how many times the wrapped backend was actually invoked, and
    /// returns an input-derived result so pass-through is verifiable.
    struct CountingBackend {
        calls: Cell<usize>,
    }

    impl MeasureBackend for CountingBackend {
        fn measure(&self, req: &MeasureRequest<'_>) -> MeasureResult {
            self.calls.set(self.calls.get() + 1);
            let width: f32 = req.runs.iter().map(|r| r.text.len() as f32).sum();
            MeasureResult {
                width,
                height: 4.0,
                line_count: 1,
                baseline: 3.2,
            }
        }
    }

    fn run(text: &str, size: f32, weight: u16) -> StyledRun<'_> {
        StyledRun {
            text,
            font_family: None,
            font_size: size,
            font_weight: weight,
            font_style: FontStyleKind::Normal,
            letter_spacing: 0.0,
        }
    }

    #[test]
    fn identical_requests_hit_the_cache() {
        let counter = Rc::new(CountingBackend {
            calls: Cell::new(0),
        });
        let backend = CachingMeasureBackend::new(counter.clone());

        let runs = [run("hello", 16.0, 400)];
        let req = MeasureRequest {
            runs: &runs,
            line_height: 0.0,
            max_width: None,
        };

        let first = backend.measure(&req);
        let second = backend.measure(&req);

        assert_eq!(
            counter.calls.get(),
            1,
            "second identical measure must hit the cache"
        );
        assert_eq!(first.width, second.width);
        assert_eq!(
            first.width, 5.0,
            "result must pass through from the inner backend"
        );
    }

    #[test]
    fn distinct_inputs_each_measure_once() {
        let counter = Rc::new(CountingBackend {
            calls: Cell::new(0),
        });
        let backend = CachingMeasureBackend::new(counter.clone());

        // Each axis that feeds shaping must produce a distinct cache key.
        let cases: &[(&str, f32, u16, f32, Option<f32>)] = &[
            ("hello", 16.0, 400, 0.0, None),
            ("world", 16.0, 400, 0.0, None),       // text differs
            ("hello", 18.0, 400, 0.0, None),       // size differs
            ("hello", 16.0, 700, 0.0, None),       // weight differs
            ("hello", 16.0, 400, 0.0, Some(80.0)), // max_width differs
            ("hello", 16.0, 400, 2.0, None),       // line_height differs
        ];
        for (text, size, weight, line_height, max_width) in cases {
            let runs = [run(text, *size, *weight)];
            let req = MeasureRequest {
                runs: &runs,
                line_height: *line_height,
                max_width: *max_width,
            };
            backend.measure(&req);
            // Re-measuring the same case must not add a call.
            backend.measure(&req);
        }

        assert_eq!(
            counter.calls.get(),
            cases.len(),
            "each distinct shaping input measures exactly once"
        );
    }
}
