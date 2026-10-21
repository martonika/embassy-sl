# embassy-silabs-bt-empty

BLE advertising and memory LCD on **BRD4186C** / **BRD4187C** (EFR32MG24) using Embassy and the Silicon Labs BLE stack.

## Status

| Area | Status |
|------|--------|
| BLE connectable advertising | Working — scanner sees **"Embassy BLE"** |
| Memory LCD (LS013B7DH03) | Working — device name, MAC, "advertising" |
| LED0 blink | Working |
| Connect / GATT | Not implemented |

Runtime uses an async Embassy BLE pump (`ble::step()` every 5 ms), Rust `sl_bt_on_event`, and separate display and EXTCOMIN tasks.

## Prerequisites

```bash
export SILABS_SDK=/path/to/simplicity-sdk
```

Target: `thumbv8m.main-none-eabi`. Flash via probe-rs (see `.cargo/config.toml`).

## Build for a board

Board selection is a Cargo feature. It also picks the matching MG24 part and RAIL PA sequencer image (+10 dBm vs +20 dBm).

### BRD4186C (default, +10 dBm / EFR32MG24B210)

```bash
cd embassy-silabs-bt-empty
cargo build --release
cargo run --release
```

Set probe chip in `.cargo/config.toml` to `EFR32MG24B210F1536IM48`.

### BRD4187C (+20 dBm / EFR32MG24B220)

```bash
cd embassy-silabs-bt-empty
cargo build --release --no-default-features --features ble-empty,board-brd4187c,lcd
cargo run --release --no-default-features --features ble-empty,board-brd4187c,lcd
```

Set probe chip in `.cargo/config.toml` to `EFR32MG24B220F1536IM48`.

Boot log confirms the board:

```
ble empty board=BRD4186C part=EFR32MG24B210F1536IM48 pa=10dBm
```

## Docs

- [NOTES.md](NOTES.md) — BLE/LCD bring-up details, linker glue, and pitfalls
