# BLE stack bring-up on Embassy + EFR32MG24

This document records what was required to get the Silicon Labs BLE host/controller
**running under Embassy Rust** on BRD4186C (EFR32MG24B220F1536IM48).

**Status:** 2026-09-02 · tag `ble-empty-discover-v70` (SDK `sisdk-2026.12`)

**On-air RF confirmed** (v68, rechecked on v70): nRF Connect sees **"Embassy BLE"**.
Root fix: force-link strong `RAIL_BLE_Phy*` (`silabs_rail_ble_phy_force.c` +
`-usilabs_force_rail_ble_phys`). librail weak NULLs left the 39 MHz PHY unloaded →
`config_channel` `INVALID_STATE` (`0x2`) / `start_tx` `INVALID_CALL` (`0xE`).

**v70:** Removed BLE/RAIL diagnostic wraps and all `--wrap=sli_*` hooks. Kept only
functional glue (HCI LTO re-drive, `usch_ScheduleProcess` gating, raise-events wakeup
recovery, PHY force-link). Boot tag only. Advertising still works after cleanup.

Reference target: Silicon Labs `bt_soc_empty` (advertise as **"Embassy BLE"**, connectable
legacy advertising).

---

## Goal

Run the SDK BLE stack from an Embassy async task instead of `sl_main`, with application
logic in Rust (`sl_bt_on_event`) and only minimal C glue for init, pumping, and
link-layer integration.

**Success criteria (met on v68+):**

- Host: `sl_bt_evt_system_boot` processed; adv setup steps 1–4 OK; pump stays alive
- RF: nRF Connect (or equivalent scanner) sees **"Embassy BLE"**
- Connect / GATT: **not done yet**

---

## Project layout

| Piece | Role |
|-------|------|
| `embassy-silabs-bt-empty` | BLE-only app crate (`ble-empty` binary) |
| `embassy-silabs` (`ble` feature) | Thin Rust API: `init_step`, `step`, `INIT_STEPS` |
| `silabs-csdk` (`ble` + `ble-rust-handler`) | Platform init, pump, deferred adv start, linker glue |
| `silabs-bluetooth-sys` | bindgen BGAPI + links SDK host/controller archives |
| `silabs-rail-sys` | RAIL blob + radio IRQ object extraction |
| `silabs-pac` (`rt`) | Vector table / `device.x` / `__INTERRUPTS` |

Main sources:

- `embassy-silabs-bt-empty/src/bin/ble_empty.rs` — Embassy `main`, LED, logging
- `embassy-silabs-bt-empty/src/ble_runtime.rs` — init + `ble_stack_task` loop
- `embassy-silabs-bt-empty/src/ble_app.rs` — Rust `sl_bt_on_event`, adv configuration
- `silabs-csdk/src/ble_platform_init.c` — stepped platform + stack init
- `silabs-csdk/src/sl_bluetooth.c` — pump/event split (`sl_bt_run` + pop event)
- `silabs-csdk/src/silabs_ble_adv_start.c` — deferred `legacy_advertiser_start()`
- `silabs-csdk/src/silabs_linklayer_pump.c` — `ubt_run` wrap, HCI drain, LL events
- `silabs-csdk/src/silabs_ll_hci_call.c` — strong `ll_hciCall` (matches `ll_hci.c`)
- `silabs-csdk/src/silabs_ble_hci_usch.c` — HCI ext-adv wraps + `usch_ScheduleProcess` gating
- `silabs-csdk/src/silabs_bgmessage_stub.c` — non-blocking BG message wait
- `silabs-csdk/src/sl_btctrl_pendsv.c` — `PendSV` → `sl_bt_priority_handle`
- `silabs-csdk/src/silabs_ll_raise_wrap.c` — null/invalid wakeup slot recovery
- `silabs-csdk/src/silabs_rail_ble_phy_force.c` — strong `RAIL_BLE_Phy*` (39 MHz) + `-u`
- `silabs-csdk/src/silabs_radio_irq_vectors.c` — cortex-m-rt → RAIL IRQ bridges
- `reference_project/` — local Silicon Labs `bt_soc_empty` copy for diffing (**not in git**; requires separate SDK checkout)

---

## Architecture (runtime)

