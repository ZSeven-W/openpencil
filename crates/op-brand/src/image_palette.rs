//! Screenshot pixels → [`BrandSignals`], deterministically.
//!
//! The image is downsampled to at most [`SAMPLE_EDGE`] px on its long side,
//! then colour-quantised: pixels fall into 5-bit-per-channel bins, bins
//! are merged greedily (largest first) into swatches whose members sit
//! within [`MERGE_DISTANCE`] of the swatch mean. That is a median-cut
//! style reduction without its randomness — the same bytes always yield
//! the same kit. Roles are then read off the swatches:
//!
//! - **background**: the swatch that owns the image frame (outer band);
//! - **foreground**: the highest-contrast near-neutral swatch against it;
//! - **primary**: the most prominent *colourful* swatch (share × chroma);
//! - **card / border**: neutral swatches just off the background.

use std::collections::HashMap;

use crate::color::{best_on, hue_distance, Rgb};
use crate::error::BrandError;
use crate::signals::{BrandSignals, ModeSignals};

const SAMPLE_EDGE: u32 = 160;
const MERGE_DISTANCE: f64 = 34.0;
const MAX_SWATCHES: usize = 24;
/// Outer band (fraction of each dimension) read as the page frame.
const FRAME_BAND: f64 = 0.04;

/// A histogram bin: `(pixel count, frame-band count, channel sums)`.
type Bin = (u64, u64, [u64; 3]);

/// One quantised colour cluster.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    pub color: Rgb,
    /// Fraction of opaque sampled pixels, `0..=1`.
    pub share: f64,
    /// Fraction of frame-band pixels.
    pub frame_share: f64,
}

/// Decode, downsample, and quantise `bytes` (PNG / JPEG).
pub fn palette_from_image(bytes: &[u8]) -> Result<Vec<Swatch>, BrandError> {
    let image = image::load_from_memory(bytes).map_err(|e| BrandError::ImageDecode {
        detail: e.to_string(),
    })?;
    let small = image.thumbnail(SAMPLE_EDGE, SAMPLE_EDGE).to_rgba8();
    let (w, h) = small.dimensions();
    if w == 0 || h == 0 {
        return Err(BrandError::ImageEmpty);
    }
    let band_x = ((f64::from(w) * FRAME_BAND).ceil() as u32).max(1);
    let band_y = ((f64::from(h) * FRAME_BAND).ceil() as u32).max(1);

    // bin key → (count, frame count, channel sums)
    let mut bins: HashMap<u16, Bin> = HashMap::new();
    let mut total = 0u64;
    let mut frame_total = 0u64;
    for (x, y, px) in small.enumerate_pixels() {
        let [r, g, b, a] = px.0;
        if a < 128 {
            continue;
        }
        let key = (u16::from(r >> 3) << 10) | (u16::from(g >> 3) << 5) | u16::from(b >> 3);
        let frame = x < band_x || y < band_y || x >= w - band_x || y >= h - band_y;
        let entry = bins.entry(key).or_insert((0, 0, [0; 3]));
        entry.0 += 1;
        entry.2[0] += u64::from(r);
        entry.2[1] += u64::from(g);
        entry.2[2] += u64::from(b);
        total += 1;
        if frame {
            entry.1 += 1;
            frame_total += 1;
        }
    }
    if total == 0 {
        return Err(BrandError::ImageEmpty);
    }

    let mut ordered: Vec<(u16, Bin)> = bins.into_iter().collect();
    ordered.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(&b.0)));
    struct Acc {
        count: u64,
        frame: u64,
        sums: [u64; 3],
    }
    let mut clusters: Vec<Acc> = Vec::new();
    for (_, (count, frame, sums)) in ordered {
        let mean = mean_of(count, sums);
        let nearest = clusters
            .iter_mut()
            .map(|c| {
                let d = mean_of(c.count, c.sums).distance(mean);
                (d, c)
            })
            .filter(|(d, _)| *d < MERGE_DISTANCE)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match nearest {
            Some((_, cluster)) => {
                cluster.count += count;
                cluster.frame += frame;
                for (acc, add) in cluster.sums.iter_mut().zip(sums) {
                    *acc += add;
                }
            }
            None => clusters.push(Acc { count, frame, sums }),
        }
    }
    let mut swatches: Vec<Swatch> = clusters
        .iter()
        .map(|c| Swatch {
            color: mean_of(c.count, c.sums),
            share: c.count as f64 / total as f64,
            frame_share: if frame_total == 0 {
                0.0
            } else {
                c.frame as f64 / frame_total as f64
            },
        })
        .collect();
    swatches.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.color.cmp(&b.color)));
    swatches.truncate(MAX_SWATCHES);
    Ok(swatches)
}

