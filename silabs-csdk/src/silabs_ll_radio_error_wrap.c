#include <stdint.h>
#include <stddef.h>

#include "silabs_bgapi_debug.h"

extern void __real_sli_ll_radio_raise_error(uint8_t error);

static volatile uint32_t diag_ll_radio_raise_error_calls;
static volatile uint32_t diag_ll_radio_raise_error_last;
static volatile uint32_t diag_ll_radio_raise_error14_skip_once_calls;
static volatile uint32_t diag_ll_radio_raise_error_caller_last;
static volatile uint32_t diag_ll_radio_raise_error14_caller_first;
static volatile uint32_t diag_ll_radio_raise_error14_caller_last;

uint32_t silabs_bgapi_ll_radio_raise_error_calls(void)
{
  return diag_ll_radio_raise_error_calls;
}

uint32_t silabs_bgapi_ll_radio_raise_error_last(void)
{
  return diag_ll_radio_raise_error_last;
}

uint32_t silabs_bgapi_ll_radio_raise_error14_skip_once_calls(void)
{
  return diag_ll_radio_raise_error14_skip_once_calls;
}

uint32_t silabs_bgapi_ll_radio_raise_error_caller_last(void)
{
  return diag_ll_radio_raise_error_caller_last;
}

uint32_t silabs_bgapi_ll_radio_raise_error14_caller_first(void)
{
  return diag_ll_radio_raise_error14_caller_first;
}

uint32_t silabs_bgapi_ll_radio_raise_error14_caller_last(void)
{
  return diag_ll_radio_raise_error14_caller_last;
}

void __wrap_sli_ll_radio_raise_error(uint8_t error)
{
  uint32_t caller = (uint32_t)(uintptr_t)__builtin_return_address(0);
  diag_ll_radio_raise_error_calls++;
  diag_ll_radio_raise_error_last = (uint32_t)error;
  diag_ll_radio_raise_error_caller_last = caller;
  if (error == 14u) {
    if (diag_ll_radio_raise_error14_caller_first == 0u) {
      diag_ll_radio_raise_error14_caller_first = caller;
    }
    diag_ll_radio_raise_error14_caller_last = caller;
  }
  __real_sli_ll_radio_raise_error(error);
}