```
embassy main
  ├─ embassy_silabs::init()
  ├─ ble_runtime::init_stack()       # 9 steps + mark_initialized + post_stack_pump + adv_start
  └─ ble_runtime::pump_loop()        # startup_ll_pump, then tight step + 5 ms delay
        └─ embassy_silabs::ble::step()
              ├─ silabs_ble_step_app_timer()   → sli_app_timer_step()
              ├─ silabs_ble_step_sleeptimer()  # (empty; sleeptimer uses SYSRTC IRQ)
              └─ silabs_ble_step_bt_host()
                    ├─ silabs_ble_step_bt_pump()           → sl_bt_run()
                    ├─ silabs_ble_step_bt_event()          → pop event → sl_bt_on_event (Rust)
                    ├─ silabs_ble_finish_pending_adv_start() → legacy_advertiser_start() (C)
                    ├─ silabs_ble_hci_drain()
                    ├─ silabs_service_linklayer_events()     → drain ll_events
                    └─ sl_bt_priority_handle()
```

After boot, steady state is `ble::step()` as above. `init_stack()` runs a short
`post_stack_pump` (boot event + pending adv setup), then `pump_finish` calls deferred
`legacy_advertiser_start` with the scheduler gated. `pump_loop()` runs `startup_ll_pump`
before the steady loop.

---

## Build / feature wiring

`embassy-silabs-bt-empty` enables:

```toml
ble-empty = [
  "silabs-csdk/ble",
  "silabs-csdk/ble-rust-handler",
  "silabs-bluetooth-sys/ble",
  "silabs-rail-sys/rail",
  "embassy-silabs/ble",
]
```

Environment:

- `SILABS_SDK` — required; path to the Simplicity SDK root directory
  (current: `/Users/manika/silabs/sisdk-2026.12`)
- Target: `thumbv8m.main-none-eabi` (see `.cargo/config.toml` under this crate)
- Flash/run: `cargo run --release` (probe-rs + `EFR32MG24B220F1536IM48`)
- Source of truth for LL/HCI: `~/silabs/bluetooth-le` (not binaries)
- Source of truth for RAIL: `~/silabs/rail`

### SDK 2026.12 path deltas vs older bring-up notes

| Change | Detail |
|--------|--------|
| `platform/.../service/sl_log/` | **New.** `memory_manager` includes `sli_memory_manager_log.h` → `sl_log_component.h`. `silabs-csdk` adds those include dirs + local `sl_log_common_config.h` with `LEVEL_NONE`. |
| `memory_manager/profiler/` | **Removed.** Dropped `sli_memory_profiler_stubs.c` from `sdk_ble_sources`. |
| `bgapi_protocol/.../protocol/task/` | **New** `libbgapi_task.a` (`sli_bgapi_task_step`, `sli_bgapi_shared_task`, …). Linked from `silabs-bluetooth-sys/build.rs`. |
| `bluetooth_le_host/.../ubt` | Still missing from packaged SDK; host HCI source lives in `~/silabs/bluetooth-le/.../ubt/`. |
| `usch_*` opacity | **Lifted** for source investigation: `bluetooth-le/bluetooth_le_controller/src/scheduler/scheduler.c` has `usch_ScheduleProcess`. |

GATT device name **"Embassy BLE"** comes from generated `gatt_db.c`
(`silabs-csdk/ble/gatt_configuration.btconf`), not from `sl_bt_gap_set_device_name`.

---

## Platform init (9 steps)

Called from `ble_runtime::init_stack()` via `embassy_silabs::ble::init_step(1..=9)`:

| Step | Action |
|------|--------|
| 1 | `sl_memory_init()` |
| 2 | DCDC |
| 3 | HFXO (39 MHz) |
| 4 | Clocks |
| 5 | EMU |
| 6 | Sleeptimer |
| 7 | `sli_bt_stack_permanent_allocation()` |
| 8 | `sl_rail_util_pa_init()`, RAIL PM, PTI |
| 9 | `silabs_bt_stack_functional_init()` → `sli_bt_start_bgapi_device()` |

Init is **split into steps** so Embassy can run it from Rust without blocking inside a
single giant C `sl_main_init()`. The BGAPI device starts at step 9; events are not
processed until the stack task pumps.

---

## Stack pump loop

### Why not call BGAPI start inside `on_event`?

Early attempts matched `bt_soc_empty` and called `sl_bt_legacy_advertiser_start()` **inside**
`sl_bt_on_event` on `system_boot`. That **hung** inside the event handler (HCI / link-layer
not serviced correctly while still inside the BGAPI callback).

**Working pattern (v14+):**

