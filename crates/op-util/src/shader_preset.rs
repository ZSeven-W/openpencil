//! Shared contracts for built-in shader-fill presets.

/// Smallest supported turbulence FBM octave count.
pub const MIN_NUM_OCTAVES: usize = 1;

/// Largest supported turbulence FBM octave count.
pub const MAX_NUM_OCTAVES: usize = 6;

/// Turbulence FBM octave count used when an authored value is unusable.
pub const DEFAULT_NUM_OCTAVES: usize = 3;

/// How an authored turbulence octave count resolves at load time.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OctaveResolution {
    /// The authored value is already inside the supported range.
    Exact(usize),
    /// A finite integer lies outside the supported range and is clamped.
    Clamped(usize),
    /// A non-finite or fractional value cannot be used, so the default wins.
    Defaulted,
}

impl OctaveResolution {
    /// Octave count the loader must bake into the generated SkSL.
    #[must_use]
    pub const fn effective(self) -> usize {
        match self {
            Self::Exact(value) | Self::Clamped(value) => value,
            Self::Defaulted => DEFAULT_NUM_OCTAVES,
        }
    }
}

/// Classify one authored turbulence octave count without renderer dependencies.
#[must_use]
pub fn resolve_num_octaves(value: f32) -> OctaveResolution {
    if !value.is_finite() || value.fract() != 0.0 {
        return OctaveResolution::Defaulted;
    }

    if value < MIN_NUM_OCTAVES as f32 {
        OctaveResolution::Clamped(MIN_NUM_OCTAVES)
    } else if value > MAX_NUM_OCTAVES as f32 {
        OctaveResolution::Clamped(MAX_NUM_OCTAVES)
    } else {
        OctaveResolution::Exact(value as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_integer_values_are_exact_or_clamped() {
        assert_eq!(resolve_num_octaves(1.0), OctaveResolution::Exact(1));
        assert_eq!(resolve_num_octaves(6.0), OctaveResolution::Exact(6));
        assert_eq!(resolve_num_octaves(0.0), OctaveResolution::Clamped(1));
        assert_eq!(resolve_num_octaves(99.0), OctaveResolution::Clamped(6));
    }

    #[test]
    fn fractional_and_non_finite_values_use_the_default() {
        for value in [2.5, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let resolution = resolve_num_octaves(value);
            assert_eq!(resolution, OctaveResolution::Defaulted);
            assert_eq!(resolution.effective(), DEFAULT_NUM_OCTAVES);
        }
    }
}
