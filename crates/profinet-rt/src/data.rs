//! Encodage/décodage des types process PROFINET.
//!
//! Tous les types multi-octets sont en big-endian (« format Motorola »),
//! identique à la représentation mémoire Siemens : aucun word-swap nécessaire.
//! `REAL` est de l'IEEE-754 32 bits. `BOOL` est packé 8 bits par octet, LSB-first
//! (le bit `octet.0` est le bit de poids faible), convention d'adressage Siemens.

use thiserror::Error;

/// Les 5 types process supportés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Bool,
    Int,
    Word,
    Dint,
    Real,
}

impl FieldType {
    /// Taille en octets pour les types alignés-octet ; `None` pour `Bool` (bit-packé).
    pub fn byte_len(self) -> Option<usize> {
        match self {
            FieldType::Bool => None,
            FieldType::Int | FieldType::Word => Some(2),
            FieldType::Dint | FieldType::Real => Some(4),
        }
    }
}

/// Valeur process typée.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i16),
    Word(u16),
    Dint(i32),
    Real(f32),
}

/// Erreurs d'encodage/décodage.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("buffer too short: need {need}, have {have}")]
    TooShort { need: usize, have: usize },
    #[error("bit index {bit} out of range for {bytes}-byte buffer")]
    BitOutOfRange { bit: usize, bytes: usize },
}

pub fn encode_i16(v: i16) -> [u8; 2] {
    v.to_be_bytes()
}
pub fn encode_u16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}
pub fn encode_i32(v: i32) -> [u8; 4] {
    v.to_be_bytes()
}
pub fn encode_f32(v: f32) -> [u8; 4] {
    v.to_be_bytes()
}

pub fn decode_i16(b: &[u8]) -> Result<i16, CodecError> {
    let a = take::<2>(b)?;
    Ok(i16::from_be_bytes(a))
}
pub fn decode_u16(b: &[u8]) -> Result<u16, CodecError> {
    let a = take::<2>(b)?;
    Ok(u16::from_be_bytes(a))
}
pub fn decode_i32(b: &[u8]) -> Result<i32, CodecError> {
    let a = take::<4>(b)?;
    Ok(i32::from_be_bytes(a))
}
pub fn decode_f32(b: &[u8]) -> Result<f32, CodecError> {
    let a = take::<4>(b)?;
    Ok(f32::from_be_bytes(a))
}

/// Copie les `N` premiers octets de `b`, ou renvoie `TooShort`.
fn take<const N: usize>(b: &[u8]) -> Result<[u8; N], CodecError> {
    if b.len() < N {
        return Err(CodecError::TooShort {
            need: N,
            have: b.len(),
        });
    }
    let mut a = [0u8; N];
    a.copy_from_slice(&b[..N]);
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors_big_endian() {
        assert_eq!(encode_i16(-1), [0xFF, 0xFF]);
        assert_eq!(encode_u16(0x0102), [0x01, 0x02]);
        assert_eq!(encode_i32(-1), [0xFF, 0xFF, 0xFF, 0xFF]);
        // IEEE-754 : 1.0_f32 = 0x3F800000 ; -2.0_f32 = 0xC0000000
        assert_eq!(encode_f32(1.0), [0x3F, 0x80, 0x00, 0x00]);
        assert_eq!(encode_f32(-2.0), [0xC0, 0x00, 0x00, 0x00]);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn scalar_round_trips() {
        for v in [i16::MIN, -1, 0, 1, 1234, i16::MAX] {
            assert_eq!(decode_i16(&encode_i16(v)).unwrap(), v);
        }
        for v in [0u16, 1, 0x00FF, 0xFF00, u16::MAX] {
            assert_eq!(decode_u16(&encode_u16(v)).unwrap(), v);
        }
        for v in [i32::MIN, -1, 0, 1, 70000, i32::MAX] {
            assert_eq!(decode_i32(&encode_i32(v)).unwrap(), v);
        }
        for v in [f32::MIN, -2.0, -0.5, 0.0, 0.5, 3.14159, f32::MAX] {
            assert_eq!(decode_f32(&encode_f32(v)).unwrap(), v);
        }
    }

    #[test]
    fn decode_too_short_errors() {
        assert_eq!(
            decode_i16(&[0x00]),
            Err(CodecError::TooShort { need: 2, have: 1 })
        );
        assert_eq!(
            decode_i32(&[0x00, 0x00]),
            Err(CodecError::TooShort { need: 4, have: 2 })
        );
        assert_eq!(
            decode_f32(&[]),
            Err(CodecError::TooShort { need: 4, have: 0 })
        );
    }

    #[test]
    fn decode_ignores_extra_bytes() {
        // un buffer plus long que nécessaire décode les premiers octets
        assert_eq!(decode_u16(&[0x01, 0x02, 0x03]).unwrap(), 0x0102);
    }

    #[test]
    fn field_type_byte_len() {
        assert_eq!(FieldType::Bool.byte_len(), None);
        assert_eq!(FieldType::Int.byte_len(), Some(2));
        assert_eq!(FieldType::Word.byte_len(), Some(2));
        assert_eq!(FieldType::Dint.byte_len(), Some(4));
        assert_eq!(FieldType::Real.byte_len(), Some(4));
    }
}
