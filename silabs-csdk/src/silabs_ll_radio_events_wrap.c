#include <stdint.h>

#include "silabs_bgapi_debug.h"

extern void __real_sli_ll_radio_raise_ll_events(uint32_t events);

static volatile uint32_t diag_ll_radio_raise_ll_events_calls;
static volatile uint32_t diag_ll_radio_raise_ll_events_last;

uint32_t silabs_bgapi_ll_radio_raise_ll_events_calls(void)
{
  return diag_ll_radio_raise_ll_events_calls;
}

uint32_t silabs_bgapi_ll_radio_raise_ll_events_last(void)
{
  return diag_ll_radio_raise_ll_events_last;
}

void __wrap_sli_ll_radio_raise_ll_events(uint32_t events)
{
  diag_ll_radio_raise_ll_events_calls++;
  diag_ll_radio_raise_ll_events_last = events;
  __real_sli_ll_radio_raise_ll_events(events);
}
