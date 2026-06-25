# PROFINET RT — Plan 1 : Fondations (scaffold + couche `eth` + harnais golden-frames)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poser les fondations de la crate `profinet-rt` : workspace + CI, l'abstraction d'E/S trames Ethernet niveau 2 (trait mockable + backend `AF_PACKET`), et un harnais de capture/replay pour tester les couches supérieures contre des trames « golden » Wireshark.

**Architecture:** Crate Rust pure, sans pile C tierce. La couche `eth` expose un trait `EthTransport` (send/recv de trames brutes EtherType `0x8892`) avec deux implémentations : `MockTransport` (file en mémoire, pour les tests) et `AfPacketTransport` (socket raw Linux). Un module `capture` lit des fichiers `.pcap` (parser Rust pur) pour rejouer des échanges réels dans les tests. Ce plan ne dépend ni de la norme IEC ni d'un PLC.

**Tech Stack:** Rust 2021 (rust-version ≥ 1.74), `nix` (socket AF_PACKET), `thiserror` (erreurs), `pcap-file` (dev-dependency, parsing pcap pur Rust). CI : `cargo test` + `cargo clippy -D warnings` + `cargo fmt --check`.

## Global Constraints

- **Rust 100 % natif** — aucune pile/dépendance C tierce bundlée (interdiction stricte de code `p-net` ou autre pile GPL). Les bindings de syscalls (`nix`/`libc`) sont autorisés : ce sont des appels noyau, pas une pile C embarquée.
- **Double licence MIT OR Apache-2.0** — fichiers `LICENSE-MIT` et `LICENSE-APACHE` présents, champ `license = "MIT OR Apache-2.0"` dans chaque `Cargo.toml`.
- **Marque** — « PROFINET » uniquement en usage descriptif ; le README contient un disclaimer de non-affiliation PI. Pas de logo, pas de « certified ».
- **Aucun texte de norme IEC** recopié dans le code ou les commentaires (paraphrase uniquement).
- **Plateforme** — Linux (Debian PREEMPT_RT cible) ; le backend `AF_PACKET` est `#[cfg(target_os = "linux")]`.
- **EtherType PROFINET** = `0x8892`. **EtherType VLAN** = `0x8100`.

---

### Task 1 : Scaffold du workspace, CI, licences, disclaimer

**Files:**
- Create: `Cargo.toml` (workspace racine)
- Create: `crates/profinet-rt/Cargo.toml`
- Create: `crates/profinet-rt/src/lib.rs`
- Create: `LICENSE-MIT`, `LICENSE-APACHE`
- Create: `README.md`
- Create: `rustfmt.toml`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: (rien)
- Produces: crate `profinet-rt` compilable, fonction de fumée `pub fn version() -> &'static str`.

- [ ] **Step 1: Écrire le test de fumée**

Dans `crates/profinet-rt/src/lib.rs` :

```rust
//! `profinet-rt` — pile IO-Device PROFINET RT en Rust pur.
//!
//! Projet communautaire, NON affilié à / approuvé par PROFIBUS & PROFINET
//! International. « PROFINET » est une marque déposée de PNO.

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
```

- [ ] **Step 2: Créer les manifestes et fichiers de support**

`Cargo.toml` (racine) :

```toml
[workspace]
members = ["crates/profinet-rt"]
resolver = "2"

[workspace.package]
edition = "2021"
rust-version = "1.74"
license = "MIT OR Apache-2.0"
repository = "https://github.com/martinc8/profinet-rt"
```

`crates/profinet-rt/Cargo.toml` :

```toml
[package]
name = "profinet-rt"
version = "0.0.0"
description = "PROFINET RT IO-Device stack in pure Rust (community, not affiliated with PI)"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
nix = { version = "0.27", default-features = false, features = ["net", "socket"] }
thiserror = "1"

[dev-dependencies]
pcap-file = "2"
```

`rustfmt.toml` :

```toml
max_width = 100
```

`README.md` (au minimum) :

```markdown
# profinet-rt

Pile **IO-Device PROFINET RT classe 1 / CC-A** en Rust pur pour Linux PREEMPT_RT.

> **Disclaimer.** Projet communautaire, **non affilié à, ni approuvé ou certifié par**
> PROFIBUS & PROFINET International (PI). « PROFINET » est une marque déposée de PNO.
> Cette bibliothèque est une implémentation clean-room à partir de la norme publique
> IEC 61158/61784. Aucun texte de norme n'y est reproduit.

## Licence

Double licence, au choix : [MIT](LICENSE-MIT) ou [Apache-2.0](LICENSE-APACHE).
```

