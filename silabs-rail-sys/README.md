# silabs-rail-sys

Low-level FFI bindings to Silicon Labs RAIL for EFR32xG24.

## Requirements

- Simplicity SDK install (`SILABS_SDK` required)
- `arm-none-eabi-gcc` for linking the proprietary RAIL `.a` blob

## Usage

```toml
[dependencies]
silabs-rail-sys = { path = "../silabs-rail-sys", default-features = false, features = ["rail"] }
```

```bash
export SILABS_SDK=/path/to/simplicity-sdk
cargo build --target thumbv8m.main-none-eabi
```

## Features

- `rail` — link `librail_efr32xg24_gcc_release.a` (proprietary PHY)
- `rail-multiprotocol` — link `librail_multiprotocol_efr32xg24_gcc_release.a` (BLE coexistence)

## Licensing

RAIL binary blobs are distributed under Silicon Labs MSLA. They are **not** included in this repository; the build script reads them from your local SDK install.
