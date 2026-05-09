use skia_bindings as sb;

pub fn swap_rb(dest: &mut [u32], src: &[u32]) {
    assert_eq!(dest.len(), src.len());
    unsafe {
        sb::SkSwapRB(
            dest.as_mut_ptr(),
            src.as_ptr(),
            dest.len().try_into().unwrap(),
        )
    }
}

pub fn swap_rb_inplace(pixels: &mut [u32]) {
    unsafe {
        sb::SkSwapRB(
            pixels.as_mut_ptr(),
            pixels.as_ptr(),
            pixels.len().try_into().unwrap(),
        )
    }
}
