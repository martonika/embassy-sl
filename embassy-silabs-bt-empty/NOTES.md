# embassy-silabs-bt-empty — development notes

Technical notes from bringing up the Silicon Labs BLE stack under Embassy on EFR32MG24.
For a quick overview and build commands, see [README.md](README.md).

## Board and RAIL PA

BRD4186C and BRD4187C share the same WSTK pinout but ship different radio modules:

| Board feature | Board | Part | PA | RAIL sequencer |
|---------------|-------|------|----|----------------|
| `board-brd4186c` | BRD4186C | EFR32MG24B210F1536IM48 | +10 dBm | `RAIL_SEQ_IMAGE_PA_10_DBM` |
| `board-brd4187c` | BRD4187C | EFR32MG24B220F1536IM48 | +20 dBm | `RAIL_SEQ_IMAGE_PA_20_DBM` |

The board Cargo feature propagates `mg24-b210` (or its absence for B220) into `silabs-csdk`, `silabs-bluetooth-sys`, and `silabs-rail-sys`. **The probe-rs chip in `.cargo/config.toml` must match the board.**

A mismatched PA profile (e.g. B220 firmware on a B210 module) can show `adv_step=4` in logs while producing no on-air packets and `scan_req=0`.

## Architecture

```
embassy main
  ├─ embassy_silabs::init()           # SYSRTC time driver (SYSRTC_APP IRQ)
  ├─ ble_runtime::init_stack()        # sync: 9 init steps + post-adv pump
  ├─ MemLcd::new (EUSART1)
  ├─ spawn extcomin_task_owned        # PC06 @ 60 Hz
  ├─ spawn display_task               # power_on, draw, flush
  ├─ spawn blink_led
  └─ spawn ble_pump_task              # ble::step every 5 ms

ble_pump_task → embassy_silabs::ble::step()
  ├─ silabs_ble_step_app_timer()
  ├─ silabs_ble_step_sleeptimer()     # stub; see SYSRTC section
  └─ silabs_ble_step_bt_host()
        ├─ sl_bt_run()
        ├─ pop event → sl_bt_on_event (Rust)
        ├─ silabs_ble_finish_pending_adv_start()
        ├─ silabs_ble_hci_drain()
        ├─ silabs_service_linklayer_events()
        └─ sl_bt_priority_handle()
```

`init_stack()` runs synchronously in `main` and completes advertising setup before async tasks start.

## Platform init (9 steps)

| Step | Action |
|------|--------|
| 1 | `sl_memory_init()` |
| 2 | DCDC |
| 3 | HFXO (39 MHz) |
| 4 | Clocks — SYSCLK→HFXO; **EM01GRPCCLK stays HFRCODPLL** (display SPI) |
| 5 | EMU |
| 6 | Sleeptimer |
| 7 | `sli_bt_stack_permanent_allocation()` |
| 8 | `sl_rail_util_pa_init()`, RAIL PM, PTI |
| 9 | `silabs_bt_stack_functional_init()` |

## Deferred advertising start

Do **not** call `sl_bt_legacy_advertiser_start` inside `sl_bt_on_event` on boot — it hangs.

Working pattern:

1. In Rust `on_event`: create adv set, generate data, set timing.
2. Set `PENDING_ADV_START = true`.
3. After `on_event` returns, C `silabs_ble_finish_pending_adv_start()` calls `legacy_advertiser_start` from the pump path.

## Critical linker glue

Required `--wrap` flags (see `build.rs`):

```text
--wrap=sl_btctrl_init_functional
--wrap=ubt_run
--wrap=hci_le_set_extended_advertising_enable
--wrap=hci_le_set_extended_advertising_parameters
--wrap=hci_le_set_extended_advertising_data
--wrap=usch_ScheduleProcess
--wrap=sl_btctrl_raise_events
--wrap=bg_message_queue_wait_time
-usilabs_force_rail_ble_phys
```

Plus `-uMODEM`, `-uMODEM_IRQHandler`, etc. to force-link RAIL radio IRQ handlers.

### RAIL BLE PHY force-link

`librail_efr32xg24_*.a` exports weak NULL `RAIL_BLE_Phy*` stubs. Without strong overrides the 39 MHz PHY is never loaded → `config_channel` `INVALID_STATE` / `start_tx` `INVALID_CALL`. Fix: `silabs_rail_ble_phy_force.c` + `-usilabs_force_rail_ble_phys`.

### HCI extended advertising wraps

Prebuilt `libble_host.a` (LTO) can skip `ll_hciCall` between HCI init and response. The wraps in `silabs_ble_hci_usch.c` re-drive controller HCI for enable, parameters, and advertising data.

