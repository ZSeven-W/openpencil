use crate::{scalar, Matrix, Path, PathEffect};

impl PathEffect {
    pub fn line_2d(width: scalar, matrix: &Matrix) -> Option<PathEffect> {
        line_2d_path_effect::new(width, matrix)
    }

    pub fn path_2d(matrix: &Matrix, path: &Path) -> PathEffect {
        path_2d_path_effect::new(matrix, path)
    }
}

pub mod line_2d_path_effect {
    use crate::{prelude::*, scalar, Matrix, PathEffect};
    use skia_bindings as sb;

    pub fn new(width: scalar, matrix: &Matrix) -> Option<PathEffect> {
        PathEffect::from_ptr(unsafe { sb::C_SkLine2DPathEffect_Make(width, matrix.native()) })
    }
}

pub mod path_2d_path_effect {
    use crate::{prelude::*, Matrix, Path, PathEffect};
    use skia_bindings as sb;

    pub fn new(matrix: &Matrix, path: &Path) -> PathEffect {
        PathEffect::from_ptr(unsafe {
            sb::C_SkPath2DPathEffect_Make(matrix.native(), path.native())
        })
        .unwrap()
    }
}