1. In Rust `sl_bt_on_event` on boot: create adv set, generate data, set timing only.
2. Set `PENDING_ADV_START = true`.
3. After `on_event` returns, C `silabs_ble_finish_pending_adv_start()` calls host GAP
   `legacy_advertiser_start(handle, connectable)` directly (not the BGAPI wrapper).

### Pump contents

Each iteration (`ble_runtime` steps 1–3 explicitly, then `ble::step()`):

1. **App timer + sleeptimer** — SDK time bases for stack timers.
2. **`sl_bt_run()`** — BGAPI message pump (`silabs_sl_bt_run_pump`).
3. **Pop one event** — `sli_bgapi_device_pop_event` → `sl_bt_process_event` → Rust handler.
4. **Deferred adv start** — if pending, run `legacy_advertiser_start` + post-start drain.

`ble_runtime` runs the first three iterations with extra logging, then uses `ble::step()`.

---

## Critical linker / runtime glue

These were **required** to stop hangs and get `adv_step=4 sc=0x00`:

### `--wrap=ubt_run` (`silabs_linklayer_pump.c`)

Before each `ubt_run`, call `sli_app_timer_step()`. Keeps host app timers aligned with
Embassy’s manual stepping. (Sleeptimer advances via `SYSRTC_APP` IRQ; no explicit step
function is called from the pump.)

### `--wrap=hci_le_set_extended_advertising_enable` / `_parameters` (`silabs_ble_hci_usch.c`) — **critical v56/v63**

Prebuilt `libble_host.a` (LTO) **skips `ll_hciCall`** between `hci_command_init_shared` and
`hci_command_shared_response`. The wraps re-drive controller HCI so enable and parameters
reach the LL.

Strong `ll_hciCall` in [`silabs_ll_hci_call.c`](../silabs-csdk/src/silabs_ll_hci_call.c) matches
controller `ll_hci.c` (`bluetooth_le_controller/src/ll_hci.c`, `ll_hci.call` mailbox).

### `ll_hciCall` post-service (`silabs_ll_hci_post_service.c` + pump)

After synchronous HCI, the pump calls `silabs_service_linklayer_events()` and
`sl_bt_priority_handle()` so `LL_EVENT_HCI_MESSAGE` / `LL_EVENT_SCHEDULE` are processed.

### `--wrap=bg_message_queue_wait_time` (`silabs_bgmessage_stub.c`)

Return zero wait so `bg_message_run` never blocks waiting on an internal timer when
Embassy owns the main thread.

### `PendSV` → `sl_bt_priority_handle` (`sl_btctrl_pendsv.c`)

cortex-m-rt uses `PendSV` as the vector name on this target (not `PendSV_Handler`).

### Deferred start + schedule gate (`silabs_ble_adv_start.c`, `silabs_ble_hci_usch.c`)

Deferred `legacy_advertiser_start` runs with `usch_ScheduleProcess` gated. `pump_loop`
then calls `silabs_ble_try_real_schedule_once()` and leaves `schedule_allow_real=1`.

### Linker flags in `embassy-silabs-bt-empty/build.rs`

```text
--wrap=sl_btctrl_init_functional
--wrap=ubt_run
--wrap=hci_le_set_extended_advertising_enable
--wrap=hci_le_set_extended_advertising_parameters
--wrap=usch_ScheduleProcess
--wrap=sl_btctrl_raise_events
--wrap=bg_message_queue_wait_time
-usilabs_force_rail_ble_phys
```

Plus `-uMODEM`, `-uMODEM_IRQHandler`, etc. to force-link RAIL radio IRQ handlers.

Do **not** wrap internal `sli_*` symbols or RAIL TX/config APIs for diagnostics.

### `--wrap=sl_btctrl_raise_events` (`silabs_ll_raise_wrap.c`)

Ensures the compatibility wakeup slot is callable before the real raise path. Uses
`&sli_bt_host_adaptation_compatibility_linklayer_wakeup` (not a hardcoded RAM address).

### `silabs_service_linklayer_events()` (`silabs_linklayer_pump.c`)

Must atomically read-clear `ll_events` and call `sl_btctrl_process_events(events)`.

### RAIL / controller alignment vs `bt_soc_empty` (v50)

Reference project uses single-protocol RAIL. Rust build was adjusted to match:

