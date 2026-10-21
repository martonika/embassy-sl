# silabs-csdk

C support code compiled from the Simplicity SDK for RAIL on EFR32MG24.

## Requirements

- `SILABS_SDK` pointing at your SDK tree (required)
- `arm-none-eabi-gcc` and `arm-none-eabi-ar`

## Proprietary radio config (optional)

For proprietary PHY (Simplicity Studio `rail_soc_simple_trx`), export `rail_config.c` / `rail_config.h` and either:

- Place them in `silabs-csdk/rail_config/`, or
- Set `SILABS_RAIL_CONFIG_DIR` to the directory containing those files

Without a custom config, the default build uses the built-in IEEE 802.15.4 2.4 GHz PHY (39 MHz HFXO).
