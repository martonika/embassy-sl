# embassy-silabs-bt-empty

BLE + memory-LCD bring-up on **BRD4186C / BRD4187C** (EFR32MG24) using Embassy and
Silicon Labs SDK stack libraries.

## Status (build tag `ble-empty-lcd-v76`)

| Area | Status |
|------|--------|
| **Build** | OK (`cargo build --release`) |
| **BLE advertising** | OK — nRF Connect sees **"Embassy BLE"** |
| **Memory LCD** | **Not working** — see [BLE_BRINGUP.md](BLE_BRINGUP.md#memory-lcd-bring-up-v76) |

Runtime uses an async Embassy task pump (`ble::step()` every 5 ms), Rust `sl_bt_on_event`,
and a separate display task for the WSTK LS013B7DH03 panel.

## Run

```bash
export SILABS_SDK=/path/to/sisdk-release   # required
cd embassy-silabs-bt-empty
cargo run --release
```

Boot log should include `ble empty boot tag=ble-empty-lcd-v76` and
`display spi ref=19000000 Hz` (HFRCODPLL, not 39 MHz).

## Docs

- [BLE_BRINGUP.md](BLE_BRINGUP.md) — full BLE glue history, architecture, LCD investigation

## Notes

- Target/chip: `thumbv8m.main-none-eabi` / `EFR32MG24B220F1536IM48` (`.cargo/config.toml`)
- Reference LCD app on the same board: `embassy-silabs-project`
