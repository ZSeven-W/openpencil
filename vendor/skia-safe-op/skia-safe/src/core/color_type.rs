use crate::{prelude::*, AlphaType};
use sb::SkColorType;
use skia_bindings as sb;

/// Describes how pixel bits encode color. A pixel may be an alpha mask, a grayscale, RGB, or ARGB.
///
/// The names of each variant implicitly define the channel ordering and size in memory. Due to
/// historical reasons the names do not follow 100% identical convention, but are typically labeled
/// from least significant to most significant.
///
/// Unless specified otherwise, a channel's value is treated as an unsigned integer with a range of
/// [0, 2^N-1] and this is mapped uniformly to a floating point value of [0.0, 1.0]. Some color
/// types instead store data directly in 32-bit floating point (assumed to be IEEE), or in 16-bit
/// "half" floating point values.
///
/// Note: By default, Skia operates with the assumption of a little-Endian system. The bit patterns
/// shown in the documentation assume LE byte order.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[repr(i32)]
pub enum ColorType {
    /// Unknown or unrepresentable as an SkColorType.
    Unknown = SkColorType::kUnknown_SkColorType as _,
    /// Single channel data (8-bit) interpreted as an alpha value. RGB are 0.
    /// Bits: [A:7..0]
    Alpha8 = SkColorType::kAlpha_8_SkColorType as _,
    /// Three channel BGR data (5 bits red, 6 bits green, 5 bits blue) packed into a LE 16-bit word.
    /// Bits: [R:15..11 G:10..5 B:4..0]
    RGB565 = SkColorType::kRGB_565_SkColorType as _,
    /// Four channel ABGR data (4 bits per channel) packed into a LE 16-bit word.
    /// Bits: [R:15..12 G:11..8 B:7..4 A:3..0]
    ARGB4444 = SkColorType::kARGB_4444_SkColorType as _,
    /// Four channel RGBA data (8 bits per channel) packed into a LE 32-bit word.
    /// Bits: [A:31..24 B:23..16 G:15..8 R:7..0]
    RGBA8888 = SkColorType::kRGBA_8888_SkColorType as _,
    /// Three channel RGB data (8 bits per channel) packed into a LE 32-bit word. The remaining bits
    /// are ignored and alpha is forced to opaque.
    /// Bits: [x:31..24 B:23..16 G:15..8 R:7..0]
    RGB888x = SkColorType::kRGB_888x_SkColorType as _,
    /// Four channel BGRA data (8 bits per channel) packed into a LE 32-bit word. R and B are swapped
    /// relative to RGBA8888.
    /// Bits: [A:31..24 R:23..16 G:15..8 B:7..0]
    BGRA8888 = SkColorType::kBGRA_8888_SkColorType as _,
    /// Four channel RGBA data (10 bits per color, 2 bits for alpha) packed into a LE 32-bit word.
    /// Bits: [A:31..30 B:29..20 G:19..10 R:9..0]
    RGBA1010102 = SkColorType::kRGBA_1010102_SkColorType as _,
    /// Four channel BGRA data (10 bits per color, 2 bits for alpha) packed into a LE 32-bit word.
    /// R and B are swapped relative to RGBA1010102.
    /// Bits: [A:31..30 R:29..20 G:19..10 B:9..0]
    BGRA1010102 = SkColorType::kBGRA_1010102_SkColorType as _,
    /// Three channel RGB data (10 bits per channel) packed into a LE 32-bit word. The remaining bits
    /// are ignored and alpha is forced to opaque.
    /// Bits: [x:31..30 B:29..20 G:19..10 R:9..0]
    RGB101010x = SkColorType::kRGB_101010x_SkColorType as _,
    /// Three channel BGR data (10 bits per channel) packed into a LE 32-bit word. The remaining bits
    /// are ignored and alpha is forced to opaque. R and B are swapped relative to RGB101010x.
    /// Bits: [x:31..30 R:29..20 G:19..10 B:9..0]
    BGR101010x = SkColorType::kBGR_101010x_SkColorType as _,
    /// Three channel BGR data (10 bits per channel) packed into a LE 32-bit word. The remaining bits
    /// are ignored and alpha is forced to opaque. Instead of normalizing [0, 1023] to [0.0, 1.0] the
    /// color channels map to an extended range of [-0.752941, 1.25098].
    /// Bits: [x:31..30 R:29..20 G:19..10 B:9..0]
    BGR101010xXR = SkColorType::kBGR_101010x_XR_SkColorType as _,
    /// Four channel BGRA data (10 bits per channel) packed into a LE 64-bit word. Each channel is
    /// preceded by 6 bits of padding. Instead of normalizing [0, 1023] to [0.0, 1.0] the color and
    /// alpha channels map to an extended range of [-0.752941, 1.25098].
    /// Bits: [A:63..54 x:53..48 R:47..38 x:37..32 G:31..22 x:21..16 B:15..6 x:5..0]
    BGRA10101010XR = SkColorType::kBGRA_10101010_XR_SkColorType as _,
    /// Four channel RGBA data (10 bits per channel) packed into a LE 64-bit word. Each channel is
    /// preceded by 6 bits of padding.
    /// Bits: [A:63..54 x:53..48 B:47..38 x:37..32 G:31..22 x:21..16 R:15..6 x:5..0]
    RGBA10x6 = SkColorType::kRGBA_10x6_SkColorType as _,
    /// Single channel data (8-bit) interpreted as a grayscale value (e.g. replicated to RGB).
    /// Bits: [G:7..0]
    Gray8 = SkColorType::kGray_8_SkColorType as _,
    /// Four channel RGBA data (16-bit half-float per channel) packed into a LE 64-bit word. Values
    /// are assumed to be in [0.0,1.0] range, unlike RGBAF16.
    /// Bits: [A:63..48 B:47..32 G:31..16 R:15..0]
    RGBAF16Norm = SkColorType::kRGBA_F16Norm_SkColorType as _,
    /// Four channel RGBA data (16-bit half-float per channel) packed into a LE 64-bit word.
    /// This has extended range compared to RGBAF16Norm.
    /// Bits: [A:63..48 B:47..32 G:31..16 R:15..0]
    RGBAF16 = SkColorType::kRGBA_F16_SkColorType as _,
    /// Three channel RGB data (16-bit half-float per channel) packed into a LE 64-bit word. The last
    /// 16 bits are ignored and alpha is forced to opaque.
    /// Bits: [x:63..48 B:47..32 G:31..16 R:15..0]
    RGBF16F16F16x = SkColorType::kRGB_F16F16F16x_SkColorType as _,
    /// Four channel RGBA data (32-bit float per channel) packed into a LE 128-bit word.
    /// Bits: [A:127..96 B:95..64 G:63..32 R:31..0]
    RGBAF32 = SkColorType::kRGBA_F32_SkColorType as _,

