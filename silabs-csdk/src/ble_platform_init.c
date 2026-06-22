#include "em_cmu.h"
#include "sl_device_init_hfxo.h"
#include "sl_device_init_dcdc.h"
#include "sl_device_init_emu.h"
#include "sl_device_init_clocks.h"
#include "sl_bluetooth.h"
#include "sl_memory_manager.h"
#include "sl_sleeptimer.h"
#include "sl_status.h"
#include "sl_bt_stack_init.h"
#include "sl_btctrl_linklayer.h"
#include "app_timer_internal.h"
#include "silabs_bgapi_debug.h"
#include "silabs_ble_platform.h"
#include "silabs_bt_stack_start.h"
#include "sl_bt_api.h"
#include "pa_conversions_efr32.h"
#include "sl_rail_util_pti.h"
#include "sl_rail_util_power_manager_init.h"

#if defined(SILABS_CSDK_BTMESH)
#include "sl_btmesh.h"
#include "nvm3_default.h"
#include "silabs_crypto_platform_init.h"
#endif

static volatile uint32_t silabs_ble_init_stage;

uint32_t silabs_ble_init_stage_read(void)
{
  return silabs_ble_init_stage;
}

void silabs_ble_platform_init_step(uint32_t step)
{
  switch (step) {
    case 1:
      silabs_ble_init_stage = 1;
      (void)sl_memory_init();
      break;
    case 2:
      silabs_ble_init_stage = 2;
      (void)sl_device_init_dcdc();
      break;
    case 3:
      silabs_ble_init_stage = 3;
      (void)sl_device_init_hfxo();
      break;
    case 4:
      silabs_ble_init_stage = 4;
      (void)sl_device_init_clocks();
      break;
    case 5:
      silabs_ble_init_stage = 5;
      (void)sl_device_init_emu();
      break;
    case 6:
      silabs_ble_init_stage = 6;
      (void)sl_sleeptimer_init();
      break;
#if defined(SILABS_CSDK_BTMESH)
    case 7:
      silabs_ble_init_stage = 7;
      (void)nvm3_initDefault();
      break;
    case 8:
      silabs_ble_init_stage = 8;
      sli_bt_stack_permanent_allocation();
      break;
    case 9:
      silabs_ble_init_stage = 9;
      silabs_crypto_platform_init();
      break;
    case 10:
      silabs_ble_init_stage = 10;
      sl_rail_util_pa_init();
      sl_rail_util_power_manager_init();
      sl_rail_util_pti_init();
      break;
    case 11:
      silabs_ble_init_stage = 11;
      (void)silabs_bt_stack_functional_init();
      break;
    case 12:
      silabs_ble_init_stage = 12;
      sl_btmesh_init();
      break;
#else
    case 7:
      silabs_ble_init_stage = 7;
      sli_bt_stack_permanent_allocation();
      break;
    case 8:
      silabs_ble_init_stage = 8;
      sl_rail_util_pa_init();
      sl_rail_util_power_manager_init();
      sl_rail_util_pti_init();
      break;
    case 9:
      silabs_ble_init_stage = 9;
      (void)silabs_bt_stack_functional_init();
      sl_btctrl_init_tasklets();
      break;
#endif
    default:
      break;
  }
}

void silabs_ble_platform_init(void)
{
#if defined(SILABS_CSDK_BTMESH)
  for (uint32_t step = 1; step <= 12; step++) {
    silabs_ble_platform_init_step(step);
  }
#else
  for (uint32_t step = 1; step <= 9; step++) {
    silabs_ble_platform_init_step(step);
  }
#endif
}

volatile uint32_t silabs_ble_step_phase;

uint32_t silabs_ble_step_phase_read(void)
{
  return silabs_ble_step_phase;
}

void silabs_ble_step_phase_write(uint32_t phase)
{
  silabs_ble_step_phase = phase;
}

void silabs_ble_step(void)
{
  silabs_ble_step_app_timer();
  silabs_ble_step_sleeptimer();
  silabs_ble_step_bt_host();
#if defined(SILABS_CSDK_BTMESH)
  sl_btmesh_step();
#endif
}

void silabs_ble_step_app_timer(void)
{
  silabs_ble_step_phase = 1;
  sli_app_timer_step();
  silabs_ble_step_phase = 2;
}

void silabs_ble_step_sleeptimer(void)
{
  silabs_ble_step_phase = 3;
}

extern uint8_t silabs_ble_post_init_pump_active_read(void);

void silabs_ble_step_bt_host(void)
{
  silabs_ble_step_bt_pump();
  silabs_ble_step_bt_event();
  if (silabs_ble_post_init_pump_active_read() != 0u) {
    /* Boot event only: avoid hci_drain / LL scheduler while still in init pump. */
    silabs_ble_step_phase = 6;
    return;
  }
  silabs_ble_finish_pending_adv_start();
  silabs_ble_hci_drain();
  silabs_service_linklayer_events();
  sl_bt_priority_handle();
  silabs_ble_step_phase = 6;
}