| Setting | Reference | Rust (v50+) |
|---------|-----------|-------------|
| `SL_RAIL_LIB_MULTIPROTOCOL_SUPPORT` | `0` | `0` |
| `SL_RAIL_UTIL_RAIL_POWER_MANAGER_INIT` | `1` | `1` |
| `sl_rail_util_sequencer.c` | linked | linked |
| `sl_rail_util_built_in_phys.c` | linked | linked |
| Multiprotocol `-u ll_radioGetRailSchedulerInfo` | not used | removed |

RAIL `init`/`ble_init` succeed once (`calls init/ble=1/1`), but post-adv `cfg/stx/tx`
counters stay at zero — the link layer never reaches RAIL TX scheduling.

---

## Rust application handler (`ble_app.rs`)

On `sl_bt_evt_system_boot_id`:

1. Log identity address (debug)
2. `sl_bt_advertiser_create_set`
3. `sl_bt_legacy_advertiser_generate_data` (general discoverable)
4. `sl_bt_advertiser_set_timing(160, 160, 0, 0)` — 100 ms interval
5. Set `PENDING_ADV_START` — **do not** call `legacy_advertiser_start` here

Deferred start runs in `silabs_ble_post_stack_init_pump_finish()` after the boot pump
(scheduler gated during init to avoid `usch_ScheduleProcess` hang).

On `sl_bt_evt_connection_closed_id`: regenerate data and set pending again.

Exported to C:

- `silabs_ble_adv_handle_read()`
- `silabs_ble_pending_adv_start_read()` / `_clear()`

---

## Hang history (what failed)

| Attempt | Symptom |
|---------|---------|
| Inline `sl_bt_legacy_advertiser_start` in `on_event` | Hang during boot event |
| Deferred `sl_bt_legacy_advertiser_start` from C after event | Hang in `try_finish` / start |
| Partial `ubt_run` / incomplete pump | Hang in start path |
| No `ll_hciCall` wrap | Hang in HCI-heavy start |
| **`sl_bt_run()` pump + deferred `legacy_advertiser_start` from C + `ll_hciCall` wrap** | **Host reports success** |

Do **not** regress (verified hang or break):

| Pattern | Result |
|---------|--------|
| Inline `sl_bt_legacy_advertiser_start` in `on_event` | Hang |
| Deferred start without `ll_hciCall` wrap | Hang |
| `bg_message_queue_wait_time` not stubbed to 0 | Hang |
| Hardcoded `0x200288d8` in raise-events wrap | Corrupts `timer_processing_ongoing` |
| Calling heavy start APIs from inside `on_event` | Hang |

**Keep** (working):

- Deferred `legacy_advertiser_start` from pump path
- `silabs_ble_post_stack_init_pump(128)` on init thread
- `--wrap=sl_btctrl_raise_events` using correct `compatibility_linklayer_wakeup` address

---

## Radio IRQ / vector table (v17–v18)

Host `adv_step=4` does **not** prove on-air packets. A separate issue was that **RAIL
radio interrupts were not wired**:

1. **`silabs-pac/src/lib.rs`** — `__INTERRUPTS` had `_reserved: 0` for IRQ 30–39 (AGC,
   BUFC, FRC, MODEM, RAC_SEQ, …) instead of handler symbols.
2. **`silabs-pac/device.x`** — incomplete IRQ list vs MG24 header (fixed to 76 entries).
3. **RAIL IRQ handlers** in `librail.a` were not linked until `-u` force symbols.
4. **`silabs_radio_irq_vectors.c`** — naked branches from cortex-m-rt names (`MODEM`) to
   RAIL (`MODEM_IRQHandler`).

Verified on v18: vector slots IRQ29–39 point at wrapper stubs that branch into RAIL.

---

## Debug phase codes (`silabs_ble_step_phase`)

Useful when reading logs:

| Phase | Meaning |
|-------|---------|
| 1–3 | App timer / sleeptimer |
| 11–16 | `sl_bt_run` pump |
| 20–27 | BGAPI event pop / handler |
| 30–42 | Inside Rust boot handler (adv setup) |
| 46–48 | Deferred `legacy_advertiser_start` |
| 44 | Adv start complete, `handler_done=1` |
| 6 | Steady `silabs_ble_step_bt_host` tail |

`adv_step` in logs = high byte of `silabs_bgapi_ble_adv_setup_status` (which setup step
last wrote status); `adv_sc` = low byte status (`0x00` = OK).

---

## Memory

`embassy-silabs-bt-empty/memory.x`:

