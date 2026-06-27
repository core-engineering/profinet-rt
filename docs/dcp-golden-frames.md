# DCP — Golden frames de référence (vérité-terrain)

Trames réelles capturées sur banc : **S7-1500 CPU 1515-2 PN (FW V2.9)** = IO-Controller
↔ instance PLCSIM `i-device`. Capture `captures/dcp-identify.pcapng`, décodage croisé
avec Wireshark/tshark 4.6.6 (dissecteur `pn_dcp`). Servent de **vecteurs de test
byte-exact** pour le module `dcp`.

Constantes communes :
- EtherType PROFINET : `0x8892` (module `eth` déjà en place)
- MAC multicast DCP Identify : `01:0e:cf:00:00:00`
- FrameIDs : `0xfefe` = Identify **request** (multicast) ; `0xfeff` = Identify **response**
  (unicast) ; `0xfefd` = Get/Set ; `0xfefc` = Hello.
- En-tête DCP (après FrameID) : `ServiceID(1) ServiceType(1) Xid(4) ResponseDelay/resv(2) DCPDataLength(2)` puis blocs.
- ServiceID : Get=3, Set=4, Identify=5, Hello=6. ServiceType : bit0 0=Request 1=Response-success.
- **Bloc TLV** : `Option(1) Suboption(1) DCPBlockLength(2) [BlockInfo(2)] Value(...)`,
  **paddé à une longueur paire** (octet `0x00` si impair).
  - ⚠️ Dans une **requête Identify**, le bloc *filtre* NameOfStation n'a **PAS** de BlockInfo
    (valeur = nom brut). Dans une **réponse**, chaque bloc commence par 2 octets **BlockInfo**.

---

## 1. Identify REQUEST (controller → multicast) — 56 octets, FrameID 0xfefe

```
010ecf000000ec1c5d61e73f8892fefe0500030001520001000c02020008692d646576696365000000000000000000000000000000000000
```

| Offset | Octets | Champ | Valeur |
|---|---|---|---|
| 0 | `01 0e cf 00 00 00` | Eth dst | multicast DCP |
| 6 | `ec 1c 5d 61 e7 3f` | Eth src | contrôleur (Siemens) |
| 12 | `88 92` | EtherType | PROFINET |
| 14 | `fe fe` | FrameID | Identify req |
| 16 | `05` | ServiceID | Identify |
| 17 | `00` | ServiceType | Request |
| 18 | `03 00 01 52` | Xid | 0x03000152 |
| 22 | `00 01` | ResponseDelay | 1 |
| 24 | `00 0c` | DCPDataLength | 12 |
| 26 | `02 02 00 08` | Bloc : opt=2(DeviceProperties) sub=2(NameOfStation) len=8 | **filtre, sans BlockInfo** |
| 30 | `69 2d 64 65 76 69 63 65` | NameOfStation | "i-device" |
| 38.. | `00…` | Padding | bourrage trame mini 56 o |

## 2. Identify RESPONSE (device → controller) — 114 octets, FrameID 0xfeff

```
ec1c5d61e73f02c0a8010f028892feff050103000152000000580202000a0000692d646576696365020500040000020702010012000053372d313530302028504c4353494d29020300060000002a010e020400040000000002070004000010640102000e0001c0a8013dffffff00c0a8013d
```

En-tête : `feff` / ServiceID=05 / ServiceType=01 / Xid=`03000152` (= celui de la req) /
ResponseDelay=`0000` / DCPDataLength=`0058` (88). Puis 7 blocs (chacun avec **BlockInfo 2 o**) :

| opt.sub | Nom | len | BlockInfo | Value décodée |
|---|---|---|---|---|
| 2.2 | NameOfStation | 10 | `0000` | "i-device" |
| 2.5 | DeviceOptions | 4 | `0000` | `0207` |
| 2.1 | TypeOfStation | 18 | `0000` | "S7-1500 (PLCSIM)" |
| 2.3 | DeviceID (Vendor/Device) | 6 | `0000` | VendorID `0x002A` (Siemens), DeviceID `0x010E` |
| 2.4 | DeviceRole | 4 | `0000` | `0000` |
| 2.7 | DeviceInstance | 4 | `0000` | `1064` |
| 1.2 | IPParameter | 14 | `0001` | ip `192.168.1.61` mask `255.255.255.0` gw `192.168.1.61` |

Notes :
- Le device répond avec **le Xid de la requête** (corrélation requête/réponse).
- IPParameter BlockInfo `0x0001` = IP set/valide ; valeurs en big-endian (`c0a8013d` = 192.168.1.61).
- "i-device" = 8 octets (pair) → pas d'octet de padding après ce bloc ; TypeOfStation = 16 o (pair) aussi.

## Sources
- `captures/dcp-identify.pcapng` (banc 2026-06-26)
- Décodage : Wireshark/tshark 4.6.6 `pn_dcp`
- ⚠️ Pas encore de golden frame pour **Get / Set-name / Set-IP / Flash** (PLCSIM ne reçoit pas
  de Set ; à capturer sur device réel ou via p-net). Structure TLV identique ; à valider plus tard.
