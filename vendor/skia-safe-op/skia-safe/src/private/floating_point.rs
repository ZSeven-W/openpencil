// SkFloatingPoint.h

use crate::scalar;

#[allow(clippy::eq_op)]
pub fn is_finite(array: &[scalar]) -> bool {
    let mut prod = 0.0;
    for x in array {
        prod *= x;
    }
    // At this point, `prod` will either be NaN or 0.
    prod == prod
}

#[cfg(test)]
mod tests {
    #[test]
    fn is_finite() {
        assert!(!super::is_finite(&[0.0, f32::NAN]));
        assert!(!super::is_finite(&[0.0, f32::INFINITY]));
        assert!(super::is_finite(&[0.0]));
    }
}
