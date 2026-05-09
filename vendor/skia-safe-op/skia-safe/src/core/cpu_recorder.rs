pub mod cpu {
    use std::fmt;

    use crate::{prelude::*, recorder, Recorder as _};
    use skia_bindings::{self as sb, skcpu_Recorder};

    #[repr(transparent)]
    pub struct Recorder<'a>(&'a mut skcpu_Recorder);
    require_base_type!(skcpu_Recorder, sb::SkRecorder);

    impl NativeAccess for Recorder<'_> {
        type Native = skcpu_Recorder;

        fn native(&self) -> &Self::Native {
            self.0
        }

        fn native_mut(&mut self) -> &mut Self::Native {
            self.0
        }
    }

    impl fmt::Debug for Recorder<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Recorder").field("ty", &self.ty()).finish()
        }
    }

    impl Drop for Recorder<'_> {
        fn drop(&mut self) {
            unsafe {
                sb::C_SkRecorder_delete(&mut self.native_mut()._base);
            }
        }
    }

    impl<'a> Recorder<'a> {
        #[allow(unused)]
        pub(crate) fn from_owned(recorder: &'a mut skcpu_Recorder) -> Self {
            Self(recorder)
        }

        // TODO: Wrap `makeBitmapInfo` if lifetimes can be clarified.
    }

    impl recorder::Recorder for Recorder<'_> {
        fn ty(&self) -> crate::recorder::Type {
            recorder::Type::CPU
        }
    }

    impl recorder::sealed::AsRecorderRef for Recorder<'_> {
        fn as_recorder_ref(&mut self) -> &mut recorder::RecorderRef {
            recorder::RecorderRef::from_ref_mut(&mut self.native_mut()._base)
        }
    }
}