`.github/workflows/ci.yml` :

```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

Récupérer les textes standard MIT et Apache-2.0 dans `LICENSE-MIT` / `LICENSE-APACHE`.

- [ ] **Step 3: Vérifier que le projet compile et que le test passe**

Run: `cargo test --all`
Expected: PASS (`version_is_not_empty`), 0 warning.

- [ ] **Step 4: Vérifier lint + format**

Run: `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings`
Expected: aucune sortie d'erreur.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold workspace, CI, licences MIT/Apache, disclaimer PI"
```

---

### Task 2 : Parsing/sérialisation de l'en-tête Ethernet (+ VLAN)

**Files:**
- Create: `crates/profinet-rt/src/eth/mod.rs`
- Create: `crates/profinet-rt/src/eth/header.rs`
- Modify: `crates/profinet-rt/src/lib.rs` (ajout `pub mod eth;`)

**Interfaces:**
- Consumes: (rien)
- Produces:
  - `pub struct MacAddr(pub [u8; 6])`
  - `pub struct EthHeader { pub dst: MacAddr, pub src: MacAddr, pub vlan: Option<u16>, pub ethertype: u16 }`
  - `pub fn EthHeader::parse(buf: &[u8]) -> Result<(EthHeader, usize), EthError>` (renvoie l'en-tête + offset du payload)
  - `pub fn EthHeader::write(&self, out: &mut Vec<u8>)`
  - `pub enum EthError { TooShort }` (via `thiserror`)
  - constantes `pub const ETHERTYPE_PROFINET: u16 = 0x8892;` `pub const ETHERTYPE_VLAN: u16 = 0x8100;`

- [ ] **Step 1: Écrire les tests (parse sans VLAN, parse avec VLAN, round-trip, trop court)**

Dans `crates/profinet-rt/src/eth/header.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // dst=01:0e:cf:00:00:00, src=00:11:22:33:44:55, ethertype=0x8892, payload=[0xfe,0xfe]
    const FRAME_NO_VLAN: [u8; 16] = [
        0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x88, 0x92,
        0xfe, 0xfe,
    ];

    // même trame avec tag VLAN 0x8100, TCI=0xE000 (prio 7), avant l'ethertype
    const FRAME_VLAN: [u8; 20] = [
        0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x81, 0x00,
        0xe0, 0x00, 0x88, 0x92, 0xfe, 0xfe,
    ];

    #[test]
    fn parse_without_vlan() {
        let (h, off) = EthHeader::parse(&FRAME_NO_VLAN).unwrap();
        assert_eq!(h.dst, MacAddr([0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00]));
        assert_eq!(h.src, MacAddr([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]));
        assert_eq!(h.vlan, None);
        assert_eq!(h.ethertype, ETHERTYPE_PROFINET);
        assert_eq!(off, 14);
        assert_eq!(&FRAME_NO_VLAN[off..], &[0xfe, 0xfe]);
    }

    #[test]
    fn parse_with_vlan() {
        let (h, off) = EthHeader::parse(&FRAME_VLAN).unwrap();
        assert_eq!(h.vlan, Some(0xe000));
        assert_eq!(h.ethertype, ETHERTYPE_PROFINET);
        assert_eq!(off, 18);
    }

    #[test]
    fn round_trip_no_vlan() {
        let (h, _) = EthHeader::parse(&FRAME_NO_VLAN).unwrap();
        let mut out = Vec::new();
        h.write(&mut out);
        assert_eq!(out, &FRAME_NO_VLAN[..14]);
    }

    #[test]
    fn too_short_is_error() {
        assert!(matches!(EthHeader::parse(&[0u8; 8]), Err(EthError::TooShort)));
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p profinet-rt eth::header -v`
Expected: FAIL (compilation : `EthHeader` / `MacAddr` non définis).

- [ ] **Step 3: Implémenter l'en-tête**

En tête de `crates/profinet-rt/src/eth/header.rs` :

```rust
use thiserror::Error;

pub const ETHERTYPE_PROFINET: u16 = 0x8892;
pub const ETHERTYPE_VLAN: u16 = 0x8100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddr(pub [u8; 6]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthHeader {
    pub dst: MacAddr,
    pub src: MacAddr,
    pub vlan: Option<u16>,
    pub ethertype: u16,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EthError {
    #[error("frame too short")]
    TooShort,
}

impl EthHeader {
    /// Parse l'en-tête L2 ; renvoie (en-tête, offset du payload).
    pub fn parse(buf: &[u8]) -> Result<(Self, usize), EthError> {
        if buf.len() < 14 {
            return Err(EthError::TooShort);
        }
        let mut dst = [0u8; 6];
        let mut src = [0u8; 6];
        dst.copy_from_slice(&buf[0..6]);
        src.copy_from_slice(&buf[6..12]);

        let first = u16::from_be_bytes([buf[12], buf[13]]);
        let (vlan, ethertype, off) = if first == ETHERTYPE_VLAN {
            if buf.len() < 18 {
                return Err(EthError::TooShort);
            }
            let tci = u16::from_be_bytes([buf[14], buf[15]]);
            let et = u16::from_be_bytes([buf[16], buf[17]]);
            (Some(tci), et, 18)
        } else {
            (None, first, 14)
        };

        Ok((Self { dst: MacAddr(dst), src: MacAddr(src), vlan, ethertype }, off))
    }

    /// Sérialise l'en-tête (sans le payload) dans `out`.
    pub fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.dst.0);
        out.extend_from_slice(&self.src.0);
        if let Some(tci) = self.vlan {
            out.extend_from_slice(&ETHERTYPE_VLAN.to_be_bytes());
            out.extend_from_slice(&tci.to_be_bytes());
        }
        out.extend_from_slice(&self.ethertype.to_be_bytes());
    }
}
```

`crates/profinet-rt/src/eth/mod.rs` :

```rust
mod header;
pub use header::{EthError, EthHeader, MacAddr, ETHERTYPE_PROFINET, ETHERTYPE_VLAN};
```

Ajouter dans `crates/profinet-rt/src/lib.rs` : `pub mod eth;`

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p profinet-rt eth::header -v`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(eth): parsing/serialisation en-tete Ethernet + VLAN"
```

---

### Task 3 : Trait `EthTransport` + `MockTransport`

**Files:**
- Create: `crates/profinet-rt/src/eth/transport.rs`
- Modify: `crates/profinet-rt/src/eth/mod.rs` (exports)

**Interfaces:**
- Consumes: `MacAddr` (Task 2)
- Produces:
  - `pub trait EthTransport { fn send(&self, frame: &[u8]) -> Result<(), TransportError>; fn recv(&self, timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError>; }`
  - `pub struct MockTransport` avec `pub fn new() -> Self`, `pub fn push_rx(&self, frame: Vec<u8>)`, `pub fn sent(&self) -> Vec<Vec<u8>>`
  - `pub enum TransportError` (via `thiserror`)

- [ ] **Step 1: Écrire les tests du mock**

Dans `crates/profinet-rt/src/eth/transport.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_records_sent_frames() {
        let t = MockTransport::new();
        t.send(&[1, 2, 3]).unwrap();
        t.send(&[4, 5]).unwrap();
        assert_eq!(t.sent(), vec![vec![1, 2, 3], vec![4, 5]]);
    }

    #[test]
    fn mock_returns_pushed_rx_in_order_then_none() {
        let t = MockTransport::new();
        t.push_rx(vec![9, 9]);
        assert_eq!(t.recv(None).unwrap(), Some(vec![9, 9]));
        assert_eq!(t.recv(None).unwrap(), None);
    }
}
```

- [ ] **Step 2: Lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p profinet-rt eth::transport -v`
Expected: FAIL (compilation : `MockTransport` non défini).

- [ ] **Step 3: Implémenter le trait et le mock**

En tête de `crates/profinet-rt/src/eth/transport.rs` :

```rust
use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(String),
}

