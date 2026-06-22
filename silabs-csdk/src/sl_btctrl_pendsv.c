#include <stdint.h>
#include "silabs_bgapi_debug.h"
#include "sl_btctrl_linklayer.h"
#include "sl_core.h"

void sl_bt_priority_handle(void);

/*
 * SDK bare-metal: sli_bt_host_adaptation.c defines PendSV_Handler ->
 * sl_bt_priority_handle(). cortex-m-rt names the vector PendSV on this target.
 */
void PendSV(void)
{
  silabs_bgapi_note_irq_pendsv();
  sl_bt_priority_handle();
}
