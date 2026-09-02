# BLE stack bring-up on Embassy + EFR32MG24

This document records what was required to get the Silicon Labs BLE host/controller
**running under Embassy Rust** on BRD4186C (EFR32MG24B220F1536IM48).

**Resumed:** 2026-09-02 · tag `ble-empty-discover-v60` (RAIL HW path proven; BLE RF still open)

Host-level init and HCI→link-layer **advertising enable** work (`adv4=0x4000000`, `ll_en=2`, `add_task=1`).
BLE **on-air advertising is not confirmed** — `usch` stuck at 1/1, `ll_tx=0`, RAIL `cfg/stx/tx=0`
after extended pumping. Standalone RAIL CW + packet TX **do** work (see below), so the BLE stall
is **above RAIL** (HCI/LL/scheduler), not clocks/PA/IRQs/hardware TX.

Reference target: Silicon Labs `bt_soc_empty` (advertise as **"Embassy BLE"**, connectable
legacy advertising).

---

## Goal

Run the SDK BLE stack from an Embassy async task instead of `sl_main`, with application
logic in Rust (`sl_bt_on_event`) and only minimal C glue for init, pumping, and
link-layer integration.

**Success criterion for “loop running”** (host-level, not RF confirmed):

- `sl_bt_evt_system_boot` is processed
- Advertising setup steps 1–3 succeed in `sl_bt_on_event`
- Deferred `legacy_advertiser_start()` returns `SL_STATUS_OK` (`adv_step=4 sc=0x00`)
- `handler_done=1` and the main loop stays in phase 6 without hanging

Example log tail:

```
ble step 1 end ... adv_step=4 adv_sc=0x00 handler_done=1
ble step 6 end ... phase 6 -> 6 ... handler_done=1
BLE advertising active: ... adv_step=4 sc=0x00 scan_req=0
```

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
- `silabs-csdk/src/silabs_ll_dispatch_diag.c` — HCI adv enable wrap + `usch_*` diagnostics
- `silabs-csdk/src/silabs_bgmessage_stub.c` — non-blocking BG message wait
- `silabs-csdk/src/sl_btctrl_pendsv.c` — `PendSV` → `sl_bt_priority_handle`
- `silabs-csdk/src/silabs_ll_raise_wrap.c` — raise-events diagnostics
- `silabs-csdk/src/silabs_rail_api_wrap.c` — RAIL init/TX instrumentation
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
- Target: `thumbv8m.main-none-eabi` (see `.cargo/config.toml`)
- Flash/run: `cargo run --release` (probe-rs + `EFR32MG24B220F1536IM48`)

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

### `--wrap=hci_le_set_extended_advertising_enable` (`silabs_ll_dispatch_diag.c`) — **critical v56**

Prebuilt `libble_host.a` (LTO) **skips `ll_hciCall`** between `hci_command_init_shared` and
`hci_command_shared_response`. The wrap re-implements `hci_adv.c` (under
`bluetooth_le_host/legacy_to_refactor/bgstack/ubt/` in the SDK) and calls
`ll_hciCall(ll_hciCmdSetExtendedAdvertisingEnable)`.

Strong `ll_hciCall` in [`silabs_ll_hci_call.c`](../silabs-csdk/src/silabs_ll_hci_call.c) matches
controller `ll_hci.c` (`bluetooth_le_controller/src/ll_hci.c`, `ll_hci.call` mailbox).

**Still needed:** same treatment for `hci_le_set_extended_advertising_parameters` and `_data`.

### `ll_hciCall` post-service (`silabs_ll_hci_post_service.c` + pump)

After synchronous HCI, the pump calls `silabs_service_linklayer_events()` and
`sl_bt_priority_handle()` so `LL_EVENT_HCI_MESSAGE` / `LL_EVENT_SCHEDULE` are processed.

### `--wrap=bg_message_queue_wait_time` (`silabs_bgmessage_stub.c`)

Return zero wait so `bg_message_run` never blocks waiting on an internal timer when
Embassy owns the main thread.

### `PendSV` → `sl_bt_priority_handle` (`sl_btctrl_pendsv.c`)

cortex-m-rt uses `PendSV` as the vector name on this target (not `PendSV_Handler`).

### Deferred start implementation (`silabs_ble_adv_start.c`)

