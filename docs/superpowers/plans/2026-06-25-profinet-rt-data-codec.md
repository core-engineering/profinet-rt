# PROFINET RT — Plan « data » : codec des types process (BOOL/INT/DINT/REAL/WORD)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implémenter la couche d'encodage/décodage des 5 types process PROFINET en big-endian / IEEE-754, comme source unique de vérité réutilisée par les couches RT (Plan 4) et config/GSDML (Plan 6).

**Architecture:** Un module `data` pur, sans I/O ni dépendance réseau. Deux familles de primitives : (1) codecs scalaires big-endian pour `INT` (i16), `WORD` (u16), `DINT` (i32), `REAL` (f32) ; (2) accès bit pour `BOOL` (packé 8 bits/octet, LSB-first façon adressage Siemens `x.0`). Un type `FieldType` décrit les types ; un type `Value` porte une valeur typée. Tout est testable en isolation (round-trips + vecteurs connus).

**Tech Stack:** Rust 2021, `thiserror` (déjà dépendance). Aucune nouvelle dépendance.

## Global Constraints

- **Big-endian (« format Motorola »)** pour tous les types multi-octets — identique à la mémoire Siemens, aucun word-swap.
- **REAL = IEEE-754 32 bits** big-endian.
- **BOOL packé** 8 bits/octet, **LSB-first** : le bit d'indice `i` est à l'octet `i/8`, masque `1 << (i % 8)` (convention adressage Siemens où `octet.0` est le bit de poids faible). Ce choix est documenté dans le code et à **revalider contre une capture/GSDML** (consigné dans `FOLLOWUPS.md`).
- Rust 100 % natif ; pas de code GPL/p-net ; pas de texte de norme IEC en commentaire.
- Pas d'`unwrap`/`panic` sur les chemins de décodage (entrées potentiellement trop courtes) — renvoyer une erreur typée.

**Rappel environnement :** Rust est installé via rustup mais PAS sur le PATH par défaut. Préfixer chaque commande cargo par `. "$HOME/.cargo/env" && …` dans la même ligne de shell.

---

### Task 1 : Codecs scalaires big-endian + types `FieldType`/`Value` + `CodecError`

**Files:**
- Create: `crates/profinet-rt/src/data.rs`
- Modify: `crates/profinet-rt/src/lib.rs` (ajout `pub mod data;`)

**Interfaces:**
- Consumes: (rien)
- Produces:
  - `pub enum FieldType { Bool, Int, Word, Dint, Real }` avec `pub fn byte_len(self) -> Option<usize>` (None pour `Bool`, bit-packé ; 2 pour Int/Word ; 4 pour Dint/Real)
  - `pub enum Value { Bool(bool), Int(i16), Word(u16), Dint(i32), Real(f32) }`
  - `pub fn encode_i16(i16) -> [u8; 2]`, `pub fn encode_u16(u16) -> [u8; 2]`, `pub fn encode_i32(i32) -> [u8; 4]`, `pub fn encode_f32(f32) -> [u8; 4]`
  - `pub fn decode_i16(&[u8]) -> Result<i16, CodecError>`, idem `decode_u16`, `decode_i32`, `decode_f32`
  - `pub enum CodecError { TooShort { need: usize, have: usize }, BitOutOfRange { bit: usize, bytes: usize } }` (via `thiserror`)

- [ ] **Step 1: Écrire les tests (vecteurs connus + round-trips + trop court)**

Dans `crates/profinet-rt/src/data.rs` :

