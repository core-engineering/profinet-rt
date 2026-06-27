# captures/

Captures PROFINET de banc (vérité-terrain). **Les `.pcapng` ne sont pas versionnés**
(volumineux + reproductibles ; risque de corruption git sous WSL/NTFS). Les octets de
référence extraits sont figés dans [`../docs/dcp-golden-frames.md`](../docs/dcp-golden-frames.md)
et embarqués en hex dans les tests du module `dcp`.

## Provenance
Banc 2026-06-26 : **S7-1500 CPU 1515-2 PN (FW V2.9)** = IO-Controller ↔ instance **PLCSIM
Advanced `i-device`**, segment isolé (sans CPL), capture via Wireshark/npcap, décodage tshark 4.6.6.

| Fichier | Contenu |
|---|---|
| `dcp-identify.pcapng` | DCP Identify req/resp (golden frames du Plan 2) |
| `dcp-identify-01.pcapng` | idem, segment nettoyé (sans CPL) ; montre aussi le reject AR `nca_unk_if` |
| `dcp-set.pcapng` | cycles Identify/connect-retry (pas de DCP-Set réel : PLCSIM n'en reçoit pas) |

## Limite connue
PLCSIM Advanced **ne fait pas de PROFINET IO temps réel** (AR/RT cyclique) sur le fil → pas de
golden frames Connect/AR/RT/alarme ici. À capturer avec un **device réel** ou **p-net sur l'edge**
(cf. notes projet). Filtre de capture : `ether proto 0x8892 or vlan or udp port 34964`.