- SDK heap at bottom of RAM (`_heap_size = 0x28000`)
- `.data`/`.bss` above heap, main stack to RAM top
- NVM3 `.simee` sizing via dummy `DUMMY` region

---

## Comparison to `bt_soc_empty`

| `bt_soc_empty` | `embassy-silabs-bt-empty` |
|----------------|---------------------------|
| `sl_main` tight `sl_bt_step()` loop | Embassy task, 5 ms period |
| Start adv **inside** `on_event` | **Deferred** start after event returns |
| Full autogen component init | Manual 9-step `ble_platform_init` |
| Studio vector table / IRQs | cortex-m-rt + `silabs-pac` + RAIL glue |
| No linker wraps | `ubt_run`, `ll_hciCall`, `bg_message_queue_wait_time` wraps |

---

## Diagnostics removed (v70)

RF/LL/RAIL instrumentation wraps and counters are gone (`silabs_ll_radio_*_wrap`,
`silabs_rail_api_wrap`, `sli_*` wraps, IRQ note counters, periodic `ble rf` logs).
Do not reintroduce `--wrap=sli_*` or RAIL TX/config wraps for debugging.

---

## Current status (v70, SDK 2026.12)

### Build

```bash
cd embassy-silabs-bt-empty
export SILABS_SDK=/Users/manika/silabs/sisdk-2026.12
cargo build --release --bin ble-empty
```

Boot log: `ble empty boot tag=ble-empty-discover-v70`.

### Host + RF — works

| Item | Status |
|------|--------|
| Init / deferred adv / HCI enable | OK |
| One-shot real `usch_ScheduleProcess` then `schedule_allow_real` | OK |
| RAIL TX path | OK after PHY force-link |
| nRF Connect sees **"Embassy BLE"** | Confirmed on v68; rechecked on v70 |
| Connect / GATT | Not started |

### RAIL PHY force-link (the RF root cause)

`librail_efr32xg24_*.a` exports **weak NULL** stubs for `RAIL_BLE_Phy1MbpsViterbi` (etc.).
With typical archive order those NULLs win, so `sl_rail_ble_config_phy_1_mbps` never loads
a channel config → `blePhy` undefined → `sl_rail_ble_config_channel_radio_params` returns
`0x2` (`INVALID_STATE`) → `sl_rail_start_tx` returns `0xE` (`INVALID_CALL`).

Fix: [`silabs_rail_ble_phy_force.c`](../silabs-csdk/src/silabs_rail_ble_phy_force.c)
strongly defines the PHY pointers to 39 MHz configs; `embassy-silabs-bt-empty/build.rs`
passes `-usilabs_force_rail_ble_phys`.

### Next

Connect / GATT bring-up (not started).

See [`ble-investigation-handoff.md`](../ble-investigation-handoff.md) for full timeline.

---

## History (v50–v70 summary)

| Tag | Milestone |
|-----|-----------|
| v50–v53 | RAIL alignment, raise-events RAM fix, RF diagnostics |
| v54 | Reference config parity, `sli_bt_stack_functional_init` |
| v56 | **HCI enable wrap + `ll_hciCall`** |
| v57–v59 | Init pump hang fixes (defer adv, scheduler gating) |
| v60 | Startup LL pump — RF still idle |
| v61 | Build on SDK 2026.12; gated-vs-real usch; forced ScheduleProcess hung |
| v62–v65 | Schedule gating / one-shot real schedule / RTT flood control |
| v66–v67 | Schedule runs; every TX fails `0xE` / cfg `0x2` |
| v68 | Force-link strong `RAIL_BLE_Phy*` — **nRF Connect sees "Embassy BLE"** |
| v69 | Disable tracing logs |
| v70 | Remove BLE/RAIL diag wraps and all `sli_*` wraps — **adv still OK** |

---

## Source-only RF investigation (v54+)

**Method:** Compare a local `bt_soc_empty` reference checkout (not in repo) with Embassy glue.
LL/HCI source of truth: `~/silabs/bluetooth-le` (`ll_hci*.c`, `hci_adv.c`, `scheduler/scheduler.c`).
RAIL source of truth: `~/silabs/rail`. Do **not** disassemble prebuilt archives for investigation.

---

## Quick commands

```bash
cd embassy-silabs-bt-empty
export SILABS_SDK=/Users/manika/silabs/sisdk-2026.12
cargo build --release --bin ble-empty
cargo run --release
```

Build tag at boot: `ble empty boot tag=ble-empty-discover-v70`.

