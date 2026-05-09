#[cfg(windows)]
pub fn init() {
    use std::env;
    static icudtl: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/skia/icudtl.dat"));

    #[cfg(feature = "embed-icudtl")]
    {
        use std::sync::Mutex;

        static MUTEX: Mutex<()> = Mutex::new(());

        // Using `Once` does not work for yet unknown reasons.
        // https://github.com/rust-skia/rust-skia/issues/566

        let lock = MUTEX.lock().unwrap();
        unsafe { crate::C_SetICU(&icudtl[0] as &'static u8 as *const u8 as _) };
        drop(lock);
    }

    #[cfg(not(feature = "embed-icudtl"))]
    {
        use std::fs;

        let path = env::current_exe()
            .expect("Failed to resolve the current executable's path")
            .parent()
            .expect("Current executable's parent path does not point to a directory")
            .join("icudtl.dat");
        if path.exists() {
            return;
        };
        fs::write(path, &icudtl[..])
            .expect("Failed to write icudtl.dat into the current executable's directory");
    }
}

#[cfg(not(windows))]
pub fn init() {}
