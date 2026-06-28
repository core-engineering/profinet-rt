//! `profinet-rt` — pure-Rust PROFINET RT IO-Device stack.
//!
//! Community project, NOT affiliated with or endorsed by PROFIBUS & PROFINET
//! International. "PROFINET" is a registered trademark of PNO.

pub mod capture;
pub mod data;
pub mod dcp;
pub mod eth;

/// Crate version (foundations smoke test).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!version().is_empty());
    }
}