```rust
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
        assert_eq!(decode_i16(&[0x00]), Err(CodecError::TooShort { need: 2, have: 1 }));
        assert_eq!(decode_i32(&[0x00, 0x00]), Err(CodecError::TooShort { need: 4, have: 2 }));
        assert_eq!(decode_f32(&[]), Err(CodecError::TooShort { need: 4, have: 0 }));
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
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `. "$HOME/.cargo/env" && cargo test -p profinet-rt data:: -v`
Expected: FAIL (compilation : symboles non définis).

- [ ] **Step 3: Implémenter le module**

En tête de `crates/profinet-rt/src/data.rs` :

```rust
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
        return Err(CodecError::TooShort { need: N, have: b.len() });
    }
    let mut a = [0u8; N];
    a.copy_from_slice(&b[..N]);
    Ok(a)
}
```

Ajouter dans `crates/profinet-rt/src/lib.rs` : `pub mod data;`

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `. "$HOME/.cargo/env" && cargo test -p profinet-rt data:: -v`
Expected: PASS (5 tests).

- [ ] **Step 5: Vérifier fmt + clippy puis commit**

Run: `. "$HOME/.cargo/env" && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings`
Expected: aucune erreur.

```bash
git add -A
git commit -m "feat(data): codecs scalaires big-endian + FieldType/Value/CodecError"
```

---

### Task 2 : Accès bit pour `BOOL` (packé, LSB-first)

**Files:**
- Modify: `crates/profinet-rt/src/data.rs` (ajout des fonctions bit + tests)

**Interfaces:**
- Consumes: `CodecError` (Task 1)
- Produces:
  - `pub fn get_bit(buf: &[u8], bit: usize) -> Result<bool, CodecError>`
  - `pub fn set_bit(buf: &mut [u8], bit: usize, value: bool) -> Result<(), CodecError>`

- [ ] **Step 1: Écrire les tests (limites d'octet, hors-plage, set puis get)**

Ajouter dans le `mod tests` de `crates/profinet-rt/src/data.rs` :

```rust
    #[test]
    fn set_and_get_bits_lsb_first() {
        let mut buf = [0u8; 4]; // 32 bits

        // bit 0 = LSB de l'octet 0
        set_bit(&mut buf, 0, true).unwrap();
        assert_eq!(buf[0], 0b0000_0001);
        // bit 7 = MSB de l'octet 0
        set_bit(&mut buf, 7, true).unwrap();
        assert_eq!(buf[0], 0b1000_0001);
        // bit 8 = LSB de l'octet 1
        set_bit(&mut buf, 8, true).unwrap();
        assert_eq!(buf[1], 0b0000_0001);
        // bit 31 = MSB de l'octet 3
        set_bit(&mut buf, 31, true).unwrap();
        assert_eq!(buf[3], 0b1000_0000);

        for &(i, expected) in &[(0, true), (1, false), (7, true), (8, true), (31, true)] {
            assert_eq!(get_bit(&buf, i).unwrap(), expected, "bit {i}");
        }
    }

    #[test]
    fn clearing_a_bit() {
        let mut buf = [0xFFu8; 1];
        set_bit(&mut buf, 3, false).unwrap();
        assert_eq!(buf[0], 0b1111_0111);
        assert_eq!(get_bit(&buf, 3).unwrap(), false);
    }

    #[test]
    fn bit_out_of_range_errors() {
        let mut buf = [0u8; 1]; // 8 bits valides : 0..=7
        assert_eq!(get_bit(&buf, 8), Err(CodecError::BitOutOfRange { bit: 8, bytes: 1 }));
        assert_eq!(
            set_bit(&mut buf, 8, true),
            Err(CodecError::BitOutOfRange { bit: 8, bytes: 1 })
        );
    }
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `. "$HOME/.cargo/env" && cargo test -p profinet-rt data:: -v`
Expected: FAIL (compilation : `get_bit`/`set_bit` non définis).

- [ ] **Step 3: Implémenter les accès bit**

Ajouter dans `crates/profinet-rt/src/data.rs` (hors du `mod tests`) :

```rust
/// Lit le bit d'indice `bit` (LSB-first : octet `bit/8`, masque `1 << (bit % 8)`).
pub fn get_bit(buf: &[u8], bit: usize) -> Result<bool, CodecError> {
    let byte = bit / 8;
    if byte >= buf.len() {
        return Err(CodecError::BitOutOfRange { bit, bytes: buf.len() });
    }
    Ok((buf[byte] >> (bit % 8)) & 1 == 1)
}

/// Écrit le bit d'indice `bit` (même convention que `get_bit`).
pub fn set_bit(buf: &mut [u8], bit: usize, value: bool) -> Result<(), CodecError> {
    let byte = bit / 8;
    if byte >= buf.len() {
        return Err(CodecError::BitOutOfRange { bit, bytes: buf.len() });
    }
    let mask = 1u8 << (bit % 8);
    if value {
        buf[byte] |= mask;
    } else {
        buf[byte] &= !mask;
    }
    Ok(())
}
```

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `. "$HOME/.cargo/env" && cargo test -p profinet-rt data:: -v`
Expected: PASS (8 tests au total dans le module `data`).

- [ ] **Step 5: Vérifier fmt + clippy puis commit**

Run: `. "$HOME/.cargo/env" && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings`
Expected: aucune erreur.

```bash
git add -A
git commit -m "feat(data): acces bit BOOL packe (LSB-first)"
```

---

## Self-review (rempli après écriture)

- **Couverture spec** : couvre §5.3 (mapping des types) de la spec + la brique « encodage typé » de la roadmap (Plan 6). Le layout slots/sous-modules et le GSDML restent au Plan 6 ; ici seulement les primitives d'encodage. ✅
- **Placeholders** : aucun TODO/TBD ; tout le code est fourni. ✅
- **Cohérence des types** : `CodecError` défini en Task 1 et réutilisé en Task 2 ; `FieldType::byte_len -> Option<usize>` cohérent (None pour Bool). ✅
- **Suivi** : la convention de bit-order LSB-first est un choix à valider contre une capture/GSDML → à ajouter dans `FOLLOWUPS.md`.
