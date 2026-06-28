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
- ✅ **RÉSOLU (merge ba63901)** — **`CaptureError` typé** : `Io(#[from] std::io::Error)` +
  `Pcap(#[from] pcap_file::PcapError)` + `UnknownFormat([u8;4])`. **`PcapFrames` lit pcap ET
  pcapng** (auto-détection magic) et l'itérateur renvoie `Result<Vec<u8>, CaptureError>`
  (plus de swallow). Reste : appliquer le même traitement à **`TransportError::Io(String)`**
  (module `eth`) pour la cohérence inter-modules — NON fait.

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

### ✅ RÉSOLU (merge ba63901) — durcissement dcp
- **Over-response Identify corrigée** : `IdentifyFilter` classe désormais NameOfStation /
  AllSelector (0xff,0xff) / autres filtres ; `handle_dcp_frame` ne répond que sur match
  confirmable (nom qui matche, ou AllSelector explicite) et **jamais** si un filtre non reconnu
  est présent.
- **Minors soldés** : `DcpError::BadFrameId` retiré ; `pub use` re-exports au niveau `dcp::`
  (dont `DCP_MULTICAST_MAC`) ; gardes `debug_assert!` overflow dans `block.rs` ; couverture
  ajoutée (`to_u16`, erreurs `from_u8`, branche `TooShort`, empty-identify, AllSelector).

### Reste ouvert
- **`DeviceRole` encodé en u16** (role+reserved) — byte-exact vs golden (role=0) ; revérifier
  si role≠0 sur un device réel.

### Politique d'erreur RX (recommandation revue)
- `handle_dcp_frame` renvoie `Err` sur trame malformée/courte ; une vraie boucle RX doit
  **logger+drop** plutôt que propager. À documenter côté appelant (Plan 3/4).
