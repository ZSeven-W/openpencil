use std::{cell::UnsafeCell, fmt};

use crate::prelude::*;
use skia_bindings::{self as sb, SkRecorder};

pub type Type = skia_bindings::SkRecorder_Type;
variant_name!(Type::CPU);

pub trait Recorder: sealed::AsRecorderRef + fmt::Debug {
    fn ty(&self) -> Type;
    // TODO:
    // fn cpu_recorder(&mut self) -> &mut cpu::Recorder;
}

pub(crate) mod sealed {
    pub trait AsRecorderRef {
        fn as_recorder_ref(&mut self) -> &mut super::RecorderRef;
    }
}

#[repr(transparent)]
pub struct RecorderRef(UnsafeCell<SkRecorder>);

impl NativeAccess for RecorderRef {
    type Native = SkRecorder;

    fn native(&self) -> &SkRecorder {
        unsafe { &*self.0.get() }
    }

    fn native_mut(&mut self) -> &mut SkRecorder {
        unsafe { &mut (*self.0.get()) }
    }
}

impl fmt::Debug for RecorderRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recorder")
            .field("type", &self.ty())
            .finish()
    }
}

impl RecorderRef {
    #[allow(unused)] // ... in non-gpu builds.
    pub(crate) fn from_ref_mut(native: &mut SkRecorder) -> &mut Self {
        unsafe { transmute_ref_mut(native) }
    }
}

impl Recorder for RecorderRef {
    fn ty(&self) -> Type {
        let mut ty = Type::CPU;
        unsafe {
            sb::C_SkRecorder_type(self.native(), &mut ty);
        }
        ty
    }
}

impl sealed::AsRecorderRef for RecorderRef {
    fn as_recorder_ref(&mut self) -> &mut RecorderRef {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{Recorder, RecorderRef};

    #[allow(deref_nullptr)]
    fn _passing_the_different_kinds_of_recorder_compiles() {
        test(None);
        test(Some(owned_recorder()));
        test(Some(cpu_recorder()));

        fn test(_recorder: Option<&mut dyn Recorder>) {}

        fn owned_recorder() -> &'static mut RecorderRef {
            unsafe { &mut *std::ptr::null_mut() }
        }

        fn cpu_recorder() -> &'static mut crate::cpu::Recorder<'static> {
            unsafe { &mut *std::ptr::null_mut() }
        }
    }
}
