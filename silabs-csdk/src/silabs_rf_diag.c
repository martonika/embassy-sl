#include <stdint.h>
#include <stddef.h>

#include "em_device.h"
#include "silabs_bgapi_debug.h"

extern uint32_t ll_events;

uint32_t silabs_bgapi_ll_events_peek(void)
{
  return ll_events;
}

uint32_t silabs_bgapi_ll_pending_peek(void)
{
  /* Controller struct: pending mask at +16, ll_events at +40. */
  volatile uint32_t *base = (volatile uint32_t *)((uintptr_t)&ll_events - 40u);
  return base[4];
}

uint32_t silabs_bgapi_sysrtc_cnt(void)
{
  return SYSRTC0->CNT;
}

uint32_t silabs_bgapi_primask(void)
{
  uint32_t primask;

  __asm volatile("MRS %0, primask" : "=r"(primask));
  return primask;
}

uint32_t silabs_bgapi_nvic_iser0(void)
{
  return NVIC->ISER[0];
}

uint32_t silabs_bgapi_nvic_iser1(void)
{
  return NVIC->ISER[1];
}

uint32_t silabs_bgapi_nvic_iser2(void)
{
  return NVIC->ISER[2];
}
