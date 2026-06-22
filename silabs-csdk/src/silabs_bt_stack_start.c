#include "silabs_bt_stack_start.h"
#include "silabs_bgapi_debug.h"
#include "sli_bt_api.h"
#include "sl_bt_stack_init.h"
#include "sl_component_catalog.h"
#include "sl_status.h"

#if defined(SL_CATALOG_BLUETOOTH_EVENT_SYSTEM_IPC_PRESENT)
#include "sli_bt_event_system.h"
#endif

#if defined(SL_CATALOG_KERNEL_PRESENT)
#include "sl_bt_rtos_adaptation.h"
#endif

sl_status_t silabs_bt_stack_functional_init(void)
{
#if defined(SL_CATALOG_BLUETOOTH_EVENT_SYSTEM_IPC_PRESENT)
  sl_status_t status = sli_bt_event_system_functional_init();
  if (status != SL_STATUS_OK) {
    silabs_bgapi_note_bt_start_status(status);
    return status;
  }
#endif

#if !defined(SL_CATALOG_BLUETOOTH_ON_DEMAND_START_PRESENT)
#if defined(SL_CATALOG_KERNEL_PRESENT)
  sl_status_t status = sli_bt_rtos_adaptation_start();
  silabs_bgapi_note_bt_start_status(status);
  return status;
#else
  // Match reference sl_stack_init → sli_bt_stack_functional_init().
  sli_bt_stack_functional_init();
  silabs_bgapi_note_bt_start_status(SL_STATUS_OK);
  return SL_STATUS_OK;
#endif
#else
  silabs_bgapi_note_bt_start_status(SL_STATUS_OK);
  return SL_STATUS_OK;
#endif
}
