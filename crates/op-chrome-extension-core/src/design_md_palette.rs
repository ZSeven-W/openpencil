//! Evidence-ranked palette roles for deterministic `design.md` output.

use std::collections::BTreeSet;

use crate::design_md::{ColorEvidence, Evidence};

#[derive(Debug)]
pub(super) struct PaletteColor {
    pub hex: String,
    pub role: &'static str,
    pub description: String,
}

pub(super) fn select_palette(evidence: &Evidence, background: &str) -> Vec<PaletteColor> {
    let mut colors = evidence.colors.clone();
    colors.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| usage_rank(&a.usage).cmp(&usage_rank(&b.usage)))
            .then_with(|| a.value.cmp(&b.value))
    });
    let surfaces = observed_colors(&colors, "background", background);
    let texts = observed_colors(&colors, "text", background);
    let borders = observed_colors(&colors, "border", background);
    let gradients = observed_colors(&colors, "gradient", background);

    let surface = card_surface(evidence, background)
        .or_else(|| {
            surfaces
                .iter()
                .find(|value| value.hex != background)
                .cloned()
        })
        .unwrap_or_else(|| ObservedColor::derived(background));
    let primary_text = texts.first().cloned().unwrap_or_else(|| {
        ObservedColor::derived(if relative_luminance(background) < 0.35 {
            "#FFFFFF"
        } else {
            "#111111"
        })
    });
    let secondary_text = texts
        .iter()
        .find(|value| value.hex != primary_text.hex)
        .cloned()
        .unwrap_or_else(|| ObservedColor::derived(&mix_hex(&primary_text.hex, background, 184)));
    let muted_text = texts
        .iter()
        .find(|value| value.hex != primary_text.hex && value.hex != secondary_text.hex)
        .cloned()
        .unwrap_or_else(|| ObservedColor::derived(&mix_hex(&primary_text.hex, background, 128)));
    let border = borders
        .first()
        .cloned()
        .unwrap_or_else(|| ObservedColor::derived(&mix_hex(&primary_text.hex, background, 36)));
    let accent = gradients.first().cloned().unwrap_or_else(|| {
        colors
            .iter()
            .map(|value| ObservedColor {
                hex: opaque_hex(&value.value, background),
                count: Some(value.count),
            })
            .filter(|value| value.hex != background && value.hex != surface.hex)
            .max_by(|a, b| {
                chroma(&a.hex)
                    .cmp(&chroma(&b.hex))
                    .then_with(|| a.count.cmp(&b.count))
                    .then_with(|| b.hex.cmp(&a.hex))
            })
            .unwrap_or_else(|| primary_text.clone())
    });

    vec![
        palette("Page Background", background, None, "Root page background"),
        palette(
            "Card Surface",
            &surface.hex,
            surface.count,
            "Cards and elevated containers",
        ),
        palette(
            "Primary Accent",
            &accent.hex,
            accent.count,
            "Primary actions and emphasis",
        ),
        palette(
            "Primary Text",
            &primary_text.hex,
            primary_text.count,
            "Headings and primary content",
        ),
        palette(
            "Secondary Text",
            &secondary_text.hex,
            secondary_text.count,
            "Body copy and descriptions",
        ),
        palette(
            "Muted Text",
            &muted_text.hex,
            muted_text.count,
            "Captions and low-emphasis metadata",
        ),
        palette(
            "Default Border",
            &border.hex,
            border.count,
            "Dividers, controls, and container outlines",
        ),
    ]
}

fn card_surface(evidence: &Evidence, background: &str) -> Option<ObservedColor> {
    let mut counts = std::collections::BTreeMap::<String, u64>::new();
    for component in &evidence.components {
        if !is_card_like(&component.kind) {
            continue;
        }
        for sample in &component.samples {
            let Some(value) = sample.background.as_deref() else {
                continue;
            };
            let hex = opaque_hex(value, background);
            if hex != background {
                *counts.entry(hex).or_default() += component.count;
            }
        }
    }
    counts
        .into_iter()
        .max_by(|(hex_a, count_a), (hex_b, count_b)| {
            count_a.cmp(count_b).then_with(|| hex_b.cmp(hex_a))
        })
        .map(|(hex, count)| ObservedColor {
            hex,
            count: Some(count),
        })
}

