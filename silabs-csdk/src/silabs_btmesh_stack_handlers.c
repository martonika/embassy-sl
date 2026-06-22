#include "silabs_bgapi_debug.h"
#include "sl_bt_api.h"
#include "sl_bluetooth.h"
#include "sl_btmesh_api.h"
#include "sl_btmesh.h"
#include "sl_status.h"

static uint8_t advertising_set_handle = 0xff;

static void note_adv_status(uint8_t step, sl_status_t sc)
{
  silabs_bgapi_note_ble_adv_setup_status(((uint32_t)step << 24) | (uint32_t)sc);
}

void sl_bt_on_event(sl_bt_msg_t *evt)
{
  sl_status_t sc;

  switch (SL_BT_MSG_ID(evt->header)) {
    case sl_bt_evt_system_boot_id: {
      bd_addr address;
      uint8_t addr_type;

      sc = sl_bt_gap_get_identity_address(&address, &addr_type);
      silabs_bgapi_note_ble_identity_status(sc);
      if (sc == SL_STATUS_OK) {
        silabs_bgapi_note_ble_identity_address(address.addr, addr_type);
      }

      sc = sl_bt_advertiser_create_set(&advertising_set_handle);
      note_adv_status(1, sc);
      if (sc != SL_STATUS_OK) {
        break;
      }
      sc = sl_bt_legacy_advertiser_generate_data(advertising_set_handle,
                                                 sl_bt_advertiser_general_discoverable);
      note_adv_status(2, sc);
      if (sc != SL_STATUS_OK) {
        break;
      }
      sc = sl_bt_advertiser_set_timing(advertising_set_handle, 160, 160, 0, 0);
      note_adv_status(3, sc);
      if (sc != SL_STATUS_OK) {
        break;
      }
      sc = sl_bt_legacy_advertiser_start(advertising_set_handle,
                                         sl_bt_legacy_advertiser_connectable);
      note_adv_status(4, sc);
      silabs_bgapi_note_ble_system_boot_handler_done(sc == SL_STATUS_OK);
      break;
    }

    case sl_bt_evt_connection_closed_id:
      sc = sl_bt_legacy_advertiser_generate_data(advertising_set_handle,
                                                 sl_bt_advertiser_general_discoverable);
      note_adv_status(5, sc);
      if (sc != SL_STATUS_OK) {
        break;
      }
      sc = sl_bt_legacy_advertiser_start(advertising_set_handle,
                                         sl_bt_legacy_advertiser_connectable);
      note_adv_status(6, sc);
      break;

    default:
      break;
  }
}

void sl_btmesh_on_event(sl_btmesh_msg_t *evt)
{
  switch (SL_BT_MSG_ID(evt->header)) {
    case sl_btmesh_evt_node_initialized_id:
      (void)sl_btmesh_node_start_unprov_beaconing(3);
      break;
    default:
      break;
  }
}
