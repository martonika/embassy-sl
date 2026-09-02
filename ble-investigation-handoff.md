# BLE Advertising Investigation Handoff (BRD4186C / EFR32MG24)

**Resumed:** 2026-09-02 · build tag `ble-empty-discover-v60`

Standalone RAIL CW + packet TX **PASS** (see `embassy-silabs-bt-empty/BLE_BRINGUP.md`).
Hardware/RAIL/IRQs/PA are fine; remaining work is BLE HCI/LL/scheduler.

## Goal

Bring up connectable BLE advertising (`"Embassy BLE"`) discoverable in nRF Connect.

## Current status (v60 + RAIL isolation)

### Host + init — works

| Item | v60 observation |
|------|-----------------|
| Full init through `ble init done` | OK |
| `adv4=0x4000000` (step 4, `sc=0x00`) | OK |
| `handler_done=1`, main loop runs | OK |
| `hci=1/1/2` at init | HCI enable wrap fired (on + off during setup) |
| `add_task=1/1`, `ll_en=2/0x0` | LL adv enable + scheduler task queue reached |
| `usch=1/1` at init | Schedule requested once during adv enable |
| `rail init/ble=1/1` | RAIL + RAIL-BLE init succeed |
| PendSV | `irq p` ≈ 46 after ~2 s pumping |

### Link layer / RF — not working

| Item | v60 observation (runtime, ~2 s) |
|------|-----------------------------------|
| nRF Connect visibility | **Not seen** |
| `usch` sched/req | **Stuck at 1 / 1** (no further `ScheduleProcess` after init) |
| `ll_tx` | **0** |
| `rail cfg/stx/tx` | **0 / 0 / 0** |
| `irq m/f` (MODEM/FRC) | **0 / 0** |
| `steps` | 1230+ (pump running; RF path idle) |

### Root cause found (v56, SDK source)

Prebuilt `libble_host.a` (LTO) **elides `ll_hciCall`** between `hci_command_init_shared` and `hci_command_shared_response` for all legacy `hci_adv.c` commands. Host reads a stale success response; the controller handler never ran.

**Source path (Silicon Labs `bluetooth_le_controller`):**

```
hci_adv.c: hci_command_init_shared → ll_hciCall(handler) → hci_command_shared_response
ll_hci.c:  ll_hci.call = handler; raise LL_EVENT_HCI_MESSAGE; sync invoke handler
ll_hci_adv.c: ll_hciCmdSetExtendedAdvertisingEnable → sli_ll_adv_set_advertising_enable → usch_AddTask
sli_ll_init.c: LL_EVENT_SCHEDULE → usch_ScheduleProcess → … → RAIL TX
```

**Fix applied:** `__wrap_hci_le_set_extended_advertising_enable` in [`silabs_ll_dispatch_diag.c`](silabs-csdk/src/silabs_ll_dispatch_diag.c) re-implements the `hci_adv.c` sequence and calls `ll_hciCall(ll_hciCmdSetExtendedAdvertisingEnable)`. Strong `ll_hciCall` from [`silabs_ll_hci_call.c`](silabs-csdk/src/silabs_ll_hci_call.c).

### Remaining RF gap (v60)

HCI **enable** reaches the LL (`ll_en`, `add_task` non-zero), but **`usch_ScheduleProcess` does not progress** after init (count stays 1) and BLE never invokes RAIL TX. **Not** a dead radio: isolation test proved CW + immediate packet TX with `TX_PACKET_SENT`. Likely causes when resuming:

1. Other HCI setup commands (`set_extended_advertising_parameters`, `_data`) still use LTO-broken path — wrap like enable.
2. `usch_ScheduleProcess` inner `while (usch_TrySchedule() == false)` may spin or stall without valid `sli_ll_get_current_time_us()` / RAIL time.
3. Real `ScheduleProcess` was gated during init (v59); startup pump (v60) did not raise `usch` count — pending `LL_EVENT_SCHEDULE` may not be re-processed.

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