/// Abstraction d'E/S de trames Ethernet brutes (en-tête L2 inclus).
pub trait EthTransport {
    fn send(&self, frame: &[u8]) -> Result<(), TransportError>;
    /// Renvoie `Ok(None)` si aucune trame n'est disponible avant `timeout`.
    fn recv(&self, timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError>;
}

/// Transport en mémoire pour les tests.
#[derive(Default)]
pub struct MockTransport {
    tx: Mutex<Vec<Vec<u8>>>,
    rx: Mutex<std::collections::VecDeque<Vec<u8>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }
    /// Empile une trame que `recv` retournera ensuite (FIFO).
    pub fn push_rx(&self, frame: Vec<u8>) {
        self.rx.lock().unwrap().push_back(frame);
    }
    /// Toutes les trames émises via `send`, dans l'ordre.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.tx.lock().unwrap().clone()
    }
}

impl EthTransport for MockTransport {
    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        self.tx.lock().unwrap().push(frame.to_vec());
        Ok(())
    }
    fn recv(&self, _timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(self.rx.lock().unwrap().pop_front())
    }
}
```

Mettre à jour `crates/profinet-rt/src/eth/mod.rs` :

```rust
mod header;
mod transport;
pub use header::{EthError, EthHeader, MacAddr, ETHERTYPE_PROFINET, ETHERTYPE_VLAN};
pub use transport::{EthTransport, MockTransport, TransportError};
```

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p profinet-rt eth::transport -v`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(eth): trait EthTransport + MockTransport"
```

---

### Task 4 : Backend `AfPacketTransport` (socket raw Linux)

**Files:**
- Create: `crates/profinet-rt/src/eth/afpacket.rs`
- Modify: `crates/profinet-rt/src/eth/mod.rs` (export `#[cfg(target_os = "linux")]`)

