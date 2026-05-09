pub use skia_bindings::GrDriverBugWorkarounds as DriverBugWorkarounds;
#[test]
fn test_driver_bug_workarounds_naming() {
    fn _n(workarounds: &DriverBugWorkarounds) {
        let _ = workarounds.max_fragment_uniform_vectors_32;
    }
}

pub trait ApplyOverrides {
    fn apply_overrides(&mut self, workarounds: &DriverBugWorkarounds);
}

impl ApplyOverrides for DriverBugWorkarounds {
    fn apply_overrides(&mut self, workarounds: &DriverBugWorkarounds) {
        unsafe { self.applyOverrides(workarounds) }
    }
}
