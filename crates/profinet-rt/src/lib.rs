//! `profinet-rt` — pile IO-Device PROFINET RT en Rust pur.
//!
//! Projet communautaire, NON affilié à / approuvé par PROFIBUS & PROFINET
//! International. « PROFINET » est une marque déposée de PNO.

pub mod capture;
pub mod eth;

/// Version de la crate (smoke test des fondations).
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