**Interfaces:**
- Consumes: `EthTransport`, `TransportError` (Task 3)
- Produces:
  - `#[cfg(target_os = "linux")] pub struct AfPacketTransport`
  - `pub fn AfPacketTransport::open(ifname: &str) -> Result<Self, TransportError>` — ouvre un `AF_PACKET`/`SOCK_RAW` lié à l'interface, filtré sur EtherType PROFINET
  - implémente `EthTransport`

- [ ] **Step 1: Écrire le test (erreur sur interface inexistante)**

> Note : `open()` sur une vraie interface exige `CAP_NET_RAW` et un NIC ; ces tests sont
> marqués `#[ignore]` et lancés manuellement. Le test non-ignoré valide le chemin d'erreur.

Dans `crates/profinet-rt/src/eth/afpacket.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_unknown_interface_errors() {
        let r = AfPacketTransport::open("nonexistent-iface-xyz");
        assert!(r.is_err());
    }

    #[test]
    #[ignore = "necessite CAP_NET_RAW + une interface reelle ; lancer: cargo test -- --ignored"]
    fn open_loopback_succeeds() {
        // Adapter le nom d'interface a la machine de test (ex. "lo", "eth0").
        let t = AfPacketTransport::open("lo").expect("open lo");
        let _ = t.recv(Some(std::time::Duration::from_millis(10)));
    }
}
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p profinet-rt eth::afpacket -v`
Expected: FAIL (compilation : `AfPacketTransport` non défini).

- [ ] **Step 3: Implémenter le backend AF_PACKET**

En tête de `crates/profinet-rt/src/eth/afpacket.rs` (implémentation via `nix`) :

```rust
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Duration;

use nix::sys::socket::{
    bind, recv, send, socket, AddressFamily, LinkAddr, MsgFlags, SockFlag, SockProtocol, SockType,
};

use super::transport::{EthTransport, TransportError};
use super::ETHERTYPE_PROFINET;

fn io_err<E: std::fmt::Display>(e: E) -> TransportError {
    TransportError::Io(e.to_string())
}

/// Socket raw AF_PACKET lié à une interface, filtré sur l'EtherType PROFINET.
pub struct AfPacketTransport {
    fd: OwnedFd,
}

impl AfPacketTransport {
    pub fn open(ifname: &str) -> Result<Self, TransportError> {
        let proto = SockProtocol::EthAll; // on filtre l'EtherType nous-memes au recv
        let fd = socket(AddressFamily::Packet, SockType::Raw, SockFlag::empty(), proto)
            .map_err(io_err)?;

        let ifindex = nix::net::if_::if_nametoindex(ifname).map_err(io_err)?;
        let addr = LinkAddr::new(libc_ethertype_all(), ifindex as i32);
        bind(fd.as_raw_fd(), &addr).map_err(io_err)?;

        Ok(Self { fd })
    }
}

fn libc_ethertype_all() -> u16 {
    // ETH_P_ALL = 0x0003 (htons applique par la couche socket selon la plateforme)
    0x0003
}

impl EthTransport for AfPacketTransport {
    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        send(self.fd.as_raw_fd(), frame, MsgFlags::empty()).map_err(io_err)?;
        Ok(())
    }

    fn recv(&self, _timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError> {
        let mut buf = vec![0u8; 1522];
        let n = recv(self.fd.as_raw_fd(), &mut buf, MsgFlags::empty()).map_err(io_err)?;
        buf.truncate(n);
        // Filtre EtherType PROFINET (offset 12, hors VLAN — un filtrage fin viendra avec eth::header).
        if n >= 14 && u16::from_be_bytes([buf[12], buf[13]]) == ETHERTYPE_PROFINET {
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }
}
```