### `ubt_run` wrap

Calls `sli_app_timer_step()` before each `ubt_run` so host app timers stay aligned with Embassy's manual stepping.

### `bg_message_queue_wait_time` stub

Return zero so `bg_message_run` never blocks on an internal timer.

### `PendSV` → `sl_bt_priority_handle`

cortex-m-rt uses `PendSV` as the vector name on this target.

### Radio IRQ vectors

`silabs-pac` vector table must expose IRQ 30–39 (AGC, MODEM, RAC, …). `silabs_radio_irq_vectors.c` bridges cortex-m-rt names to RAIL `*_IRQHandler` symbols.

## SYSRTC and sleeptimer

Embassy `_time-driver` owns **SYSRTC_APP**. Do **not** force-link `SYSRTC_APP` / `SYSRTC_APP_IRQHandler` — duplicate symbols break Embassy timers.

For `ble-rust-handler` builds, link `sl_sleeptimer_stub.c` instead of the SDK `sl_sleeptimer_hal_sysrtc.c`. The SDK HAL re-inits SYSRTC_APP during BLE init step 6 and steals the peripheral from Embassy, which breaks async `Timer`/`Ticker` (display task never reaches `power_on()`, BLE pump stalls).

`silabs_ble_step_sleeptimer()` calls `silabs_sleeptimer_step()` to advance the stub from the BLE pump.

## Memory LCD

WSTK **LS013B7DH03** on EUSART1: PC01 MOSI, PC03 CLK, PC08 CS, PC09 DISP_ENABLE, PC06 EXTCOMIN.

- `vcom_enable` (PB00) must be held high for the WSTK board controller.
- `init_stack()` must complete before display init (SDK clock/GPIO setup).
- Display SPI `ref_freq_hz` must reflect EM01GRPCCLK on HFRCODPLL (~19 MHz), not 39 MHz HFXO. `sl_device_init_clocks.c` skips moving EM01GRPCCLK to HFXO.
- Spawn display and extcomin tasks; do not drop `MemLcd` at end of `main`.

Working reference without BLE: `embassy-silabs-project`.

## Debug phase codes

| Phase | Meaning |
|-------|---------|
| 1–3 | App timer / sleeptimer |
| 11–16 | `sl_bt_run` pump |
| 20–27 | BGAPI event pop / handler |
| 30–42 | Inside Rust boot handler (adv setup) |
| 46–48 | Deferred `legacy_advertiser_start` |
| 6 | Steady `silabs_ble_step_bt_host` tail |

`adv_step` = high byte of `silabs_bgapi_ble_adv_setup_status`; `adv_sc` = low byte (`0x00` = OK). `adv_step=4` with `adv_sc=0` means the host reports success but does not prove on-air TX without `scan_req` incrementing.

## Patterns that hang or break

| Pattern | Result |
|---------|--------|
| Inline `sl_bt_legacy_advertiser_start` in `on_event` | Hang |
| Deferred start without HCI wraps | Hang |
| `bg_message_queue_wait_time` not stubbed to 0 | Hang |
| Hardcoded wakeup slot address in raise-events wrap | Corrupts `timer_processing_ongoing` |
| SDK `sl_sleeptimer_hal_sysrtc.c` with Embassy time driver | Async timers stop |
| Wrong board PA / part for physical module | No on-air advertising |
| `--wrap=sli_*` or RAIL TX/config diagnostic wraps | Breaks working build |

## Memory layout

`memory.x`: SDK heap at bottom of RAM (`_heap_size = 0x28000`), `.data`/`.bss` above, main stack to RAM top.

## Comparison to `bt_soc_empty`

| `bt_soc_empty` | `embassy-silabs-bt-empty` |
|----------------|---------------------------|
| `sl_main` + `sl_bt_step()` loop | Embassy async pump, 5 ms |
| Start adv inside `on_event` | Deferred start after event returns |
| Studio autogen init | Manual 9-step `ble_platform_init` |
| Studio vector table | cortex-m-rt + `silabs-pac` + RAIL glue |
| No linker wraps | `ubt_run`, HCI, `bg_message_queue_wait_time` wraps |

## Source trees (outside this repo)

| Topic | Location |
|-------|----------|
| BLE host / LL / HCI | `~/silabs/bluetooth-le` |
| RAIL | `~/silabs/rail` |
| Platform / clocks | Simplicity SDK `platform_core` |
| Reference app | `reference_project/` (gitignored local copy of `bt_soc_empty`) |
