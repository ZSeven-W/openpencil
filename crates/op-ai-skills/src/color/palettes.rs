use std::collections::BTreeMap;

use super::oklch::{oklch_to_hex, scale12, Mode as OklchMode, Oklch};

pub const PALETTE_COUNT: usize = 7;
pub const HARSH_PALETTE_NAME: &str = "Amber Field";
pub const HARSH_ROLE: &str = "accent.primary";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaletteSeed {
    pub name: &'static str,
    pub neutral_hue: f64,
    pub accent_hue: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteMode {
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaletteAnchors {
    pub name: &'static str,
    pub mode: PaletteMode,
    pub roles: BTreeMap<String, String>,
    pub neutral_scale: [String; 12],
    pub accent_scale: [String; 12],
}

const PALETTE_SEEDS: [PaletteSeed; PALETTE_COUNT] = [
    PaletteSeed {
        name: "Alloy Blue",
        neutral_hue: 252.0,
        accent_hue: 248.0,
    },
    PaletteSeed {
        name: "Harbor Teal",
        neutral_hue: 218.0,
        accent_hue: 188.0,
    },
    PaletteSeed {
        name: "Fern Signal",
        neutral_hue: 156.0,
        accent_hue: 142.0,
    },
    PaletteSeed {
        name: "Ember Coral",
        neutral_hue: 28.0,
        accent_hue: 24.0,
    },
    PaletteSeed {
        name: "Iris Steel",
        neutral_hue: 266.0,
        accent_hue: 286.0,
    },
    PaletteSeed {
        name: HARSH_PALETTE_NAME,
        neutral_hue: 92.0,
        accent_hue: 82.0,
    },
    PaletteSeed {
        name: "Rose Circuit",
        neutral_hue: 334.0,
        accent_hue: 342.0,
    },
];

pub fn palette_names() -> Vec<&'static str> {
    PALETTE_SEEDS.iter().map(|seed| seed.name).collect()
}

pub fn palette_seed(name: &str) -> Option<&'static PaletteSeed> {
    PALETTE_SEEDS
        .iter()
        .find(|seed| seed.name.eq_ignore_ascii_case(name.trim()))
}

pub fn palette_anchors(name: &str, mode: PaletteMode) -> Option<PaletteAnchors> {
    palette_seed(name).map(|seed| palette_anchors_from_seed(seed, mode))
}

pub fn palette_anchors_from_seed(seed: &PaletteSeed, mode: PaletteMode) -> PaletteAnchors {
    let oklch_mode = match mode {
        PaletteMode::Light => OklchMode::Light,
        PaletteMode::Dark => OklchMode::Dark,
    };
    let neutral_scale = scale12(seed.neutral_hue, 0.006, oklch_mode, true);
    let accent_scale = scale12(seed.accent_hue, 0.118, oklch_mode, false);
    let mut roles = BTreeMap::new();

    roles.insert("surface.primary".to_string(), neutral_scale[0].clone());
    roles.insert("surface.secondary".to_string(), neutral_scale[1].clone());
    roles.insert("surface.inverse".to_string(), neutral_scale[11].clone());
    roles.insert("foreground.primary".to_string(), neutral_scale[11].clone());
    roles.insert(
        "foreground.secondary".to_string(),
        neutral_scale[10].clone(),
    );
    roles.insert("foreground.muted".to_string(), neutral_scale[10].clone());
    roles.insert("foreground.inverse".to_string(), neutral_scale[0].clone());
    roles.insert("border.subtle".to_string(), neutral_scale[5].clone());

    let accent_primary = if seed.name == HARSH_PALETTE_NAME {
        oklch_to_hex(Oklch {
            l: match mode {
                PaletteMode::Light => 0.86,
                PaletteMode::Dark => 0.80,
            },
            c: 0.18,
            h: seed.accent_hue,
        })
    } else {
        oklch_to_hex(Oklch {
            l: match mode {
                PaletteMode::Light => 0.48,
                PaletteMode::Dark => 0.78,
            },
            c: 0.118,
            h: seed.accent_hue,
        })
    };
    roles.insert(HARSH_ROLE.to_string(), accent_primary);

    PaletteAnchors {
        name: seed.name,
        mode,
        roles,
        neutral_scale,
        accent_scale,
    }
}
