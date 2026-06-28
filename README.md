# profinet-rt

[![CI](https://github.com/core-engineering/profinet-rt/actions/workflows/ci.yml/badge.svg)](https://github.com/core-engineering/profinet-rt/actions/workflows/ci.yml)
![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![status](https://img.shields.io/badge/statut-pré--1.0%20(WIP)-orange)

Pile **IO-Device PROFINET RT (classe 1 / Conformance Class A)** en **Rust pur**, pour Linux
**PREEMPT_RT** — pensée pour fermer des boucles de régulation côté *edge* avec un automate
S7‑1500 (IO‑Controller), cycle visé **< 2 ms**.

> **Disclaimer.** Projet communautaire, **non affilié à, ni approuvé ou certifié par**
> PROFIBUS & PROFINET International (PI). « PROFINET » est une marque déposée de PNO.
> Cette bibliothèque est une implémentation **clean‑room** à partir de la norme publique
> IEC 61158/61784. Aucun texte de norme n'y est reproduit.

## Pourquoi

Les protocoles d'échange acycliques (S7comm, Modbus, OPC UA) ne conviennent pas à une
**régulation déterministe** : il faut le canal **cyclique temps réel** de PROFINET. Les piles
existantes posent des contraintes de licence (p. ex. `p-net` est en GPLv3 + commerciale). Ce
projet vise une pile **réutilisable, sous double licence permissive**, dont on maîtrise l'IP.

## État

Développement actif, **pré‑1.0**. Validé **byte‑exact** contre des captures réelles d'un
S7‑1500 (1515‑2 PN).

| Brique | Module | État |
|---|---|---|
| Couche L2 Ethernet (en‑tête + VLAN, transport AF_PACKET, mock) | `eth` | ✅ |
| Harnais de rejeu de captures **pcap & pcapng** | `capture` | ✅ |
| Codecs des types process (INT/WORD/DINT/REAL big‑endian, BOOL packé) | `data` | ✅ |
| **DCP** côté device : Identify (parse requête + réponse byte‑exact, dispatch) | `dcp` | ✅ |
| DCP Get / Set‑name / Set‑IP / Flash | `dcp` | ⏳ |
| Établissement d'AR (DCE/RPC, machine d'état) | `cm` | ⏳ |
| Échange cyclique RT (PPM/CPM, IOPS/IOCS, watchdog, thread `SCHED_FIFO`) | `rt` | ⏳ |
| Alarmes + I&M | `alarm`/`im` | ⏳ |
| Modèle de config + GSDML + API publique | `config` | ⏳ |
| Intégration HIL + déterminisme (S7‑1500 réel, mesure de gigue) | — | ⏳ |

## Architecture

- **Rust pur**, pas de dépendance lourde ; tout est en **big‑endian** (format « Motorola »,
  identique à la mémoire Siemens — pas de word‑swap).
- Décomposition par couches du protocole (`eth` → `dcp` → `cm`/AR → `rt`/alarmes), chacune
  testable indépendamment.
- Cible runtime : Debian **PREEMPT_RT**, send clock 1 ms, thread RT `SCHED_FIFO`, image d'E/S
  double‑buffer/seqlock (à venir avec la couche `rt`).

## Démarrage rapide

```bash
git clone https://github.com/core-engineering/profinet-rt.git
cd profinet-rt
cargo test          # suite unitaire + test d'intégration de rejeu de capture
cargo clippy --all-targets -- -D warnings
```

Le backend `AfPacketTransport` (sockets L2 brutes) nécessite Linux et la capability
`CAP_NET_RAW` à l'exécution ; les tests qui en dépendent sont marqués `#[ignore]`.

## Approche clean‑room

L'implémentation est dérivée de la **norme IEC publique** (IEC 61158‑6‑10 pour le protocole,
61158‑5‑10 pour les services, 61784‑2‑3 pour les profils RT) et de **captures Wireshark** de
trafic réel. Les trames de référence (« golden frames ») et leur provenance sont documentées
dans [`docs/dcp-golden-frames.md`](docs/dcp-golden-frames.md). Aucun code tiers sous copyleft
n'est inclus.

> ⚠️ Pour un déploiement réel, un **Vendor ID** légitime auprès de PI est requis (l'exemple
> utilise des valeurs de test).

## Documentation

- Conception : [`docs/superpowers/specs/`](docs/superpowers/specs/)
- Plans d'implémentation (TDD, tâche par tâche) : [`docs/superpowers/plans/`](docs/superpowers/plans/)
- Bancs d'essai : [`docs/bench-capture-protocol.md`](docs/bench-capture-protocol.md),
  [`docs/bench-pnet-device.md`](docs/bench-pnet-device.md)

## Feuille de route

`cm` (AR) → `rt` (cyclique 1 ms) → `alarm`/`im` → `config`/GSDML/API → intégration HIL &
mesure de déterminisme. Détail dans les plans ci‑dessus.

## Licence

Au choix : [MIT](LICENSE-MIT) ou [Apache‑2.0](LICENSE-APACHE)
(`SPDX: MIT OR Apache-2.0`).

Sauf mention contraire, toute contribution soumise pour inclusion est sous cette double licence.
