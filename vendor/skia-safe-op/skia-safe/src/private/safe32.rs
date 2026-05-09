pub mod sk64 {
    const SK_MAX_S32: i32 = i32::MAX;
    const SK_MIN_S32: i32 = -SK_MAX_S32;

    pub fn pin_to_s32(x: i64) -> i32 {
        if x < i64::from(SK_MIN_S32) {
            return SK_MIN_S32;
        }
        if x > i64::from(SK_MAX_S32) {
            return SK_MAX_S32;
        }
        x as i32
    }
}

pub mod sk32 {
    use super::sk64;

    pub fn sat_add(a: i32, b: i32) -> i32 {
        sk64::pin_to_s32(i64::from(a) + i64::from(b))
    }

    pub fn sat_sub(a: i32, b: i32) -> i32 {
        sk64::pin_to_s32(i64::from(a) - i64::from(b))
    }

    // The original Skia implementations were created
    // to circumvent an LLVM sanitizer check, but do cause
    // a "subtract with overflow" for simple cases (see testcase below),
    // so we keep the naive implementation for now.
    // Ref:
    // https://skia-review.googlesource.com/c/skia/+/90544
    // https://skia-review.googlesource.com/c/skia/+/101881
    #[allow(dead_code)]
    pub const fn can_overflow_add(a: i32, b: i32) -> i32 {
        // ((a as u32) + (b as u32)) as i32
        a + b
    }

    pub const fn can_overflow_sub(a: i32, b: i32) -> i32 {
        // ((a as u32) - (b as u32)) as i32
        a - b
    }

    #[test]
    fn subtraction_with_negative_does_not_overflow() {
        can_overflow_sub(111, -257);
    }
}
