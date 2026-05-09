// Unsupported, not used in any public APIs.
// mod executor;
// Unsupported, because it's not used in publicly exposed APIs:
// mod font_lcd_config;

mod alpha_type;
mod annotation;
pub mod arc;
mod bbh_factory;
mod bitmap;
mod blend_mode;
mod blender;
mod blur_types;
pub mod canvas;
mod clip_op;
mod color;
pub mod color_filter;
mod color_space;
mod color_table;
mod color_type;
pub mod contour_measure;
mod coverage_mode;
mod cpu_recorder;
mod cubic_map;
mod data;
mod data_table;
pub mod document;
pub mod drawable;
mod flattenable;
pub mod font;
pub mod font_arguments;
pub mod font_metrics;
mod font_mgr;
pub mod font_parameters;
mod font_scanner;
pub mod font_style;
mod font_types;
mod four_byte_tag;
pub mod graphics;
pub mod image;
pub mod image_filter;
mod image_generator;
mod image_info;
mod m44;
mod mask_filter;
pub mod matrix;
mod mesh;
mod milestone;
pub mod paint;
pub mod path;
pub mod path_builder;
pub mod path_effect;
mod path_iter;
pub mod path_measure;
pub mod path_types;
pub mod path_utils;
mod picture;
pub mod picture_recorder;
mod pixel_ref;
mod pixmap;
mod point;
mod point3;
mod raster_handle_allocator;
pub mod recorder;
mod rect;
pub mod region;
pub mod rrect;
mod rsxform;
pub mod sampling_options;
mod scalar_;
pub mod shader;
mod size;
pub mod stroke_rec;
pub mod surface;
mod surface_props;
mod swizzle;
mod text_blob;
mod texture_compression_type;
mod tile_mode;
pub mod tiled_image_utils;
mod time;
mod trace_memory_dump;
pub mod typeface;
mod types;
mod un_pre_multiply;
pub mod vertices;
pub mod yuva_info;
pub mod yuva_pixmaps;

pub use alpha_type::*;
pub use annotation::annotate;
pub use bbh_factory::*;
pub use bitmap::*;
pub use blend_mode::*;
pub use blender::*;
pub use blur_types::*;
pub use canvas::{AutoCanvasRestore, Canvas, OwnedCanvas};
pub use clip_op::*;
pub use color::*;
pub use color_filter::{color_filters, ColorFilter};
pub use color_space::*;
pub use color_table::*;
pub use color_type::*;
pub use contour_measure::{ContourMeasure, ContourMeasureIter};
pub use coverage_mode::*;
pub use cubic_map::*;
pub use data::*;
pub use data_table::*;
pub use document::Document;
pub use drawable::Drawable;
pub use flattenable::*;
pub use font::Font;
pub use font_arguments::FontArguments;
pub use font_metrics::FontMetrics;
pub use font_mgr::*;
pub use font_style::FontStyle;
pub use font_types::*;
pub use four_byte_tag::*;
pub use image::{images, Image};
pub use image_filter::ImageFilter;
pub use image_generator::*;
pub use image_info::*;
pub use m44::*;
pub use mask_filter::*;
pub use matrix::Matrix;
pub use milestone::*;
pub use paint::Paint;
pub use tile_mode::*;
// We keep these around for the time being.
pub use arc::Arc;
pub use cpu_recorder::cpu;
pub use paint::Cap as PaintCap;
pub use paint::Join as PaintJoin;
pub use paint::Style as PaintStyle;
pub use path::Path;
pub use path_builder::PathBuilder;
pub use path_effect::PathEffect;
pub use path_iter::*;
pub use path_measure::PathMeasure;
pub use path_types::*;
pub use picture::*;
pub use picture_recorder::PictureRecorder;
pub use pixel_ref::*;
pub use pixmap::*;
pub use point::*;
pub use point3::*;
#[allow(unused)]
pub use raster_handle_allocator::*;
pub use recorder::{Recorder, RecorderRef};
pub use rect::*;
pub use region::Region;
pub use rrect::RRect;
pub use rsxform::*;
#[allow(deprecated)]
pub use sampling_options::{
    CubicResampler, FilterMode, FilterOptions, MipmapMode, SamplingMode, SamplingOptions,
};
pub use scalar_::*;
pub use shader::Shader;
pub use size::*;
pub use stroke_rec::StrokeRec;
pub use surface::{surfaces, Surface};
pub use surface_props::*;
pub use swizzle::*;
pub use text_blob::*;
pub use texture_compression_type::*;
#[allow(unused)]
pub use time::*;
#[allow(unused)]
pub use trace_memory_dump::*;
pub use typeface::Typeface;
pub use types::*;
#[allow(unused)]
pub use un_pre_multiply::*;
pub use vertices::Vertices;
pub use yuva_info::YUVAInfo;
pub use yuva_pixmaps::{yuva_pixmap_info, YUVAPixmapInfo, YUVAPixmaps};
//
// Skia specific traits used for overloading functions.
//

pub trait Contains<T> {
    fn contains(&self, other: T) -> bool;
}

pub trait QuickReject<T> {
    fn quick_reject(&self, other: &T) -> bool;
}

pub mod shaders {
    pub use super::shader::shaders::*;
    use crate::{prelude::*, scalar, ISize, Shader};
    use skia_bindings as sb;

    impl Shader {
        pub fn fractal_perlin_noise(
            base_frequency: (scalar, scalar),
            num_octaves: usize,
            seed: scalar,
            tile_size: impl Into<Option<ISize>>,
        ) -> Option<Self> {
            fractal_noise(base_frequency, num_octaves, seed, tile_size)
        }

        pub fn turbulence_perlin_noise(
            base_frequency: (scalar, scalar),
            num_octaves: usize,
            seed: scalar,
            tile_size: impl Into<Option<ISize>>,
        ) -> Option<Self> {
            turbulence(base_frequency, num_octaves, seed, tile_size)
        }
    }

    pub fn fractal_noise(
        base_frequency: (scalar, scalar),
        num_octaves: usize,
        seed: scalar,
        tile_size: impl Into<Option<ISize>>,
    ) -> Option<Shader> {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_MakeFractalNoise(
                base_frequency.0,
                base_frequency.1,
                num_octaves.try_into().unwrap(),
                seed,
                tile_size.into().native().as_ptr_or_null(),
            )
        })
    }

    pub fn turbulence(
        base_frequency: (scalar, scalar),
        num_octaves: usize,
        seed: scalar,
        tile_size: impl Into<Option<ISize>>,
    ) -> Option<Shader> {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_MakeTurbulence(
                base_frequency.0,
                base_frequency.1,
                num_octaves.try_into().unwrap(),
                seed,
                tile_size.into().native().as_ptr_or_null(),
            )
        })
    }
}
