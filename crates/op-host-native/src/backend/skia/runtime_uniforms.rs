//! Pack float uniforms using Skia's reflected offsets, including arrays.

use skia_safe::runtime_effect::uniform::Type;
use skia_safe::{Data, RuntimeEffect};

pub(super) fn pack(effect: &RuntimeEffect, values: &[(&str, &[f32])]) -> Data {
    let mut bytes = vec![0; effect.uniform_size()];
    for (name, components) in values {
        let Some(uniform) = effect
            .uniforms()
            .iter()
            .find(|uniform| uniform.name() == *name)
        else {
            continue;
        };
        if !matches!(
            uniform.ty(),
            Type::Float
                | Type::Float2
                | Type::Float3
                | Type::Float4
                | Type::Float2x2
                | Type::Float3x3
                | Type::Float4x4
        ) || components.len().checked_mul(4) != Some(uniform.size_in_bytes())
            || components.iter().any(|value| !value.is_finite())
        {
            continue;
        }
        let start = uniform.offset();
        let Some(end) = start.checked_add(uniform.size_in_bytes()) else {
            continue;
        };
        let Some(destination) = bytes.get_mut(start..end) else {
            continue;
        };
        for (destination, component) in destination.chunks_exact_mut(4).zip(*components) {
            destination.copy_from_slice(&component.to_ne_bytes());
        }
    }
    Data::new_copy(&bytes)
}