fn is_card_like(kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    [
        "card",
        "panel",
        "modal",
        "dialog",
        "popover",
        "container",
        "article",
        "section",
        "aside",
        "fieldset",
        "form",
        "table",
        "menu",
        "alert",
    ]
    .iter()
    .any(|token| kind.contains(token))
}

#[derive(Debug, Clone)]
struct ObservedColor {
    hex: String,
    count: Option<u64>,
}

impl ObservedColor {
    fn derived(hex: &str) -> Self {
        Self {
            hex: hex.to_owned(),
            count: None,
        }
    }
}

fn observed_colors(colors: &[ColorEvidence], usage: &str, background: &str) -> Vec<ObservedColor> {
    let mut seen = BTreeSet::new();
    colors
        .iter()
        .filter(|value| value.usage == usage)
        .filter_map(|value| {
            let hex = opaque_hex(&value.value, background);
            seen.insert(hex.clone()).then_some(ObservedColor {
                hex,
                count: Some(value.count),
            })
        })
        .collect()
}

fn palette(role: &'static str, hex: &str, count: Option<u64>, usage: &'static str) -> PaletteColor {
    let description = match count {
        Some(count) => format!("{usage}; observed {count} times"),
        None => format!("{usage}; deterministic fallback"),
    };
    PaletteColor {
        hex: hex.to_owned(),
        role,
        description,
    }
}

pub(super) fn opaque_hex(value: &str, background: &str) -> String {
    let Some((red, green, blue, alpha)) = rgba(value) else {
        return "#000000".to_owned();
    };
    if alpha == 255 {
        return format!("#{red:02X}{green:02X}{blue:02X}");
    }
    let (br, bg, bb, _) = rgba(background).unwrap_or((255, 255, 255, 255));
    let blend = |front: u8, back: u8| -> u8 {
        let alpha = u32::from(alpha);
        (((u32::from(front) * alpha) + (u32::from(back) * (255 - alpha)) + 127) / 255) as u8
    };
    format!(
        "#{:02X}{:02X}{:02X}",
        blend(red, br),
        blend(green, bg),
        blend(blue, bb)
    )
}

fn mix_hex(front: &str, back: &str, opacity: u8) -> String {
    let (fr, fg, fb, _) = rgba(front).unwrap_or((0, 0, 0, 255));
    let (br, bg, bb, _) = rgba(back).unwrap_or((255, 255, 255, 255));
    let mix = |front: u8, back: u8| {
        let alpha = u32::from(opacity);
        (((u32::from(front) * alpha) + (u32::from(back) * (255 - alpha)) + 127) / 255) as u8
    };
    format!("#{:02X}{:02X}{:02X}", mix(fr, br), mix(fg, bg), mix(fb, bb))
}

fn chroma(value: &str) -> u8 {
    let Some((red, green, blue, _)) = rgba(value) else {
        return 0;
    };
    red.max(green).max(blue) - red.min(green).min(blue)
}

fn rgba(value: &str) -> Option<(u8, u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) {
        return None;
    }
    let channel = |start| u8::from_str_radix(&hex[start..start + 2], 16).ok();
    Some((
        channel(0)?,
        channel(2)?,
        channel(4)?,
        if hex.len() == 8 { channel(6)? } else { 255 },
    ))
}

pub(super) fn relative_luminance(value: &str) -> f64 {
    let Some((red, green, blue, _)) = rgba(value) else {
        return 1.0;
    };
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue)
}

fn usage_rank(usage: &str) -> u8 {
    match usage {
        "text" => 0,
        "background" => 1,
        "border" => 2,
        "gradient" => 3,
        "shadow" => 4,
        _ => 5,
    }
}
