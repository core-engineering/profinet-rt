# Spec — `profinet-rt` : pile IO-Device PROFINET RT en Rust

- **Date** : 2026-06-25
- **Statut** : design validé, en attente de relecture utilisateur
- **Auteur** : Camille Martin
- **Projet** : `203-profinet-rt`

## 1. Contexte & objectif

Disposer d'une **bibliothèque de communication Rust réutilisable** permettant à une
machine *edge* (Debian PREEMPT_RT) de participer à l'échange **cyclique PROFINET RT**
avec un automate **S7-1500** (IO-Controller), afin de fermer des **boucles de
régulation** côté edge avec un temps de cycle **< 2 ms**.

Le besoin initial parlait d'« interface Profinet READ/WRITE » et d'un « protocole OT
éprouvé plutôt que S7 ». Le cadrage a clarifié que :

- Ni S7/S7+, ni Modbus, ni OPC UA ne conviennent : ce sont des protocoles
  requête/réponse **acycliques**, inadaptés à une boucle de régulation déterministe.
- Le besoin réel est le **canal cyclique RT** de PROFINET, où l'edge se comporte en
  **IO-Device** scruté par l'automate à chaque cycle (le PLC fournit les PV, l'edge
  calcule, renvoie les MV).

## 2. Périmètre

### Dans le périmètre

- Pile **IO-Device PROFINET RT classe 1, Conformance Class A (CC-A)**, en **Rust pur**
  (aucune dépendance C), pour **Linux PREEMPT_RT**.
- Send clock **1 ms**, reduction ratio 1 (cycle effectif < 2 ms).
- Échange cyclique de **données process typées** : `BOOL`, `INT`, `DINT`, `REAL`,
  `WORD`, dans les deux sens.
- Services nécessaires à l'interopérabilité avec un S7-1500 :
  DCP, établissement de l'AR (DCE/RPC), RT cyclique (PPM/CPM), alarmes minimales,
  records I&M0.
- **API de bibliothèque générique** sur le mapping de données, fournie via une
  configuration ; **GSDML d'exemple** livré pour le cas 16 `REAL` + 32 `BOOL` par sens.

### Hors périmètre

- Toute **logique de régulation / métier** : elle est *consommatrice* de la lib via
  une API propre, et ignore PROFINET.
