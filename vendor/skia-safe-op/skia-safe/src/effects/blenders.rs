use crate::Blender;
use skia_bindings as sb;

impl Blender {
    pub fn arithmetic(k1: f32, k2: f32, k3: f32, k4: f32, enforce_premul: bool) -> Option<Blender> {
        arithmetic(k1, k2, k3, k4, enforce_premul)
    }
}

/// Create a blender that implements the following:
/// `k1 * src * dst + k2 * src + k3 * dst + k4`
///
/// - `k1`, `k2`, `k3`, `k4` The four coefficients.
/// - `enforce_premul` If `true`, the RGB channels will be clamped to the calculated alpha.
pub fn arithmetic(k1: f32, k2: f32, k3: f32, k4: f32, enforce_premul: bool) -> Option<Blender> {
    Blender::from_ptr(unsafe { sb::C_SkBlenders_Arithmetic(k1, k2, k3, k4, enforce_premul) })
}
