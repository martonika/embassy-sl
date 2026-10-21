# embassy-sl

Embassy on Silicon Labs EFR32 microcontrollers — a Rust HAL, peripheral access crate, and glue for linking against the Simplicity SDK (RAIL, Bluetooth LE, Bluetooth Mesh).

Primary target today: **EFR32MG24** on **BRD4186C** and **BRD4187C** (xG24 Dev Kit). The HAL is structured so additional xG families and boards can be added behind chip and board features.

## Repository layout

The repo is a Cargo workspace (`Cargo.toml` at the root). Crates fall into three layers: core Rust hardware support, SDK FFI/bindings, and firmware examples.

### Core crates

#### `embassy-silabs`

Embassy HAL for Silabs chips. Provides `embassy-silabs::init()`, peripheral drivers, board helpers, and optional radio stack integration behind Cargo features.

| Feature | Enables |
|---------|---------|
| `xg24` | EFR32MG24 chip family |
| `board-brd4186c` | BRD4186C (+10 dBm, B210) + matching RAIL PA sequencer |
| `board-brd4187c` | BRD4187C (+20 dBm, B220) + matching RAIL PA sequencer |
| `board-brd2601b` | Thunderboard Sense 2 (EFR32MG24B210) |
| `defmt` | Structured logging via RTT |
| `memlcd-driver` | Memory LCD (e.g. on WSTK) |
| `rail` | Proprietary RAIL radio (`silabs-rail-sys` + `silabs-csdk`) |
| `ble` | Bluetooth LE host/controller (`silabs-bluetooth-sys` + RAIL-BLE) |
| `btmesh` | Bluetooth Mesh provisionee (`silabs-btmesh-sys`, requires `ble`) |

Drivers today include GPIO, USART/EUSART, I2C, SPI, timers, IADC, flash, LDMA, CMU/EMU, and watchdog. Module entry points live in `embassy-silabs/src/`.

#### `silabs-pac`

Peripheral access crate generated from the device SVD using [chiptool](https://github.com/embassy-rs/chiptool), with extra transform rules in `transform.yaml`. This is the low-level register API that `embassy-silabs` builds on.

To regenerate after SVD or transform changes, see `silabs-pac/README.md`.

### SDK integration (`*-sys` + C glue)

These crates link prebuilt libraries from a local Simplicity SDK install. They are **not** vendored in git.

| Crate | Role |
|-------|------|
| `silabs-rail-sys` | RAIL FFI and linker setup |
| `silabs-bluetooth-sys` | BLE host/controller BGAPI and static archives |
| `silabs-btmesh-sys` | Bluetooth Mesh stack |
| `silabs-csdk` | C platform init, RAIL/BLE glue, stack config headers |

See each crate's `README.md` for details. RAIL PHY config is optional via `silabs-csdk/rail_config/` or `SILABS_RAIL_CONFIG_DIR`.

### Firmware examples

#### `embassy-silabs-project`

HAL demo for BRD4186C: Embassy executor, defmt, memory LCD, and Si7021 temperature/humidity over I2C.

```bash
cd embassy-silabs-project
cargo run --release
```

#### `embassy-silabs-bt-empty`

BLE advertising + memory LCD on BRD4186C/BRD4187C using the SDK BLE stack with an Embassy async pump and Rust `sl_bt_on_event` handler. See [`embassy-silabs-bt-empty/README.md`](embassy-silabs-bt-empty/README.md) for build instructions per board.

#### `embassy-silabs-btmesh-project`

Workspace member reserved for a Bluetooth Mesh demo (not yet present in the tree).

### Other paths

| Path | Purpose |
|------|---------|
| `.vscode/` | Editor workspace for the Rust crates |
| `reference_project/` | Local `bt_soc_empty` SDK copy for diffing (**gitignored**, not required to build) |

## Building

**Toolchain:** `thumbv8m.main-none-eabi` (Cortex-M33), `arm-none-eabi-gcc` / `arm-none-eabi-ar` for C/SDK objects.

**SDK:** Set `SILABS_SDK` to the root of your Simplicity SDK checkout before building anything that touches RAIL or BLE:

```bash
export SILABS_SDK=/path/to/simplicity-sdk
```

From the repo root:

```bash
cargo build --release
```

Or build a single crate, e.g. `cargo build -p embassy-silabs-project --release`.

RAIL and BLE binary blobs ship under the SDK license (MSLA); this repository only contains the Rust/C glue that links them.

## License

MIT OR Apache-2.0 (see `LICENSE`). Silicon Labs SDK libraries remain under their separate license when linked at build time.
