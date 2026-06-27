# Protocole de banc — captures PROFINET (vérité-terrain pour Plans 2-5)

But : capturer **côté IO-Device** une session PROFINET complète avec un pair conforme,
pour alimenter les vecteurs de test des Plans 2-5 et valider les points ouverts
(`FOLLOWUPS.md`, notamment l'ordre des bits BOOL LSB-first).

## Topologie

```
   S7-1500 physique  ───câble───  NIC PC Windows
   (IO-Controller)                (TIA + PLCSIM Advanced
    192.168.1.60                   = PLC virtuel en I-Device
    "io-controller"                = IO-Device = NOTRE modèle
                                    192.168.1.61 "i-device")
                                        │
                            capture Wireshark sur
                            "Siemens PLCSIM Virtual Ethernet Adapter"
```

Avantage : l'adaptateur virtuel Siemens voit tout son trafic L2 → capture DCP + AR + RT
**sans TAP ni switch managé**.

⚠️ Le timing/jitter PLCSIM n'est PAS représentatif (simu). La **structure des trames est
fidèle** = suffisant pour Plans 2-5. La validation déterminisme < 2 ms (Plan 7) exigera le
banc 100 % physique.

## Plan d'adressage

| Rôle | IP | NameOfStation |
|---|---|---|
| IO-Controller (S7-1500 1515-2 PN, FW V2.9) | 192.168.1.60 | `io-controller` |
| I-Device (PLC virtuel PLCSIM Advanced) | 192.168.1.61 | `i-device` |
| PC de capture (NIC) | 192.168.1.10 | — |

Send clock : **1 ms**. Mapping d'exemple : **16 REAL + 32 BOOL par sens** (= 68 octets/sens).

## Capture

- **Filtre de capture** (npcap/libpcap) :
  ```
  ether proto 0x8892 or vlan or udp port 34964
  ```
- **Filtre d'affichage** Wireshark : `pn_dcp or pn_rt or pn_io or udp.port == 34964`
- Wireshark a un dissecteur PROFINET natif → sert d'oracle pour valider notre décodage.
- Équivalent edge (plus tard, quand la pile Rust tournera) :
  ```
  sudo tcpdump -i eth0 -w capture.pcap 'ether proto 0x8892 or vlan or udp port 34964'
  ```

## Scénario (1 .pcapng par phase)

- [ ] **1. DCP Identify** — TIA → *Accessible devices*. Capturable sur la NIC du PC **sans
      I-Device**. → Plan 2. Fichier : `dcp-identify.pcapng`
- [ ] **2. DCP Set** — clic droit device → *Assign PROFINET device name*. → Plan 2.
      Fichier : `dcp-set.pcapng`
- [ ] **3. Connect / AR** — mise en RUN, montée de l'AR : RPC Connect / Write Record /
      Dcontrol / Ccontrol (UDP 34964). → Plan 3. Fichier : `ar-connect.pcapng`
- [ ] **4. Cyclique RT** — RUN stable ~5-10 s : PPM/CPM à 1 ms, IOPS/IOCS/data-status.
      → Plan 4. **Valider ici l'ordre des bits BOOL (LSB-first).** Fichier : `rt-cyclic.pcapng`
- [ ] **5. Alarme** — forcer un défaut (module retiré / voie forcée). → Plan 5.
      Fichier : `alarm.pcapng`
- [ ] **6. Déconnexion** — STOP propre. Fichier : `release.pcapng`

Garder aussi des **extraits courts** (1-2 trames/type) pour des tests unitaires légers.

## Métadonnées à relever (référence clean-room)

- [ ] Send clock + reduction ratio + temps de mise à jour du device
- [ ] NameOfStation des deux côtés (confirmés ci-dessus)
- [ ] Vendor ID / Device ID observés sur le fil
- [ ] Présence d'un tag VLAN + priorité (souvent 6 pour RT)
- [ ] **Export GSDML** du I-Device d'exemple → référence mapping déclaration → (octet, bit)

## Dépôt

Captures dans `203-profinet-rt/captures/`. Rejouables par `capture::PcapFrames`.