- **IRT / isochrone (CC-C)**, TSN.
- **Certification PROFINET** officielle (label PI).
- Rôle **IO-Controller** (l'edge est uniquement IO-Device).
- Topologie / LLDP (CC-B) — *non requis en CC-A*, voir évolutions possibles.

## 3. Décisions de cadrage (verrouillées)

| Sujet | Décision | Raison |
|---|---|---|
| Rôle | Edge = **IO-Device** | Le S7-1500 reste IO-Controller et possède les E/S réelles |
| Protocole | **PROFINET RT classe 1 / CC-A** | Seul canal cyclique déterministe pour la régulation |
| Cycle | **send clock 1 ms**, < 2 ms | Cible utilisateur, jouable en logiciel sur PREEMPT_RT |
| Langage | **Rust 100 % natif** | Éviter GPLv3 / licence commerciale de `p-net` ; posséder l'IP |
| Déterminisme | **Logiciel** (pas d'offload HW) | < 2 ms RT classe 1 atteignable sur PREEMPT_RT bien configuré |
| Cible | **Interop fonctionnelle**, pas de certif | Usage interne edge ↔ S7-1500 |
| Livrable | **Crate de communication réutilisable** | Réemploi sur d'autres projets |

## 4. Pourquoi pas `p-net` (rappel décision)

`p-net` (rt-labs) est en **double licence GPLv3 + commerciale**. La version libre est
GPLv3 (copyleft → contamine tout le produit edge) et annoncée *« not intended for
production use »*. La version prod impose l'achat de la licence commerciale. Écrire une
pile Rust native évite **et** la contamination GPLv3 **et** la redevance, **et** donne
la pleine propriété de l'IP. Aucune pile PROFINET mûre n'existe en Rust (terrain neuf).

> **Hygiène licence** : l'implémentation est **clean-room**. On n'incorpore aucun code
> `p-net` (ni d'autre pile GPL). Référence = la **norme IEC 61158 / 61784 acquise
> légalement** + le dissecteur Wireshark + nos propres captures.

## 5. Architecture

### 5.1 Décomposition en couches

Chaque couche a une responsabilité unique, une interface définie, et est testable
isolément.

| Module | Responsabilité | Dépend de | Risque |
|---|---|---|---|
| `eth` | E/S trames L2 EtherType `0x8892` via `AF_PACKET` (idéalement `PACKET_MMAP`), derrière un trait mockable | NIC | faible |
| `dcp` | Discovery & Config Protocol : identify, set-name-of-station, set-IP, flash | `eth` | moyen |
| `cm` (Context Manager) | DCE/RPC sur UDP 34964 : machine d'état de l'**AR** (Connect / Write / Read / Dcontrol / Ccontrol / Release), records | UDP | **élevé** |
| `rt` | Échange cyclique **PPM** (producteur edge→PLC) / **CPM** (consommateur PLC→edge) : IOPS/IOCS, data status, cycle counter, watchdog consommateur | `eth` | **élevé** |
| `alarm` | Canal alarmes RT minimal : Application-Ready, plug / return-of-submodule | `cm` | moyen |
| `im` | Records **I&M0** (identification obligatoire) | `cm` | faible |
| `config` | Modèle de config (slots / sous-modules + map de variables typées) ↔ GSDML | — | moyen |
| `api` | `ProfinetDevice` + accesseurs typés, gestion de l'image d'E/S, état de l'AR | tous | — |

### 5.2 Modèle de threads (cœur du déterminisme)

- **Thread RT** : `SCHED_FIFO`, épinglé sur un cœur isolé (`isolcpus` / `nohz_full`),
  IRQ de la NIC routées sur ce cœur. Il possède la boucle PPM/CPM cadencée au send
  clock. **Contrainte dure : aucune allocation, aucun lock bloquant, aucun I/O
  syscall lent dans cette boucle.**
- **Thread acyclique** : priorité normale ; gère DCP, RPC/AR, alarmes, I&M.
- **Échange RT ↔ application** : image d'E/S partagée via **double-buffer / seqlock**
  (accès non bloquant côté RT), contrat de cohérence **par cycle**. L'application
  consommatrice ne touche jamais au réseau.

### 5.3 Mapping des types (rappel : PROFINET = big-endian « Motorola »)

PROFINET et la mémoire Siemens étant tous deux big-endian, **aucun word-swap** (à la
différence de Modbus). Les données d'un sous-module sont un tableau d'octets, encadré
par les octets de statut provider/consumer (IOPS/IOCS).

| Type | Taille | Encodage sur le fil |
|---|---|---|
| `BOOL` | 1 bit | bits packés (8/octet), exposés via index |
| `INT` | 2 o | i16 big-endian |
| `WORD` | 2 o | u16 big-endian |
| `DINT` | 4 o | i32 big-endian |
| `REAL` | 4 o | f32 IEEE-754 big-endian |

Cas d'exemple (16 `REAL` + 32 `BOOL` par sens) : 64 + 4 = **68 octets** par sens, très
en deçà des limites de trame RT (~1440 o).

### 5.4 Esquisse d'API publique

```rust
let cfg = DeviceConfig::builder()
    .station_name("edge-reg-01")
    .vendor_id(0x0000)          // ID de TEST en dev — à régulariser (voir §7)
    .device_id(0x0001)
    .send_clock(SendClock::Ms1)
    .input_submodule(Slot(1), &[Field::Real; 16])   // PLC -> edge
    .input_submodule(Slot(2), &[Field::Bool; 32])
    .output_submodule(Slot(3), &[Field::Real; 16])  // edge -> PLC
    .output_submodule(Slot(4), &[Field::Bool; 32])
    .build()?;

let dev = ProfinetDevice::start(cfg, "eth0")?;       // lance threads RT + acyclique

// boucle de régulation (consommateur, HORS lib) :
let pv:  f32     = dev.read_real(Slot(1), 0)?;       // dernière image cohérente
let cmd: bool    = dev.read_bool(Slot(2), 5)?;
dev.write_real(Slot(3), 0, mv)?;                     // publié au prochain cycle
let st:  ArState = dev.ar_state();                   // RUN / Offline / Connecting ...
```

## 6. Flux de données & cycle de vie

1. **Découverte** : l'IO-Controller (TIA) trouve le device via DCP, lui assigne
   nom de station + IP.
2. **Établissement AR** : DCE/RPC `Connect` → écriture des paramètres (records) →
   `Dcontrol`/`Ccontrol` → alarme **Application-Ready** → AR en état **DATA/RUN**.
3. **Échange cyclique** : CPM consomme les sorties du PLC (entrées du device),
   PPM produit les sorties du device, à chaque tick du send clock ; gestion
   IOPS/IOCS et data status.
4. **Supervision** : watchdog consommateur (perte de trames → AR Offline) ;
   reconnexion ; remontée d'alarmes.
5. **Arrêt** : `Release` propre de l'AR.

## 7. Gestion des erreurs & cas limites

- **Timeout d'AR** / refus du Controller → log explicite + retour à l'état Connecting.
- **Watchdog consommateur** : absence de trames CPM au-delà de `cycle × ratio ×
  facteur` → AR Offline, données figées + indicateur d'invalidité exposé à l'appli.
- **Data status** : gestion PRIMARY/BACKUP et bit « problem indicator ».
- **Vendor ID / Device ID** : ID de **test** en dev ; un device réellement déployé
  doit obtenir un **Vendor ID légitime** auprès de PI (sinon risque de collision
  réseau) — à régulariser avant tout déploiement large.

## 8. Stratégie de test

1. **Unitaires par couche** contre des **trames « golden »** capturées à Wireshark
   (parse/serialize DCP, RPC, RT, alarmes) — en **TDD**.
2. **Intégration** : harnais *mock IO-Controller* rejouant un échange capturé ;
   tests ciblés de la machine d'état de l'AR.
3. **Hardware-in-the-loop** : S7-1500 réel + TIA Portal → vérification automatisée
   que l'AR atteint **RUN** et que les valeurs typées font l'aller-retour correctement.
4. **Déterminisme** : mesure de la gigue du cycle sous charge (méthode type
   `cyclictest`) sur l'edge, validation de la tenue du send clock 1 ms.

## 9. Risques & dépendances

| Risque | Impact | Mitigation |
|---|---|---|
| Machine d'état de l'AR (n°1) | Le PLC n'atteint pas RUN | Captures Wireshark + montée incrémentale couche par couche |
| Statuts IOPS/IOCS & data status | Données rejetées par le PLC | Tests golden + HIL tôt |
| Correspondance GSDML ↔ config ingéniérée | Refus de connexion | GSDML d'exemple verrouillé, généré depuis la config |
| Jitter NIC / driver sous Linux | Cycle 1 ms non tenu | NIC propre + `PACKET_MMAP`, isolation cœur, affinité IRQ, PREEMPT_RT |
| Accès norme | Implémentation à l'aveugle | **Achat norme IEC 61158/61784** (en cours côté utilisateur) |

### Dimension propriété intellectuelle / publication

- **Implémenter le protocole** à partir de la norme = légal (clean-room ; un protocole
  n'est pas protégeable, seul le *texte* de la norme l'est → ne pas le recopier).
- **Marque** « PROFINET® » = propriété de PNO, usage réservé aux membres PI ; usage
  **descriptif** autorisé. Pas de logo, pas de « certified ».
- **Brevets** : exposition faible en RT classe 1 / CC-A (pas d'IRT/TSN), non nulle ;
  revue juridique recommandée **avant industrialisation/distribution**.
- **Publication open source** envisageable : crate `profinet-rt` (nom libre sur
  crates.io), **double licence MIT/Apache-2.0**, **disclaimer de non-affiliation** PI,
  **aucun texte de norme** ni code GPL dans le repo. *Rien n'est poussé en public avant
  acquisition de la norme et validation finale du nom.*

## 10. Évolutions possibles (hors périmètre actuel)

- **CC-B** : ajout LLDP + données de topologie (diagnostic réseau enrichi).
- Backend `eth` alternatif (XDP/AF_XDP) pour latence encore plus basse.
- Multi-AR / Shared-Device.
- Cycles plus courts (500/250 µs) selon les mesures de gigue réelles.

## 11. Nommage & livrables

- Projet : `203-profinet-rt` — Crate : **`profinet-rt`**.
- Livrables : la crate, le **GSDML d'exemple** (16 `REAL` + 32 `BOOL`), un binaire de
  démonstration, la documentation de configuration et de déploiement (tuning
  PREEMPT_RT).
