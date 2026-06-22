#include "silabs_bgapi_debug.h"
#include "silabs_ble_platform.h"
#include "sl_bt_api.h"
#include "sl_bluetooth.h"
#include "sl_status.h"

extern volatile uint32_t silabs_ble_step_phase;

static uint8_t advertising_set_handle = 0xff;
static volatile uint8_t pending_adv_start;

typedef enum {
  ADV_FSM_IDLE = 0,
  ADV_FSM_DONE,
} adv_fsm_t;

static volatile adv_fsm_t adv_fsm;

static void note_adv_status(uint8_t step, sl_status_t sc)
{
  silabs_bgapi_note_ble_adv_setup_status(((uint32_t)step << 24) | (uint32_t)sc);
}

uint8_t silabs_ble_adv_in_progress(void)
{
  return pending_adv_start;
}

uint8_t silabs_ble_adv_fsm_read(void)
{
  return (uint8_t)adv_fsm;
}

uint8_t silabs_ble_pending_adv_start_read(void)
{
  return pending_adv_start;
}

void silabs_ble_pending_adv_start_clear(void)
{
  pending_adv_start = 0;
}

void silabs_ble_adv_tick(void)
{
}

static void note_identity_address(void)
{
  sl_status_t sc;
  bd_addr address;
  uint8_t addr_type;

  sc = sl_bt_gap_get_identity_address(&address, &addr_type);
  silabs_bgapi_note_ble_identity_status(sc);
  if (sc == SL_STATUS_OK) {
    silabs_bgapi_note_ble_identity_address(address.addr, addr_type);
  }
}

void silabs_ble_try_finish_pending_adv_start(void)
{
  sl_status_t sc;

  if (pending_adv_start == 0u) {
    return;
  }

  silabs_ble_step_phase = 47;
  sc = sl_bt_legacy_advertiser_start(advertising_set_handle,
                                     sl_bt_legacy_advertiser_connectable);
  silabs_ble_step_phase = 48;
  note_adv_status(4, sc);
  pending_adv_start = 0;

  if (sc != SL_STATUS_OK) {
    adv_fsm = ADV_FSM_IDLE;
    return;
  }

  silabs_ble_step_phase = 44;
  silabs_bgapi_note_ble_system_boot_handler_done(1);
  note_identity_address();
  adv_fsm = ADV_FSM_DONE;
}

__attribute__((weak)) void sl_bt_on_event(sl_bt_msg_t *evt)
{
  sl_status_t sc;

  silabs_bgapi_note_ble_on_event_called();

  switch (SL_BT_MSG_ID(evt->header)) {
    case sl_bt_evt_system_boot_id:
      silabs_ble_step_phase = 30;
      note_identity_address();

      silabs_ble_step_phase = 40;
      sc = sl_bt_advertiser_create_set(&advertising_set_handle);
      note_adv_status(1, sc);
      if (sc != SL_STATUS_OK) {
        adv_fsm = ADV_FSM_IDLE;
        break;
      }

      silabs_ble_step_phase = 41;
      sc = sl_bt_legacy_advertiser_generate_data(advertising_set_handle,
                                                 sl_bt_advertiser_general_discoverable);
      note_adv_status(2, sc);
      if (sc != SL_STATUS_OK) {
        adv_fsm = ADV_FSM_IDLE;
        break;
      }

      silabs_ble_step_phase = 42;
      sc = sl_bt_advertiser_set_timing(advertising_set_handle, 160, 160, 0, 0);
      note_adv_status(3, sc);
      if (sc == SL_STATUS_OK) {
        pending_adv_start = 1;
      } else {
        adv_fsm = ADV_FSM_IDLE;
      }
      break;

    case sl_bt_evt_connection_closed_id:
      sc = sl_bt_legacy_advertiser_generate_data(advertising_set_handle,
                                                 sl_bt_advertiser_general_discoverable);
      note_adv_status(5, sc);
      if (sc == SL_STATUS_OK) {
        pending_adv_start = 1;
      }
      break;

    default:
      break;
  }
}

sl_status_t silabs_ble_start_connectable_advertising(void)
{
  return SL_STATUS_OK;
}
