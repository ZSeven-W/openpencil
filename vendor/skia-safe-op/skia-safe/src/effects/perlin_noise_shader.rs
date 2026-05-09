use crate::{scalar, shaders, ISize, Shader};

pub fn fractal_noise(
    base_frequency: (scalar, scalar),
    num_octaves: usize,
    seed: scalar,
    tile_size: impl Into<Option<ISize>>,
) -> Option<Shader> {
    shaders::fractal_noise(base_frequency, num_octaves, seed, tile_size)
}

pub fn turbulence(
    base_frequency: (scalar, scalar),
    num_octaves: usize,
    seed: scalar,
    tile_size: impl Into<Option<ISize>>,
) -> Option<Shader> {
    shaders::turbulence(base_frequency, num_octaves, seed, tile_size)
}
