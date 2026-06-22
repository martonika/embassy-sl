#include <stdint.h>
#include <stddef.h>

#include "silabs_bgapi_debug.h"

extern void __real_sli_ll_radio_schedule_tx(void *params);

static volatile uint32_t diag_ll_radio_schedule_tx_calls;
static volatile uint32_t diag_ll_radio_schedule_tx_last_params;
static volatile uint32_t diag_ll_radio_schedule_tx_last_caller;
static volatile uint32_t diag_ll_radio_schedule_tx_last_err_before;
static volatile uint32_t diag_ll_radio_schedule_tx_last_err_after;
static volatile uint32_t diag_ll_radio_schedule_tx_error_transition_calls;
static volatile uint32_t diag_ll_radio_schedule_tx_param_w0;
static volatile uint32_t diag_ll_radio_schedule_tx_param_w1;
static volatile uint32_t diag_ll_radio_schedule_tx_param_w2;
static volatile uint32_t diag_ll_radio_schedule_tx_param_w3;
static volatile uint32_t diag_ll_radio_schedule_tx_param_w4;
static volatile uint32_t diag_ll_radio_schedule_tx_param_w5;

uint32_t silabs_bgapi_ll_radio_schedule_tx_calls(void)
{
  return diag_ll_radio_schedule_tx_calls;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_last_params(void)
{
  return diag_ll_radio_schedule_tx_last_params;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_last_caller(void)
{
  return diag_ll_radio_schedule_tx_last_caller;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_last_err_before(void)
{
  return diag_ll_radio_schedule_tx_last_err_before;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_last_err_after(void)
{
  return diag_ll_radio_schedule_tx_last_err_after;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_error_transition_calls(void)
{
  return diag_ll_radio_schedule_tx_error_transition_calls;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_param_w0(void)
{
  return diag_ll_radio_schedule_tx_param_w0;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_param_w1(void)
{
  return diag_ll_radio_schedule_tx_param_w1;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_param_w2(void)
{
  return diag_ll_radio_schedule_tx_param_w2;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_param_w3(void)
{
  return diag_ll_radio_schedule_tx_param_w3;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_param_w4(void)
{
  return diag_ll_radio_schedule_tx_param_w4;
}

uint32_t silabs_bgapi_ll_radio_schedule_tx_param_w5(void)
{
  return diag_ll_radio_schedule_tx_param_w5;
}

void __wrap_sli_ll_radio_schedule_tx(void *params)
{
  uint32_t before = silabs_bgapi_ll_radio_raise_error_calls();
  const volatile uint32_t *p = (const volatile uint32_t *)params;

  diag_ll_radio_schedule_tx_calls++;
  diag_ll_radio_schedule_tx_last_params = (uint32_t)(uintptr_t)params;
  diag_ll_radio_schedule_tx_last_caller = (uint32_t)(uintptr_t)__builtin_return_address(0);
  diag_ll_radio_schedule_tx_last_err_before = before;
  if (p != NULL) {
    diag_ll_radio_schedule_tx_param_w0 = p[0];
    diag_ll_radio_schedule_tx_param_w1 = p[1];
    diag_ll_radio_schedule_tx_param_w2 = p[2];
    diag_ll_radio_schedule_tx_param_w3 = p[3];
    diag_ll_radio_schedule_tx_param_w4 = p[4];
    diag_ll_radio_schedule_tx_param_w5 = p[5];
  }

  __real_sli_ll_radio_schedule_tx(params);

  uint32_t after = silabs_bgapi_ll_radio_raise_error_calls();
  diag_ll_radio_schedule_tx_last_err_after = after;
  if (after != before) {
    diag_ll_radio_schedule_tx_error_transition_calls++;
  }
}
