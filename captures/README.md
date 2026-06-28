# captures/

Bench PROFINET captures (ground truth). **The `.pcapng` files are not versioned**
(large + reproducible; risk of git corruption under WSL/NTFS). The extracted reference
bytes are frozen in [`../docs/dcp-golden-frames.md`](../docs/dcp-golden-frames.md)
and embedded as hex in the `dcp` module tests.

## Provenance
Bench 2026-06-26: **S7-1500 CPU 1515-2 PN (FW V2.9)** = IO-Controller ↔ **PLCSIM
Advanced `i-device`** instance, isolated segment (no CPL), captured via Wireshark/npcap,
decoded with tshark 4.6.6.

| File | Contents |
|---|---|
| `dcp-identify.pcapng` | DCP Identify req/resp (Plan 2 golden frames) |
| `dcp-identify-01.pcapng` | same, cleaned segment (no CPL); also shows AR reject `nca_unk_if` |
| `dcp-set.pcapng` | Identify/connect-retry cycles (no real DCP-Set: PLCSIM does not receive any) |

## Known limitation
PLCSIM Advanced **does not perform real-time PROFINET IO** (AR/RT cyclic) on the wire →
no Connect/AR/RT/alarm golden frames here. To be captured with a **real device** or
**p-net on the edge** (see project notes). Capture filter: `ether proto 0x8892 or vlan or udp port 34964`.
