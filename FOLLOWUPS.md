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

## Doc
- **Contrat de `recv`** : documenter au niveau du trait `EthTransport` les cas légitimes de
  `Ok(None)` (file vide pour le mock ; pas de trame avant timeout ; trame non-PROFINET pour
  le backend) pour un modèle mental partagé.