> ⚠️ Détail d'implémentation à ajuster à l'exécution selon la version de `nix` (signatures
> `LinkAddr`/`SockProtocol`). Si l'API `nix` diverge, retomber sur `libc` brut pour
> `socket(AF_PACKET, SOCK_RAW, htons(ETH_P_ALL))` + `bind(sockaddr_ll)`. Le contrat public
> (`open`, `send`, `recv`) reste identique. La gestion du `timeout` (via `setsockopt
> SO_RCVTIMEO` ou `poll`) sera affinée au Plan 4 quand la boucle RT en aura besoin.

Mettre à jour `crates/profinet-rt/src/eth/mod.rs` :

```rust
#[cfg(target_os = "linux")]
mod afpacket;
#[cfg(target_os = "linux")]
pub use afpacket::AfPacketTransport;
```

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p profinet-rt eth::afpacket -v`
Expected: PASS (`open_unknown_interface_errors`), l'autre est `ignored`.

Test manuel (sur l'edge, avec droits) : `sudo -E cargo test -p profinet-rt -- --ignored`

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(eth): backend AF_PACKET (socket raw Linux)"
```

---

### Task 5 : Harnais de capture/replay (lecture pcap golden-frames)

**Files:**
- Create: `crates/profinet-rt/src/capture.rs`
- Create: `crates/profinet-rt/tests/fixtures/README.md` (où déposer les `.pcap`)
- Create: `crates/profinet-rt/tests/capture_replay.rs`
- Modify: `crates/profinet-rt/src/lib.rs` (ajout `pub mod capture;`)

**Interfaces:**
- Consumes: (rien)
- Produces:
  - `pub struct PcapFrames { ... }`
  - `pub fn PcapFrames::open(path: &Path) -> Result<PcapFrames, CaptureError>`
  - `impl Iterator for PcapFrames { type Item = Vec<u8>; }` (chaque item = trame Ethernet brute)
  - `pub enum CaptureError` (via `thiserror`)

- [ ] **Step 1: Écrire le test unitaire (count + 1re trame) sur un pcap généré en mémoire**

Dans `crates/profinet-rt/src/capture.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pcap_file::pcap::{PcapPacket, PcapWriter};
    use std::io::Cursor;
    use std::time::Duration;

    fn make_pcap(frames: &[&[u8]]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = PcapWriter::new(&mut buf).unwrap();
            for f in frames {
                w.write_packet(&PcapPacket::new(Duration::ZERO, f.len() as u32, f))
                    .unwrap();
            }
        }
        buf
    }

    #[test]
    fn reads_all_frames_in_order() {
        let bytes = make_pcap(&[&[0xaa, 0xbb], &[0xcc]]);
        let frames: Vec<Vec<u8>> = PcapFrames::from_reader(Cursor::new(bytes))
            .unwrap()
            .collect();
        assert_eq!(frames, vec![vec![0xaa, 0xbb], vec![0xcc]]);
    }
}
```

- [ ] **Step 2: Lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p profinet-rt capture -v`
Expected: FAIL (compilation : `PcapFrames` non défini).

- [ ] **Step 3: Implémenter le lecteur pcap**

En tête de `crates/profinet-rt/src/capture.rs` :

```rust
use std::fs::File;
use std::io::Read;
use std::path::Path;

use pcap_file::pcap::PcapReader;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("io error: {0}")]
    Io(String),
    #[error("pcap parse error: {0}")]
    Pcap(String),
}

