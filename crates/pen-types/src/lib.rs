//! OpenPencil core types.
//!
//! Per kickoff spec §1.2 bucket A: must compile on wasm32-unknown-unknown.

#[derive(Debug, thiserror::Error)]
pub enum PenError {
    #[error("placeholder error")]
    Placeholder,
}

pub fn placeholder() -> &'static str {
    "pen-types skeleton"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_works() {
        assert_eq!(placeholder(), "pen-types skeleton");
    }
}
