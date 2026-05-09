use std::{fmt, marker::PhantomData, mem};

use skia_bindings::{
    self as sb, SkFontArguments, SkFontArguments_Palette, SkFontArguments_VariationPosition,
};

use crate::prelude::*;

/// Represents a set of actual arguments for a font.
#[repr(C)]
pub struct FontArguments<'vp, 'p> {
    args: SkFontArguments,
    pd_vp: PhantomData<&'vp [variation_position::Coordinate]>,
    pd_p: PhantomData<&'p [palette::Override]>,
}

native_transmutable!(SkFontArguments, FontArguments<'_, '_>);

/// Represents a position in the variation design space.
///
/// Any axis not specified uses the default value.
/// Any specified axis not actually present in the font is ignored.
#[derive(Clone, Debug)]
pub struct VariationPosition<'a> {
    pub coordinates: &'a [variation_position::Coordinate],
}

pub mod variation_position {
    use crate::FourByteTag;
    use skia_bindings::SkFontArguments_VariationPosition_Coordinate;

    /// A single axis/value pair in a [`crate::font_arguments::VariationPosition`].
    #[derive(Copy, Clone, PartialEq, Default, Debug)]
    #[repr(C)]
    pub struct Coordinate {
        pub axis: FourByteTag,
        pub value: f32,
    }

    native_transmutable!(SkFontArguments_VariationPosition_Coordinate, Coordinate);
}

/// Specifies a palette to use and overrides for palette entries.
///
/// `overrides` is a list of pairs of palette entry index and color.
/// Overridden palette entries use the associated color.
///
/// Override pairs with palette entry indices out of range are not applied.
/// Later override entries override earlier ones.
#[derive(Clone, Debug)]
pub struct Palette<'a> {
    pub index: i32,
    pub overrides: &'a [palette::Override],
}

pub mod palette {
    use crate::Color;
    use skia_bindings::SkFontArguments_Palette_Override;

    /// A palette entry override.
    #[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
    #[repr(C)]
    pub struct Override {
        pub index: u16,
        pub color: Color,
    }

    native_transmutable!(SkFontArguments_Palette_Override, Override);
}

impl Drop for FontArguments<'_, '_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkFontArguments_destruct(self.native_mut()) }
    }
}

impl Default for FontArguments<'_, '_> {
    fn default() -> Self {
        FontArguments::new()
    }
}

impl fmt::Debug for FontArguments<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontArguments")
            .field("collection_index", &self.collection_index())
            .field(
                "variation_design_position",
                &self.variation_design_position(),
            )
            .field("palette", &self.palette())
            .field("synthetic_bold", &self.synthetic_bold())
            .field("synthetic_oblique", &self.synthetic_oblique())
            .finish()
    }
}

impl FontArguments<'_, '_> {
    /// Creates default font arguments.
    pub fn new() -> Self {
        Self::construct(|fa| unsafe {
            sb::C_SkFontArguments_construct(fa);
        })
    }

    /// Specifies the index of the desired font.
    ///
    /// Font formats like ttc, dfont, cff, cid, pfr, t42, t1, and fon may actually be indexed
    /// collections of fonts.
    ///
    /// - `collection_index`: index of the font in an indexed collection.
    pub fn set_collection_index(&mut self, collection_index: usize) -> &mut Self {
        self.native_mut().fCollectionIndex = collection_index.try_into().unwrap();
        self
    }

    // This function consumes self for it to be able to change its lifetime,
    // because it borrows the coordinates referenced by [`VariationPosition`].
    //
    // If we would return `Self`, position's Coordinates would not be borrowed.
    /// Specifies a position in the variation design space.
    ///
    /// Any axis not specified uses the default value.
    /// Any specified axis not actually present in the font is ignored.
    ///
    /// This borrows `position` data; the value must remain valid for the lifetime of
    /// [`FontArguments`].
    ///
    /// - `position`: variation coordinates to use.
    pub fn set_variation_design_position(mut self, position: VariationPosition) -> FontArguments {
        let position = SkFontArguments_VariationPosition {
            coordinates: position.coordinates.native().as_ptr(),
            coordinateCount: position.coordinates.len().try_into().unwrap(),
        };
        unsafe {
            sb::C_SkFontArguments_setVariationDesignPosition(self.native_mut(), position);
            // note: we are _not_ returning Self here, but VariationPosition with a
            // changed lifetime.
            mem::transmute(self)
        }
    }

    /// Returns the index of the selected font in an indexed collection.
    pub fn collection_index(&self) -> usize {
        self.native().fCollectionIndex.try_into().unwrap()
    }

