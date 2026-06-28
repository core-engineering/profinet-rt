# Bench — p-net as a real IO-Device (generating AR/RT ground truth)

## Why
PLCSIM Advanced **does not support real-time PROFINET IO** (see `captures/README.md`: Connect
rejected with `nca_unk_if`). Capturing **Connect/AR + cyclic RT + alarm** (Plans 3-5) requires a
**real IO-Device** facing the S7-1500. No ET200 is available → we use the sample app
**`pn_dev` from p-net (rt-labs)** on the **Debian edge** as the test peer.

- **License**: p-net is GPLv3 — acceptable here because it is a **test peer that is not shipped** (nothing
  from p-net enters our crate). It is also an **inspectable reference implementation**.
- **Bonus**: assigning the name via TIA produces a real **DCP-Set** → golden frame for DCP Set
  (deferred to Plan 2).

## Topology (isolated segment, NO CPL/PowerLine)
```
S7-1500 .60  ──┐
   (IO-Controller) dumb switch (or direct cable)   ← never CPL/PowerLine (jitter → RT watchdog)
edge Debian ───┘  pn_dev = IO-Device, NIC e.g. .50
                  capture: tcpdump ON the edge (sees all its own traffic: DCP, AR UDP 34964, RT L2)
```
No port mirroring needed: the edge is the device, so its own interface carries DCP + AR + RT.

## 1. Edge prerequisites (Debian)
```bash
sudo apt update && sudo apt install -y build-essential cmake git tcpdump
```
- **Root** privileges required (raw AF_PACKET sockets + RT priorities). The edge already runs PREEMPT_RT ✅.

## 2. Clone + build p-net
```bash
git clone --recurse-submodules https://github.com/rtlabs-com/p-net.git
cmake -B build -S p-net -DCMAKE_BUILD_TYPE=Release -DUSE_SCHED_FIFO=ON
cmake --build build
```
(`--recurse-submodules` is mandatory: p-net pulls `osal`/`cmake` as submodules.)

## 3. Locate binary, GSDML and options (TO CONFIRM on your clone — version-dependent)
```bash
find build -name 'pn_dev' -type f          # sample app binary
find p-net -name 'GSDML*.xml'              # the GSDML to import into TIA
ls p-net/samples/*/ 2>/dev/null || ls p-net/sample_app/   # sample directory (version-dependent)
sudo ./build/.../pn_dev --help             # EXACT list of flags + default values
```
Expected (verify via `--help`):
- `-i <iface>`: network interface (e.g. `eth0` / `enp1s0`)
- `-s <station-name>`: PROFINET station name (**historical default `rt-labs-dev`** — confirm)
- `-v` / `-vv` / `-vvv`: verbosity
- `-b`, `-d`, `-p`: button file (triggers alarm), state directory, path
- a `set_network_parameters` script is sometimes provided in `build/` to prepare the interface.

## 4. Edge network interface
```bash
sudo ip addr flush dev <iface>
sudo ip addr add 192.168.1.50/24 dev <iface>
sudo ip link set <iface> up
```
(the controller can also (re-)assign the IP via DCP; starting with `.50` is safe.)

## 5. Start the p-net device
```bash
sudo ./build/.../pn_dev -vvv -i <iface> -s rt-labs-dev
```
Leave it running; it waits for the controller to open the AR. The `-vvv` logs show DCP → Connect
→ parameterization → ApplicationReady → cyclic exchange.

## 6. TIA side (the S7 = IO-Controller)
1. **Options → Manage GSD files** → import the **GSDML** found in step 3 → install.
2. In the catalog (Other field devices → PROFINET IO → RT-Labs…), **drag the device** onto the
   S7's IO system and connect it to the `192.168.1.x` network.
3. **PROFINET name** of the device = the station name from step 5 (**`rt-labs-dev`**); **IP** `192.168.1.50`.
4. Place **modules/submodules** according to the GSDML (the sample exposes simple I/O modules,
   e.g. a few bytes in/out — sufficient for the AR/RT structure; no need to match our
   16 REAL + 32 BOOL mapping, we are capturing the **structure** of the protocol).
5. **Compile + download** to the physical S7.
6. **Online → "Assign PROFINET device name"** on the detected device → writes the name on the wire via
   **DCP-Set** (⚠️ start the capture BEFORE: this provides the golden DCP-Set).

→ Transition to RUN: the AR should come up, the device turns **green** in TIA, `pn_dev` logs the cyclic exchange.

## 7. Capture (on the edge) — scenario
```bash
# Connect/AR: start capture THEN (re)launch pn_dev / put S7 back in RUN
sudo tcpdump -i <iface> -w ar-connect.pcapng 'ether proto 0x8892 or vlan or udp port 34964'
# Cyclic : let run 5-10 s in stable RUN                  -> rt-cyclic.pcapng
# Alarm  : trigger (button file -b, or remove/force a module)  -> alarm.pcapng
# DCP-Set: during a TIA "Assign device name"              -> dcp-set.pcapng
```
Copy the `.pcapng` files into `203-profinet-rt/captures/` (not versioned — decoded with
`tshark.exe` via interop to extract the golden frames for Plan 3).

## 8. Next steps
With `ar-connect.pcapng` + `rt-cyclic.pcapng` we can **scope and execute Plan 3 (`cm`/AR)**:
DCE-RPC over UDP 34964, ARBlockReq/IOCRBlockReq/ExpectedSubmodule/AlarmCR blocks, AR state machine
up to DATA — on real ground truth instead of public captures.

## Pitfalls
- **Never use CPL/PowerLine** on the segment (HomePlug `0x88e1` → jitter → RT watchdog expires, AR drops).
- p-net **must run on native Linux with L2 access** (the edge) — **not in WSL2** (NATed network).
- Run `pn_dev` **as root**; verify the interface is not managed by NetworkManager/DHCP
  which would overwrite the IP.
- The p-net device has the **rt-labs Vendor/Device ID** (not ours) — expected: we are capturing the
  **structure** of the frames, identical to what our stack will produce.