After successful `legacy_advertiser_start`:

```c
silabs_ble_hci_drain();
silabs_service_linklayer_events();
sl_bt_priority_handle();
```

Then mark `handler_done=1` for the application log.

### Linker flags in `embassy-silabs-bt-empty/build.rs`

```text
--wrap=ubt_run
--wrap=hci_le_set_extended_advertising_enable
--wrap=bg_message_queue_wait_time
```

(`--wrap=ll_hciCall` removed in v56 — use strong `ll_hciCall` + HCI enable wrap instead.)

Plus `-uMODEM`, `-uMODEM_IRQHandler`, etc. to force-link RAIL radio IRQ handlers (v18).

Additional diagnostic wraps (v50+; do not remove while RF is open):

```text
--wrap=sl_btctrl_raise_events
--wrap=sl_btctrl_process_events
--wrap=hci_le_set_extended_advertising_enable
--wrap=usch_ScheduleProcess
--wrap=usch_ScheduleReqCB
--wrap=usch_AddTask
--wrap=sli_ll_adv_set_advertising_enable
--wrap=sli_ll_radio_schedule_tx
--wrap=sl_rail_init / sl_rail_ble_init / sl_rail_ble_config_channel_radio_params
--wrap=sl_rail_start_tx / sl_rail_start_scheduled_tx
```

### `--wrap=sl_btctrl_raise_events` (`silabs_ll_raise_wrap.c`) — **critical fix v53**

The diagnostic wrap must use `&sli_bt_host_adaptation_compatibility_linklayer_wakeup`
(the RAM slot at `0x20028920` that `sl_btctrl_raise_events` branches through).

**v52 and earlier used hardcoded `0x200288d8`**, which is **`timer_processing_ongoing`**
(a host BGAPI timer bool), not the wakeup callback. Every raised LL event corrupted that
flag and could overwrite it with trampoline pointers. This likely broke host timer /
scheduler state even when HCI reported advertising success.

### `silabs_service_linklayer_events()` (`silabs_linklayer_pump.c`) — fix v53

Must atomically read-clear `ll_events` and call `sl_btctrl_process_events(events)`.
Calling only `sl_bt_priority_handle()` from this helper was insufficient for explicit
post-HCI servicing (though `sl_bt_priority_handle` itself also drains `ll_events`).

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

## RF diagnostics (`ble_empty.rs` log line, v53+)

Periodic log (every ~2 s at 32768 Hz RTC):

```
ble rf rtc=...: scan=... steps=... done=... ll_evt=peek/total usch=sched/req
  ll_en=0x... hci_en=... irq m/f/p=modem/frc/pendsv ll_err=... ll_tx=...
  rail cfg/stx/tx=... calls cfg/stx/tx=...
```

| Field | Meaning |
|-------|---------|
| `done` | `silabs_bgapi_ble_system_boot_handler_done` (1 = post-adv pump finished) |
| `ll_evt` | `ll_events` peek / cumulative bits passed to `sl_btctrl_process_events` |
| `usch` | `usch_ScheduleProcess` / `usch_ScheduleReqCB` call counts |
| `ll_en` | Last status from `sli_ll_adv_set_advertising_enable` wrap |
| `hci_en` | Last `num_sets` arg to `hci_le_set_extended_advertising_enable` (1 = on) |
| `irq m/f/p` | MODEM / FRC / PendSV IRQ entry counts |
| `ll_tx` | `sli_ll_radio_schedule_tx` calls |
| `rail cfg/stx/tx` | Last RAIL status codes; `calls` = invocation counts |

Init summary (`ble_runtime.rs`):

```
adv4=0xNNNNNNNN  → high byte = adv setup step (4 = start), low byte = sl_status_t
hci=on/off/total → hci_le_set_extended_advertising_enable on/off/total calls
```

---

## Standalone RAIL isolation (2026-09-02)

Isolated the radio from the BLE host/LL to decide whether dead BLE RF was hardware or
stack/glue. Used feature `rail-test` / bin `rail-test` (removed after PASS; helpers remain
under `embassy-silabs` `rail` + `silabs-csdk` non-BLE RAIL init).

