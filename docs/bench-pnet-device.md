# Banc — p-net comme IO-Device réel (génération de la vérité-terrain AR/RT)

## Pourquoi
PLCSIM Advanced **ne fait pas de PROFINET IO temps réel** (cf. `captures/README.md` : Connect
rejeté `nca_unk_if`). Pour capturer **Connect/AR + RT cyclique + alarme** (Plans 3-5) il faut un
**vrai IO-Device** face au S7-1500. Camille n'a pas d'ET200 → on utilise l'appli d'exemple
**`pn_dev` de p-net (rt-labs)** sur l'**edge Debian** comme pair de test.

- **Licence** : p-net est GPLv3 — OK ici car c'est un **pair de test qu'on ne livre pas** (rien
  de p-net n'entre dans notre crate). C'est aussi une **implémentation de référence** inspectable.
- **Bonus** : assigner le nom via TIA produit un vrai **DCP-Set** → golden frame pour le Set DCP
  (différé au Plan 2).

## Topologie (segment isolé, SANS CPL)
```
S7-1500 .60  ──┐
   (IO-Controller) dumb switch (ou câble direct)   ← jamais de CPL (jitter → watchdog RT)
edge Debian ───┘  pn_dev = IO-Device, NIC ex. .50
                  capture : tcpdump SUR l'edge (il voit tout son propre trafic : DCP, AR UDP 34964, RT L2)
```
Pas besoin de port-mirroring : l'edge étant le device, sa propre interface porte DCP + AR + RT.

## 1. Prérequis edge (Debian)
```bash
sudo apt update && sudo apt install -y build-essential cmake git tcpdump
```
- Droits **root** requis (sockets raw AF_PACKET + priorités RT). L'edge est déjà PREEMPT_RT ✅.

## 2. Cloner + builder p-net
```bash
git clone --recurse-submodules https://github.com/rtlabs-com/p-net.git
cmake -B build -S p-net -DCMAKE_BUILD_TYPE=Release -DUSE_SCHED_FIFO=ON
cmake --build build
```
(`--recurse-submodules` est indispensable : p-net tire `osal`/`cmake` en sous-modules.)

## 3. Repérer binaire, GSDML et options (À CONFIRMER sur ton clone — version-dépendant)
```bash
find build -name 'pn_dev' -type f          # binaire de l'appli d'exemple
find p-net -name 'GSDML*.xml'              # le GSDML à importer dans TIA
ls p-net/samples/*/ 2>/dev/null || ls p-net/sample_app/   # dossier sample (selon version)
sudo ./build/.../pn_dev --help             # liste EXACTE des flags + valeurs par défaut
```
Attendu (à vérifier via `--help`) :
- `-i <iface>` : interface réseau (ex. `eth0` / `enp1s0`)
- `-s <station-name>` : nom de station PROFINET (**défaut historique `rt-labs-dev`** — confirmer)
- `-v` / `-vv` / `-vvv` : verbosité
- `-b`, `-d`, `-p` : fichier bouton (déclenche alarme), répertoire d'état, chemin
- un script `set_network_parameters` est parfois fourni dans `build/` pour préparer l'interface.

## 4. Interface réseau edge
```bash
sudo ip addr flush dev <iface>
sudo ip addr add 192.168.1.50/24 dev <iface>
sudo ip link set <iface> up
```
(le contrôleur peut aussi (ré)assigner l'IP par DCP ; partir avec `.50` est sûr.)

## 5. Lancer le device p-net
```bash
sudo ./build/.../pn_dev -vvv -i <iface> -s rt-labs-dev
```
Laisse-le tourner ; il attend que le contrôleur ouvre l'AR. Les logs `-vvv` montrent DCP → Connect
→ paramétrage → ApplicationReady → échange cyclique.

## 6. Côté TIA (le S7 = IO-Controller)
1. **Options → Gérer les fichiers GSD** → importer le **GSDML** trouvé à l'étape 3 → installer.
2. Dans le catalogue (Other field devices → PROFINET IO → RT-Labs…), **glisser le device** sur le
   système IO du S7, le relier au réseau `192.168.1.x`.
3. **Nom PROFINET** du device = la station de l'étape 5 (**`rt-labs-dev`**) ; **IP** `192.168.1.50`.
4. Placer les **modules/sous-modules** selon le GSDML (le sample expose des modules d'E/S simples,
   ex. quelques octets in/out — suffisant pour la structure AR/RT ; pas besoin de coller à notre
   mapping 16 REAL + 32 BOOL, on capture la **structure** du protocole).
5. **Compiler + télécharger** dans le S7 physique.
6. **Online → « Assign PROFINET device name »** sur le device détecté → met le nom sur le fil par
   **DCP-Set** (⚠️ lance la capture AVANT : ça donne le golden DCP-Set).

→ Mise en RUN : l'AR doit monter, le device passe **vert** dans TIA, `pn_dev` logue l'échange cyclique.

## 7. Capture (sur l'edge) — scénario
```bash
# Connect/AR : démarre la capture, PUIS (re)lance pn_dev / repasse le S7 en RUN
sudo tcpdump -i <iface> -w ar-connect.pcapng 'ether proto 0x8892 or vlan or udp port 34964'
# Cyclique : laisser tourner 5-10 s en RUN stable        -> rt-cyclic.pcapng
# Alarme   : déclencher (fichier bouton -b, ou retirer/forcer un module)  -> alarm.pcapng
# DCP-Set  : pendant un « Assign device name » TIA        -> dcp-set.pcapng
```
Copie les `.pcapng` dans `203-profinet-rt/captures/` (non versionnés — je les décode avec
`tshark.exe` via l'interop et j'en extrais les golden frames pour le Plan 3).

## 8. Ensuite
Avec `ar-connect.pcapng` + `rt-cyclic.pcapng` je peux **cadrer et exécuter le Plan 3 (`cm`/AR)** :
DCE-RPC sur UDP 34964, blocs ARBlockReq/IOCRBlockReq/ExpectedSubmodule/AlarmCR, machine d'état AR
jusqu'à DATA — sur vérité-terrain réelle au lieu de captures publiques.

## Pièges
- **Jamais de CPL** sur le segment (HomePlug `0x88e1` → jitter → watchdog RT expire, AR retombe).
- p-net **doit tourner sur Linux natif avec accès L2** (l'edge) — **pas dans WSL2** (réseau NATé).
- Lancer `pn_dev` **en root** ; vérifier que l'interface n'est pas gérée par NetworkManager/DHCP
  qui réécrirait l'IP.
- Le device p-net a le **Vendor/Device ID rt-labs** (pas le nôtre) — normal, on capture la
  **structure** des trames, identique à ce que notre pile devra produire.
