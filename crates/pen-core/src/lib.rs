//! OpenPencil core engine.

pub fn placeholder() -> String {
    format!("pen-core ({})", pen_types::placeholder())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_links_types() {
        assert!(placeholder().contains("pen-types"));
    }
}
