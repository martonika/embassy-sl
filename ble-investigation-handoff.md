# BLE Advertising Investigation Handoff (BRD4186C / EFR32MG24)

**Resumed:** 2026-09-02 · build tag `ble-empty-discover-v61` · SDK `sisdk-2026.12`

## Goal

Bring up connectable BLE advertising (`"Embassy BLE"`) discoverable in nRF Connect.

## Current status (v61)

### Build on SDK 2026.12 — fixed

- Missing `sl_log_component.h` (new `platform/.../service/sl_log`)
- Link `libbgapi_task.a` (`sli_bgapi_task_*`)
- Dropped removed memory_manager profiler stub

### RF hypothesis (v61)

v60 `usch=1/1` was almost certainly a **gated** `ScheduleProcess` during deferred adv start
(`schedule_allow_real=0`). `LL_EVENT_SCHEDULE` is then cleared without running the real
scheduler (`bluetooth-le/.../scheduler/scheduler.c`). v61:

- Counters: `usch=total/real/gated/req`
- After ungating, `startup_ll_pump` calls `usch_ScheduleProcess()` once

### Still open

1. Flash v61; check `usch real` > 0 and whether `ll_tx` / RAIL TX move
2. Wrap `hci_le_set_extended_advertising_parameters` / `_data` (same LTO `ll_hciCall` gap)
3. If real ScheduleProcess spins: `usch_GetTimeCB` / RAIL time

Source trees: `~/silabs/bluetooth-le`, `~/silabs/rail` (no binary disassembly).


## Investigation method

| Phase | Method |
|-------|--------|
| v54 | Source/config diff vs local `reference_project/` (not committed — SDK copy) |
| v56+ | SDK source `ll_hci.c` / `ll_hci_adv.c` + binary correlation for LTO `ll_hciCall` elision |
| Opacity | `usch_*` implementation in `liblinklayer.a`; scheduler internals not in reference tree |

## Init / pump evolution (v54–v60)

| Tag | Change |
|-----|--------|
| v54 | Reference `sl_btctrl` config, `sli_bt_stack_functional_init`, sleep-clock flag wrap |
| v55 | Inline adv start experiment (reverted) |
| v56 | **`hci_le_set_extended_advertising_enable` wrap + `ll_hciCall`** — LL path opens |
| v57–v58 | Init pump hang: defer adv start, light pump (skip HCI drain during boot pump) |
| v59 | Scheduler gated during init; adv start after pump with `skip_post_adv_pump` |
| v60 | `silabs_ble_startup_ll_pump()` at `pump_loop` entry — RF still idle |

## Bisection history

- `usch_ScheduleReqCB` bypass → adv returns, scheduler blocked
- `sl_btctrl_raise_events` skip → HardFault in adv task handler
- v52 wrong RAM address `0x200288d8` → corrupted `timer_processing_ongoing` (fixed v53)
- Inline adv start + post-adv 256-pump inside `on_event` → init hang (v56+)
- `usch_ScheduleProcess` with scheduler enabled during init finish → hang at `adv_start begin` (v58)

## Key files (Embassy glue)

| File | Role |
|------|------|
| [`silabs_ll_dispatch_diag.c`](silabs-csdk/src/silabs_ll_dispatch_diag.c) | HCI enable wrap, `usch_*` / `ll_en` diagnostics, scheduler gating |
| [`silabs_ll_hci_call.c`](silabs-csdk/src/silabs_ll_hci_call.c) | Strong `ll_hciCall` matching `ll_hci.c` |
| [`silabs_ble_adv_start.c`](silabs-csdk/src/silabs_ble_adv_start.c) | Deferred adv start, startup LL pump |
| [`ble_platform_init.c`](silabs-csdk/src/ble_platform_init.c) | 9-step init, light pump during `post_init_pump_active` |
| [`ble_runtime.rs`](embassy-silabs-bt-empty/src/ble_runtime.rs) | Init pump loop, RF diagnostics |
| [`ble_app.rs`](embassy-silabs-bt-empty/src/ble_app.rs) | Rust `sl_bt_on_event`, pending adv start |

## When resuming

1. Wrap `hci_le_set_extended_advertising_parameters` and `_data` (same pattern as enable).
2. Confirm `LL_EVENT_SCHEDULE` is processed after init (`ll_evt` peek, `usch` > 1).
3. If `ScheduleProcess` spins: inspect `sli_ll_get_current_time_us()` / `sl_rail_get_time` before enabling real schedule.
4. Hardware check: `ll_tx`, `rail cfg/stx/tx`, MODEM/FRC IRQs, nRF Connect.

## SDK dependency

Requires `SILABS_SDK` pointing at the Simplicity SDK root. Prebuilt BLE/RAIL archives are not vendored in this repo.
