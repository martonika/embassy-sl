# embassy-sl

Embassy on Silicon Labs EFR32 microcontrollers — a Rust HAL, peripheral access crate, and glue for linking against the Simplicity SDK (RAIL, Bluetooth LE, Bluetooth Mesh).

Primary target today: **EFR32MG24** on **BRD4186C** (xG24 Dev Kit). The HAL is structured so additional xG families can be added behind chip features.

## Repository layout

The repo is a Cargo workspace (`Cargo.toml` at the root). Crates fall into three layers: core Rust hardware support, SDK FFI/bindings, and firmware examples.

### Core crates

#### `embassy-silabs`

Embassy HAL for Silabs chips. Provides `embassy-silabs::init()`, peripheral drivers, board helpers, and optional radio stack integration behind Cargo features.

| Feature | Enables |
|---------|---------|
| `xg24` / `board-brd4186c` | EFR32MG24 + BRD4186C defaults |
| `defmt` | Structured logging via RTT |
| `memlcd-driver` | Memory LCD (e.g. on Thunderboard-style boards) |
| `rail` | Proprietary RAIL radio (`silabs-rail-sys` + `silabs-csdk`) |
| `ble` | Bluetooth LE host/controller (`silabs-bluetooth-sys` + RAIL-BLE) |
| `btmesh` | Bluetooth Mesh provisionee (`silabs-btmesh-sys`, requires `ble`) |

Drivers today include GPIO, USART/EUSART, I2C, SPI, timers, IADC, flash, LDMA, CMU/EMU, and watchdog. Module entry points live in `embassy-silabs/src/`.

#### `silabs-pac`

Peripheral access crate generated from the device SVD using [chiptool](https://github.com/embassy-rs/chiptool), with extra transform rules in `transform.yaml`. This is the low-level register API that `embassy-silabs` builds on.

To regenerate after SVD or transform changes, see `silabs-pac/README.md`.

### SDK integration (`*-sys` + C glue)

These crates link prebuilt libraries from a local Simplicity SDK install. They are **not** vendored in git.

#### `silabs-rail-sys`

FFI and linker setup for Silicon Labs RAIL (proprietary and multiprotocol variants). Used for custom PHY experiments and as the radio layer under BLE.

#### `silabs-bluetooth-sys`

FFI to the BLE host stack, link layer, and BGAPI. `build.rs` pulls headers and static archives (`libble_host.a`, `liblinklayer.a`, etc.) from `SILABS_SDK`.

#### `silabs-btmesh-sys`

FFI to the Bluetooth Mesh stack (provisionee node). Depends on BLE + PSA crypto from the SDK.

#### `silabs-csdk`

C sources compiled into the firmware image: clock/power init stubs, RAIL callbacks, BLE platform init, link-layer pump/wrap glue, PendSV integration, and stack configuration headers copied from Simplicity Studio component layouts. Rust code in `embassy-silabs` pulls this in when `rail`, `ble`, or `btmesh` features are enabled.

See `silabs-csdk/README.md` for optional RAIL PHY config (`rail_config.c` / `SILABS_RAIL_CONFIG_DIR`).

### Firmware examples

#### `embassy-silabs-project`

End-to-end HAL demo for BRD4186C: Embassy executor, defmt, memory LCD text/graphics, and Si7021 temperature/humidity over I2C. Use this to verify basic peripherals before touching radio stacks.

```bash
cd embassy-silabs-project
cargo run --release
```

#### `embassy-silabs-bt-empty`

BLE-only bring-up firmware: minimal C runtime glue + Rust `sl_bt_on_event` handler. Targets connectable advertising on BRD4186C using the SDK BLE stack on a bare-metal Embassy pump (no RTOS).

**Status: work in progress** — builds and completes stack init, but RF advertising is not yet visible on air (see docs below).

```bash
export SILABS_SDK=/path/to/simplicity-sdk
cd embassy-silabs-bt-empty
cargo run --release
```

Investigation notes:

- `embassy-silabs-bt-empty/BLE_BRINGUP.md` — init/pump evolution, diagnostics, and open RF issues
- `ble-investigation-handoff.md` — concise handoff for resuming the advertising debug

#### `embassy-silabs-btmesh-project`

Workspace member reserved for a Bluetooth Mesh demo (not yet present in the tree).

### Other paths

| Path | Purpose |
|------|---------|
| `.vscode/` | Editor workspace for the Rust crates |
| `reference_project/` | Local `bt_soc_empty` SDK copy for diffing against Silicon Labs reference apps (**gitignored**, not required to build) |

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