    /// Returns the variation design position.
    pub fn variation_design_position(&self) -> VariationPosition {
        unsafe {
            let position = sb::C_SkFontArguments_getVariationDesignPosition(self.native());
            VariationPosition {
                coordinates: safer::from_raw_parts(
                    position.coordinates as *const _,
                    position.coordinateCount.try_into().unwrap(),
                ),
            }
        }
    }

    // This function consumes `self` for it to be able to change its lifetime, because it borrows
    // the coordinates referenced by `[Palette]`.
    /// Specifies the color palette and optional palette entry overrides.
    ///
    /// This borrows `palette` data; the value must remain valid for the lifetime of
    /// [`FontArguments`].
    ///
    /// - `palette`: palette index and override entries.
    pub fn set_palette(mut self, palette: Palette) -> FontArguments {
        let palette = SkFontArguments_Palette {
            index: palette.index,
            overrides: palette.overrides.native().as_ptr(),
            overrideCount: palette.overrides.len().try_into().unwrap(),
        };
        unsafe {
            sb::C_SkFontArguments_setPalette(self.native_mut(), palette);
            mem::transmute(self)
        }
    }

    /// Returns the palette selection and override entries.
    pub fn palette(&self) -> Palette {
        unsafe {
            let palette = sb::C_SkFontArguments_getPalette(self.native());
            Palette {
                index: palette.index,
                overrides: safer::from_raw_parts(
                    palette.overrides as *const _,
                    palette.overrideCount.try_into().unwrap(),
                ),
            }
        }
    }

    /// Sets whether synthetic bold styling is requested.
    ///
    /// - `synthetic_bold`: `Some(true)` to force synthetic bold,
    ///   `Some(false)` to force non-bold, `None` to leave unspecified.
    pub fn set_synthetic_bold(&mut self, synthetic_bold: impl Into<Option<bool>>) -> &mut Self {
        unsafe {
            sb::C_SkFontArguments_setSyntheticBold(
                self.native_mut(),
                option_bool_to_ffi(synthetic_bold.into()),
            );
        }
        self
    }

    /// Returns the synthetic bold preference, if specified.
    pub fn synthetic_bold(&self) -> Option<bool> {
        ffi_to_option_bool(unsafe { sb::C_SkFontArguments_getSyntheticBold(self.native()) })
    }

    /// Sets whether synthetic oblique styling is requested.
    ///
    /// - `synthetic_oblique`: `Some(true)` to force synthetic oblique,
    ///   `Some(false)` to force non-oblique, `None` to leave unspecified.
    pub fn set_synthetic_oblique(
        &mut self,
        synthetic_oblique: impl Into<Option<bool>>,
    ) -> &mut Self {
        unsafe {
            sb::C_SkFontArguments_setSyntheticOblique(
                self.native_mut(),
                option_bool_to_ffi(synthetic_oblique.into()),
            );
        }
        self
    }

    /// Returns the synthetic oblique preference, if specified.
    pub fn synthetic_oblique(&self) -> Option<bool> {
        ffi_to_option_bool(unsafe { sb::C_SkFontArguments_getSyntheticOblique(self.native()) })
    }
}

fn option_bool_to_ffi(value: Option<bool>) -> i32 {
    match value {
        Some(true) => 1,
        Some(false) => 0,
        None => -1,
    }
}

fn ffi_to_option_bool(value: i32) -> Option<bool> {
    match value {
        1 => Some(true),
        0 => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_arguments_with_no_coordinates() {
        let fa = FontArguments::new();
        let coordinates = fa.variation_design_position();
        assert_eq!(coordinates.coordinates, []);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn access_coordinates() {
        let coordinates = Box::new([variation_position::Coordinate {
            axis: 0.into(),
            value: 1.0,
        }]);
        let args = FontArguments::new();
        let pos = VariationPosition {
            coordinates: coordinates.as_ref(),
        };
        let args = args.set_variation_design_position(pos);
        assert_eq!(args.variation_design_position().coordinates[0].value, 1.0);
        drop(args);
    }

    #[test]
    fn synthetic_style_flags_roundtrip() {
        let mut args = FontArguments::new();

        assert_eq!(args.synthetic_bold(), None);
        assert_eq!(args.synthetic_oblique(), None);

        args.set_synthetic_bold(Some(true));
        assert_eq!(args.synthetic_bold(), Some(true));

        args.set_synthetic_bold(Some(false));
        assert_eq!(args.synthetic_bold(), Some(false));

        args.set_synthetic_bold(None);
        assert_eq!(args.synthetic_bold(), None);

        args.set_synthetic_oblique(Some(true));
        assert_eq!(args.synthetic_oblique(), Some(true));

        args.set_synthetic_oblique(Some(false));
        assert_eq!(args.synthetic_oblique(), Some(false));

        args.set_synthetic_oblique(None);
        assert_eq!(args.synthetic_oblique(), None);
    }
}
