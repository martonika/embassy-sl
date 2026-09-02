#include <stdbool.h>
#include <stdint.h>

#include "app_timer_internal.h"
#include "em_device.h"
#include "rail.h"
#include "sl_bluetooth.h"
#include "sl_btctrl_linklayer.h"
#include "sl_core.h"
#include "sl_rail_types.h"
#include "silabs_ll_hci_post_service.h"
#include "sli_bgapi.h"

extern uint32_t ll_events;

extern sli_bgapi_device_t *sli_bt_bgapi_device;

extern volatile uint32_t silabs_ble_step_phase;

extern void sl_bt_priority_handle(void);
extern void sli_ll_raise_events(uint32_t events);

bool hci_packets_waiting(void);
extern void __real_ubt_run(void);

typedef void (*ll_hciCmdHandler)(void);

typedef struct {
  void *hostToLL;
  void *sharedCmd;
  void *sharedRsp;
  volatile ll_hciCmdHandler call;
} ll_hci_t;

extern ll_hci_t ll_hci;

void silabs_ble_set_aggressive_ubt_run(uint8_t enable)
{
  (void)enable;
}

void silabs_ble_set_hci_unbounded(uint8_t enable)
{
  (void)enable;
}

void silabs_ble_set_use_real_ubt_run(uint8_t enable)
{
  (void)enable;
}

void silabs_service_linklayer_events(void)
{
  uint32_t events;
  CORE_DECLARE_IRQ_STATE;

  CORE_ENTER_ATOMIC();
  events = ll_events;
  ll_events = 0;
  CORE_EXIT_ATOMIC();

  if (events != 0u) {
    sl_btctrl_process_events(events);
  }
}

void silabs_ble_hci_pump_begin(void)
{
}

void silabs_ble_hci_pump_end(void)
{
}

void __wrap_ubt_run(void)
{
  sli_app_timer_step();
  __real_ubt_run();
}

/* Using SDK-provided ll_hciCall from linklayer library. */

void silabs_ubt_run_pumped(void)
{
  __wrap_ubt_run();
}

void silabs_ble_hci_drain(void)
{
  unsigned guard = 0;

  while (hci_packets_waiting() && guard < 256u) {
    guard++;
    silabs_ubt_run_pumped();
  }
}

void silabs_sl_bt_run_pump(void)
{
  silabs_ble_step_phase = 11;
  sl_bt_run();
  silabs_ble_step_phase = 16;
}

void silabs_ble_before_sync_bgapi_command(void)
{
  sl_bt_priority_handle();
  sl_bt_run();
}
