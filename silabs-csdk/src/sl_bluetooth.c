#include "silabs_bgapi_debug.h"
#include "silabs_ble_platform.h"
#include "sl_bluetooth.h"
#include "sli_bgapi.h"
#include "sli_bt_api.h"
#include "sl_bt_api.h"
#include "sl_assert.h"
#include "sl_bt_stack_init.h"
#include "sl_component_catalog.h"

#if defined(SILABS_CSDK_BTMESH)
#include "sl_btmesh_bgapi.h"
#include "sl_btmesh_provisionee.h"

uint8_t silabs_btmesh_skip_node_init_read(void);
#endif

extern volatile uint32_t silabs_ble_step_phase;

void silabs_service_linklayer_events(void);
void silabs_sl_bt_run_pump(void);

void sl_bt_init(void)
{
  sl_status_t err = sl_bt_stack_init();
  EFM_ASSERT(err == SL_STATUS_OK);
}

void sl_bt_process_event(sl_bt_msg_t *evt)
{
#if defined(SILABS_CSDK_BTMESH)
  sl_btmesh_bgapi_listener(evt);
  if (!silabs_btmesh_skip_node_init_read()) {
    sl_bt_provisionee_on_event(evt);
  }
#endif
  sl_bt_on_event(evt);
}

__attribute__((weak)) bool sl_bt_can_process_event(uint32_t len)
{
  (void)len;
  return true;
}

void sl_bt_step(void)
{
  silabs_ble_step_bt_pump();
  silabs_ble_step_bt_event();
}

void silabs_ble_step_bt_pump(void)
{
  silabs_ble_step_phase = 20;
  silabs_bgapi_note_bt_step();
  silabs_ble_step_phase = 22;
  silabs_sl_bt_run_pump();
  silabs_ble_step_phase = 23;
}

void silabs_ble_step_bt_event(void)
{
  unsigned guard = 0;

  while (guard < 1u) {
    sl_bt_msg_t evt;

    silabs_ble_step_phase = 24;
    size_t event_len = sli_bgapi_device_peek_event_len(sli_bt_bgapi_device);
    if ((event_len == 0) || (!sl_bt_can_process_event(event_len))) {
      silabs_ble_step_phase = 27;
      return;
    }

    silabs_ble_step_phase = 25;
    sl_status_t status = sli_bgapi_device_pop_event(sli_bt_bgapi_device,
                                                    sizeof(evt),
                                                    &evt);
    if (status != SL_STATUS_OK) {
      silabs_ble_step_phase = 27;
      return;
    }

    guard++;
    silabs_ble_step_phase = 26;
    silabs_bgapi_note_bt_event(SL_BT_MSG_ID(evt.header));
    sl_bt_process_event(&evt);
    if (SL_BT_MSG_ID(evt.header) == (uint32_t)sl_bt_evt_system_boot_id) {
      silabs_ble_step_phase = 30;
    } else {
      silabs_ble_step_phase = 27;
    }
  }
}