| Stage | Test | Result |
|-------|------|--------|
| 1 | Continuous-wave `sl_rail_start_tx_stream` on IEEE 802.15.4 ch 11 (2405 MHz) | **PASS** — `stage=6 status=0x0`; LED0 (PAEN) stayed lit for 10 s, then off |
| 2 | Immediate packet TX: FIFO + `sl_rail_start_tx`, 20 frames | **PASS** — `sent=20 aborted=0 underflow=0 start_fail=0 timeout=0`; `TX_PACKET_SENT` for all; LED0 blinked ~10 Hz |

Notes from that bring-up (keep if redoing RAIL-only work):

- RX packet queue must be power-of-2 in **[8, 512]** (4 entries → `sl_rail_init` `0x21`).
- Sequencer image for B220 (+20 dBm): `RAIL_SEQ_IMAGE_PA_20_DBM`.
- Non-BLE IRQ glue: `silabs_radio_irq_vectors_plain.c` (cortex-m-rt → `*_IRQHandler`).
- LED1 (LNAEN) stayed on after packet TX: expected — IEEE 802.15.4 config transitions
  TX success → **RX**. Nearby BLE traffic does not explain it on this PHY.

**Conclusion:** Clocks, HFXO, PA, sequencer, radio IRQs, PRS LEDs, and RAIL TX work.
BLE advertising gap is **link-layer / HCI / scheduler** (and remaining LTO HCI wraps), not
a dead radio.

---

## Current status (resumed 2026-09-02, tag `ble-empty-discover-v60`)

### Host + init — works

| Item | v60 |
|------|-----|
| Init through `ble init done` | OK |
| `adv4=0x4000000` | OK |
| `hci=1/1/2`, `add_task=1/1`, `ll_en=2/0x0` | HCI enable wrap + LL adv enable |
| `usch=1/1` at init | Schedule requested once |
| `handler_done=1`, pump `steps` > 1000 | OK |
| PendSV | `irq p` ≈ 46 |

### Standalone RAIL — works (2026-09-02)

CW + 20× packet TX with `TX_PACKET_SENT`; see section above.

### BLE link layer / on-air — not working

| Item | v60 (~2 s runtime) |
|------|---------------------|
| nRF Connect | **Not seen** |
| `usch` | **Stuck 1/1** |
| `ll_tx`, `rail cfg/stx/tx` | **0** |
| MODEM/FRC IRQs under BLE | **0** |

### Interpretation

v56 opened the HCI→LL **enable** path. After `usch_AddTask`, `usch_ScheduleProcess` does
not progress and BLE never calls RAIL TX — while standalone RAIL TX succeeds. Next:

1. Wrap remaining HCI adv commands (`parameters`, `data`) like enable.
2. Debug `LL_EVENT_SCHEDULE` / scheduler time (`sli_ll_get_current_time_us` / RAIL time).
3. Confirm BLE path reaches `ll_tx` / `rail stx/tx` / MODEM·FRC IRQs, then nRF Connect.

See [`ble-investigation-handoff.md`](../ble-investigation-handoff.md) for full timeline.

---

## History (v50–v60 + RAIL isolation)

| Tag | Milestone |
|-----|-----------|
| v50–v53 | RAIL alignment, raise-events RAM fix, RF diagnostics |
| v54 | Reference config parity, `sli_bt_stack_functional_init` |
| v56 | **HCI enable wrap + `ll_hciCall`** — `ll_en` / `add_task` non-zero |
| v57–v59 | Init pump hang fixes (defer adv, scheduler gating) |
| v60 | Startup LL pump — BLE RF still idle |
| 2026-09-02 | Standalone RAIL CW + packet TX **PASS** — HW/RAIL OK; BLE gap above RAIL |

---

## Source-only RF investigation (v54+)

**Method:** Compare a local `bt_soc_empty` reference checkout (not in repo) with Embassy glue.
SDK source: `bluetooth_le_controller/src/ll_hci*.c` (v56+).

**Opacity boundary:** `usch_*` scheduling internals in `liblinklayer.a`.

---

## Quick commands

```bash
cd embassy-silabs-bt-empty
export SILABS_SDK=/path/to/simplicity-sdk
cargo build --release --bin ble-empty
cargo run --release
```

Build tag at boot: `ble empty boot tag=ble-empty-discover-v60`.

