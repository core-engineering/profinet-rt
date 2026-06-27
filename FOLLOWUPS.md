# Suivis tracés (issus de la revue de branche Plan 1)

Findings non bloquants pour le Plan 1, à intégrer dans les briefs des plans concernés.

## Pour le Plan 4 (`rt` cyclique / thread RT)
- **Filtrage noyau & busy-spin** : `AfPacketTransport::recv` ouvre en `ETH_P_ALL` et
  renvoie `Ok(None)` pour toute trame non-PROFINET → une boucle de polling naïve peut
  tourner à vide sur le trafic broadcast. Installer un filtre BPF (`SO_ATTACH_FILTER`)
  ou binder avec `sll_protocol = htons(0x8892)` pour que le noyau ne réveille `recv` que
  sur les trames PROFINET. Couplé au point `sll_protocol` (un seul knob, dans `open`).
- **`recv` timeout** : le paramètre `_timeout` n'est pas implémenté (via `SO_RCVTIMEO`
  ou `poll`). À implémenter quand la boucle RT en aura besoin.
- **MSG_TRUNC** : `recv` ne gère pas MSG_TRUNC (non-issue pour trames RT standard ≤1522).

## Pour le Plan 2 (`dcp`) — avant comparaisons frame-exact
- **`CaptureError` typé** : remplacer `Io(String)`/`Pcap(String)` par des sources typées
  (`#[from] std::io::Error`, `#[from] pcap_file::PcapError`). Appliquer le même traitement
  à `TransportError::Io(String)` pour la cohérence inter-modules.
- **`PcapFrames::next()` non silencieux** : aujourd'hui `Some(Err(_)) => None` confond EOF
  propre et pcap corrompu → un fichier tronqué passe pour une capture courte valide.
  Surfacer l'erreur (champ `last_error` ou variante) avant d'utiliser le harnais pour des
  assertions frame-exact.

## Pour le Plan 6 (`config` / GSDML / API typée)
- **Valider l'ordre des bits BOOL (LSB-first)** : `data::get_bit`/`set_bit` packent le bit
  `i` → octet `i/8`, masque `1 << (i % 8)` (convention Siemens `byte.0` = LSB). Choix dérivé
  de l'adressage TIA mais **non vérifié sur le fil**. Avant le premier échange cyclique réel,
  confronter à une **capture S7-1500** et au **GSDML d'exemple** (16 REAL + 32 BOOL) que le
  mapping déclaration→(octet, bit) ET l'ordre des bits coïncident. Ajouter un vecteur de test
  issu de la capture.
- **`data::Value` en attente d'usage** : l'enum `Value` est une déclaration anticipée (aucun
  constructeur/consommateur pour l'instant). Le Plan 6 doit soit le câbler (dispatch typé
  `encode(Value)->bytes` / `decode(FieldType,&[u8])->Value`), soit le retirer (YAGNI).
- **Cohérence de nommage `Field`/`FieldType`** : l'esquisse d'API de la spec (§5.4) utilise
  `Field::Real`, le code utilise `FieldType::Real`. À réconcilier au Plan 6.

## Doc
- **Contrat de `recv`** : documenter au niveau du trait `EthTransport` les cas légitimes de
  `Ok(None)` (file vide pour le mock ; pas de trame avant timeout ; trame non-PROFINET pour
  le backend) pour un modèle mental partagé.

## Pour les plans DCP ultérieurs (issus de la revue de branche Plan dcp)

### Important — réponse Identify trop large (à traiter avant un segment multi-device)
- `dcp::identify::parse_identify_request` ne lit que le bloc **NameOfStation** (2,2). Un Identify
  *par DeviceID/IP/alias* ciblant un AUTRE device n'a pas de bloc nom → `name_of_station = None`
  → `handle_dcp_frame` répond quand même (over-response). OK en mono-device (banc actuel), à
  corriger avant multi-device : matcher TOUS les blocs filtres, ou être conservateur (si un bloc
  filtre non-nom est présent et que le nom ne matche pas → `Ok(None)`). Cohérent avec la portée
  planifiée de `IdentifyFilter` ; décision de priorisation = Camille.

### Minor (revue de branche, non bloquants)
- `DcpError::BadFrameId` défini mais jamais construit (FrameID inconnu → `Ok(None)` via `_`). À
  câbler ou retirer.
- `DCP_MULTICAST_MAC` (frame.rs) et le futur chemin d'émission pas encore utilisés ; pas de
  `pub use` au niveau `dcp::` (la spec mentionnait des re-exports).
- `block_len as u16` (block.rs) sans garde overflow (inoffensif < MTU ; `debug_assert!` possible).
- `DeviceRole` encodé en u16 (role+reserved) — byte-exact vs golden (role=0) ; revérifier si role≠0.
- Couverture : `to_u16` GetSet/Hello, chemins d'erreur `from_u8`/`from_u16`, branche `TooShort`
  bloc tronqué — non testés (arms triviaux).

### Politique d'erreur RX (recommandation revue)
- `handle_dcp_frame` renvoie `Err` sur trame malformée/courte ; une vraie boucle RX doit
  **logger+drop** plutôt que propager. À documenter côté appelant (Plan 3/4).
