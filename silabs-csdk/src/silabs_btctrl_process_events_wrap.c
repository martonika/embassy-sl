#include <stdint.h>

#include "silabs_bgapi_debug.h"

extern void __real_sl_btctrl_process_events(uint32_t events);

void __wrap_sl_btctrl_process_events(uint32_t events)
{
  if (events != 0u) {
    silabs_bgapi_note_ll_events(events);
  }
  __real_sl_btctrl_process_events(events);
}