fn mean_of(count: u64, sums: [u64; 3]) -> Rgb {
    let c = count.max(1);
    Rgb::new(
        (sums[0] / c) as u8,
        (sums[1] / c) as u8,
        (sums[2] / c) as u8,
    )
}

/// Swatches → signals for the scheme the screenshot shows.
pub fn signals_from_swatches(swatches: &[Swatch]) -> BrandSignals {
    let mut mode = ModeSignals::default();
    let Some(bg) = swatches
        .iter()
        .max_by(|a, b| {
            (a.frame_share * 2.0 + a.share)
                .total_cmp(&(b.frame_share * 2.0 + b.share))
                .then(b.color.cmp(&a.color))
        })
        .map(|s| s.color)
    else {
        return BrandSignals::default();
    };
    mode.background = Some(bg);
    mode.note("screenshot background: dominant frame colour");

    let visible: Vec<&Swatch> = swatches
        .iter()
        .filter(|s| s.share >= 0.002 && s.color != bg)
        .collect();

    // Ink: the strongest-contrast neutral (text is rarely saturated).
    let ink = visible
        .iter()
        .filter(|s| s.color.chroma() < 0.22)
        .max_by(|a, b| {
            a.color
                .contrast(bg)
                .total_cmp(&b.color.contrast(bg))
                .then(b.color.cmp(&a.color))
        })
        .map(|s| s.color)
        .filter(|c| c.contrast(bg) >= 3.0);
    mode.foreground = Some(ink.unwrap_or_else(|| best_on(bg)));
    mode.note(if ink.is_some() {
        "screenshot foreground: highest-contrast neutral"
    } else {
        "screenshot foreground: black/white for contrast"
    });

    // Brand: prominence × colourfulness, away from the ground.
    let score = |s: &Swatch| {
        let (_, _, l) = s.color.to_hsl();
        s.share.sqrt() * s.color.chroma() * (1.0 - (l - 0.5).abs())
    };
    let mut colourful: Vec<&Swatch> = visible
        .iter()
        .copied()
        .filter(|s| {
            s.color.is_chromatic() && s.color.chroma() >= 0.2 && s.color.distance(bg) > 40.0
        })
        .collect();
    colourful.sort_by(|a, b| score(b).total_cmp(&score(a)).then(a.color.cmp(&b.color)));
    if let Some(primary) = colourful.first() {
        mode.primary = Some(primary.color);
        mode.note("screenshot primary: most prominent colourful swatch");
        for s in colourful.iter().skip(1) {
            let distinct_hue = hue_distance(s.color, primary.color) >= 30.0;
            if distinct_hue && !mode.extra_brand.iter().any(|e| e.distance(s.color) < 60.0) {
                mode.extra_brand.push(s.color);
            }
            if mode.extra_brand.len() >= 4 {
                break;
            }
        }
    } else {
        mode.note("screenshot primary: none colourful, using ink (monochrome brand)");
    }

    // Surfaces just off the ground: a card tone and a hairline tone.
    let neutrals: Vec<&Swatch> = visible
        .iter()
        .copied()
        .filter(|s| s.color.chroma() < 0.12 && s.share >= 0.01)
        .collect();
    mode.card = neutrals
        .iter()
        .filter(|s| s.color.contrast(bg) < 1.25)
        .max_by(|a, b| a.share.total_cmp(&b.share))
        .map(|s| s.color);
    mode.border = neutrals
        .iter()
        .filter(|s| {
            let c = s.color.contrast(bg);
            (1.15..2.2).contains(&c)
        })
        .max_by(|a, b| a.share.total_cmp(&b.share))
        .map(|s| s.color);

    BrandSignals {
        base: mode,
        ..BrandSignals::default()
    }
}

#[cfg(test)]
#[path = "image_palette_tests.rs"]
pub(crate) mod tests;
