# embassy-silabs-bt-empty

BLE-only bring-up project for BRD4186C (EFR32MG24) using Embassy + Silicon Labs stack libraries.

## Status

This project is **work in progress** and **does not work currently**.

- It builds successfully (`tag=ble-empty-sdk-pump-v2`).
- Runtime uses SDK-style `sl_bt_run()` + `sl_bt_priority_handle()` with Rust-owned `sl_bt_on_event`.
- Connectable advertising is still under investigation.

## Purpose

This crate exists as a clean BLE-focused baseline separated from mesh experiments.

Current direction:
- Keep minimal C runtime glue for stack startup/pumping.
- Move BLE application logic (event handling + advertising flow) into Rust via FFI.

## Run

```bash
cargo run --release
```

## Notes

- Default target/chip settings are in `.cargo/config.toml`.
- The build links against Silicon Labs SDK libraries and expects `SILABS_SDK` to be available.
