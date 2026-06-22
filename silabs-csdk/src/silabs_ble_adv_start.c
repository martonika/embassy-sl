#include "silabs_ble_adv_start.h"

#include "em_device.h"
#include "silabs_bgapi_debug.h"
#include "silabs_ble_platform.h"
#include "silabs_ll_hci_post_service.h"
#include "sl_bluetooth.h"
#include "sl_bt_api.h"
#include "sl_status.h"

extern void sl_bt_priority_handle(void);
extern volatile uint32_t silabs_ble_step_phase;

static void silabs_ble_force_pendsv(void)
{
  SCB->ICSR = SCB_ICSR_PENDSVSET_Msk;
}

static volatile uint8_t silabs_ble_skip_post_adv_pump;

void silabs_ble_set_skip_post_adv_pump(uint8_t skip)
{
  silabs_ble_skip_post_adv_pump = skip;
}

void silabs_ble_finish_pending_adv_start(void)
{
  uint8_t handle;
  sl_status_t sc;

  if (silabs_ble_pending_adv_start_read() == 0u) {
    return;
  }

  handle = silabs_ble_adv_handle_read();
  if (handle == 0xffu) {
    return;
  }

  silabs_ble_pending_adv_start_clear();
  silabs_ble_step_phase = 46;
  sc = sl_bt_legacy_advertiser_start(handle, sl_bt_legacy_advertiser_connectable);
  silabs_ble_step_phase = 47;
  silabs_bgapi_note_ble_adv_setup_status(((uint32_t)4 << 24) | (uint32_t)sc);

  if (sc != SL_STATUS_OK) {
    return;
  }

  if (silabs_ble_skip_post_adv_pump != 0u) {
    silabs_ble_ensure_radio_irqs_enabled();
    silabs_bgapi_note_ble_adv_start_pump_done();
    silabs_bgapi_note_ble_system_boot_handler_done(1);
    silabs_ble_step_phase = 44;
    return;
  }

  silabs_ble_on_connectable_adv_started(sc);
}

void silabs_ble_post_adv_start_pump(uint32_t rounds)
{
  uint32_t i;

  for (i = 0; i < rounds; i++) {
    silabs_ble_hci_drain();
    silabs_service_linklayer_events();
    sl_bt_priority_handle();
    sl_bt_run();
  }
}

void silabs_ble_startup_ll_pump(void)
{
  uint32_t i;

  silabs_ll_hci_post_service_set(1u);
  for (i = 0; i < 32u; i++) {
    silabs_ble_force_pendsv();
    silabs_ble_hci_drain();
    silabs_service_linklayer_events();
    sl_bt_priority_handle();
    silabs_ble_step_bt_pump();
  }
  silabs_ll_hci_post_service_set(0u);
}

void silabs_ble_on_connectable_adv_started(sl_status_t sc)
{
  uint32_t i;

  if (sc != SL_STATUS_OK) {
    return;
  }

  silabs_ll_hci_post_service_set(1);

  for (i = 0; i < 32u; i++) {
    silabs_ble_force_pendsv();
    silabs_ble_hci_drain();
    silabs_service_linklayer_events();
    sl_bt_priority_handle();
    sl_bt_run();
  }
  silabs_ble_ensure_radio_irqs_enabled();
  silabs_bgapi_note_ble_adv_start_pump_done();
  silabs_bgapi_note_ble_system_boot_handler_done(1);
  silabs_ble_step_phase = 44;
}