    /// Two channel RG data (8 bits per channel). Blue is forced to 0, alpha is forced to opaque.
    /// Bits: [G:15..8 R:7..0]
    R8G8UNorm = SkColorType::kR8G8_unorm_SkColorType as _,
    /// Single channel data (16-bit half-float) interpreted as alpha. RGB are 0.
    /// Bits: [A:15..0]
    A16Float = SkColorType::kA16_float_SkColorType as _,
    /// Single channel data (16 bits half-float) interpreted as red. G and B are forced to 0, alpha
    /// is forced to opaque.
    /// Bits: [R:15..0]
    R16Float = SkColorType::kR16_float_SkColorType as _,
    /// Two channel RG data (16-bit half-float per channel) packed into a LE 32-bit word.
    /// Blue is forced to 0, alpha is forced to opaque.
    /// Bits: [G:31..16 R:15..0]
    R16G16Float = SkColorType::kR16G16_float_SkColorType as _,
    /// Single channel data (16 bits) interpreted as alpha. RGB are 0.
    /// Bits: [A:15..0]
    A16UNorm = SkColorType::kA16_unorm_SkColorType as _,
    // Single channel data (16 bits) interpreted as red. G and B are forced to 0, alpha is forced to
    // opaque.
    //   Bits: [R:15..0]
    R16UNorm = SkColorType::kR16_unorm_SkColorType as _,
    /// Two channel RG data (16 bits per channel) packed into a LE 32-bit word. B is forced to 0,
    /// alpha is forced to opaque.
    /// Bits: [G:31..16 R:15..0]
    R16G16UNorm = SkColorType::kR16G16_unorm_SkColorType as _,
    /// Four channel RGBA data (16 bits per channel) packed into a LE 64-bit word.
    /// Bits: [A:63..48 B:47..32 G:31..16 R:15..0]
    R16G16B16A16UNorm = SkColorType::kR16G16B16A16_unorm_SkColorType as _,
    /// Four channel RGBA data (8 bits per channel) packed into a LE 32-bit word. The RGB values are
    /// assumed to be encoded with the sRGB transfer function.
    /// Bits: [A:31..24 B:23..16 G:15..8 R:7..0]
    SRGBA8888 = SkColorType::kSRGBA_8888_SkColorType as _,
    /// Single channel data (8 bits) interpreted as red. G and B are forced to 0, alpha is forced to
    /// opaque.
    /// Bits: [R:7..0]
    R8UNorm = SkColorType::kR8_unorm_SkColorType as _,
}

native_transmutable!(SkColorType, ColorType);

impl ColorType {
    #[deprecated(since = "0.51.0", note = "Use ColorType::N32 ")]
    pub const fn n32() -> Self {
        Self::N32
    }

    pub const N32: Self = unsafe { *((&SkColorType::kN32_SkColorType) as *const _ as *const _) };

    pub const COUNT: usize =
        unsafe { *((&SkColorType::kLastEnum_SkColorType) as *const _ as *const _) } as usize
            + 1usize;

    pub fn bytes_per_pixel(self) -> usize {
        unsafe {
            sb::SkColorTypeBytesPerPixel(self.into_native())
                .try_into()
                .unwrap()
        }
    }

    pub fn is_always_opaque(self) -> bool {
        unsafe { sb::SkColorTypeIsAlwaysOpaque(self.into_native()) }
    }

    pub fn validate_alpha_type(self, alpha_type: AlphaType) -> Option<AlphaType> {
        let mut alpha_type_r = AlphaType::Unknown;
        unsafe {
            sb::SkColorTypeValidateAlphaType(self.into_native(), alpha_type, &mut alpha_type_r)
        }
        .then_some(alpha_type_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n32_matches() {
        assert_eq!(
            ColorType::from_native_c(skia_bindings::SkColorType::kN32_SkColorType),
            ColorType::N32
        );
    }
}