/// Itérateur sur les trames Ethernet brutes d'un fichier pcap.
pub struct PcapFrames<R: Read> {
    reader: PcapReader<R>,
}

impl PcapFrames<File> {
    pub fn open(path: &Path) -> Result<Self, CaptureError> {
        let file = File::open(path).map_err(|e| CaptureError::Io(e.to_string()))?;
        Self::from_reader(file)
    }
}

impl<R: Read> PcapFrames<R> {
    pub fn from_reader(r: R) -> Result<Self, CaptureError> {
        let reader = PcapReader::new(r).map_err(|e| CaptureError::Pcap(e.to_string()))?;
        Ok(Self { reader })
    }
}

impl<R: Read> Iterator for PcapFrames<R> {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next_packet() {
            Some(Ok(pkt)) => Some(pkt.data.into_owned()),
            _ => None,
        }
    }
}
```

> ⚠️ `pcap-file` doit être promu de `dev-dependencies` vers `dependencies` puisque
> `capture` fait partie de l'API publique. Déplacer la ligne `pcap-file = "2"` dans
> `[dependencies]` du `Cargo.toml` de la crate.

`crates/profinet-rt/tests/fixtures/README.md` :

```markdown
# Fixtures golden-frames

Déposer ici les captures Wireshark (`.pcap`) d'échanges PROFINET de référence
(DCP, établissement AR, trames RT). Elles servent de vérité terrain aux tests
des plans 2+ (parsing/sérialisation par couche).
```

`crates/profinet-rt/tests/capture_replay.rs` :

```rust
//! Test d'intégration : rejoue un pcap fixture s'il existe (sinon ignoré).
use profinet_rt::capture::PcapFrames;
use std::path::Path;

#[test]
fn replay_fixture_if_present() {
    let p = Path::new("tests/fixtures/sample.pcap");
    if !p.exists() {
        eprintln!("pas de fixture sample.pcap — test ignoré");
        return;
    }
    let n = PcapFrames::open(p).unwrap().count();
    assert!(n > 0, "le pcap fixture ne doit pas être vide");
}
```

Ajouter dans `crates/profinet-rt/src/lib.rs` : `pub mod capture;`

- [ ] **Step 4: Lancer les tests pour vérifier qu'ils passent**

Run: `cargo test -p profinet-rt -v`
Expected: PASS (tous les tests, y compris `reads_all_frames_in_order` et `replay_fixture_if_present`).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(capture): lecteur pcap golden-frames + harnais de replay"
```

---

## Feuille de route des plans suivants (à détailler le moment venu)

Chaque plan produit un livrable testable et s'appuie sur des captures Wireshark de
référence + la norme IEC acquise. Ordre conseillé :

- **Plan 2 — `dcp`** : Discovery & Config Protocol (Identify / Get / Set name-of-station /
  Set IP / flash). Premier dialogue observable avec TIA Portal ; testable contre golden
  frames et en HIL (« le device apparaît et se laisse nommer dans TIA »).
- **Plan 3 — `cm` / établissement AR** : DCE/RPC sur UDP 34964, machine d'état AR
  (Connect / Write records / Read / Dcontrol / Ccontrol / Release). **Cœur du risque.**
  Cible : l'AR atteint l'état DATA.
- **Plan 4 — `rt` cyclique** : PPM/CPM, IOPS/IOCS, data status, cycle counter, watchdog ;
  thread RT `SCHED_FIFO` + image d'E/S double-buffer/seqlock. Cible : send clock 1 ms tenu,
  données qui font l'aller-retour.
- **Plan 5 — `alarm` + `im`** : alarme Application-Ready (indispensable pour passer en RUN),
  alarmes plug/return-of-submodule, records I&M0.
- **Plan 6 — `config` + GSDML + API publique** : modèle de config typé (BOOL/INT/DINT/REAL/
  WORD), génération/cohérence du GSDML d'exemple (16 REAL + 32 BOOL), façade `ProfinetDevice`.
- **Plan 7 — Intégration HIL + déterminisme** : banc S7-1500 réel, vérif automatisée AR→RUN
  + aller-retour de valeurs, mesure de gigue (style `cyclictest`), guide de tuning PREEMPT_RT
  (`isolcpus`, `nohz_full`, affinité IRQ), binaire de démonstration.
```
