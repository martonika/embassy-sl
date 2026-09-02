#include <stdint.h>
#include <stddef.h>

#include "em_device.h"
#include "sli_bt_host_adaptation.h"

extern void __real_sl_btctrl_raise_events(uint32_t events);
extern void *sli_ll_tasklet_ptr;
extern void (*sli_bt_host_adaptation_compatibility_linklayer_wakeup)(void);
void sli_ll_shm_save(void *address);
void *sli_ll_shm_get(void);

typedef void (*ll_wakeup_cb_t)(void);

static uint8_t is_probably_valid_cb_ptr(uint32_t ptr)
{
  if (ptr == 0u) {
    return 0u;
  }
  if ((ptr & 1u) == 0u || ptr < 0x100u) {
    return 0u;
  }
  return 1u;
}

static void raise_wakeup_fallback_pendsv(void)
{
  SCB->ICSR = SCB_ICSR_PENDSVSET_Msk;
}

/*
 * Ensure the compatibility wakeup slot is callable before the real raise path
 * branches through it. Do not wrap or instrument internal sli_ll_radio_* APIs.
 */
void __wrap_sl_btctrl_raise_events(uint32_t events)
{
  volatile uint32_t *cb_ptr_slot =
    (volatile uint32_t *)(uintptr_t)&sli_bt_host_adaptation_compatibility_linklayer_wakeup;
  uint32_t cb_ptr = *cb_ptr_slot;

  if (sli_ll_tasklet_ptr == 0) {
    sli_ll_shm_save(sli_ll_shm_get());
  }

  if (cb_ptr == 0u || !is_probably_valid_cb_ptr(cb_ptr)) {
    *cb_ptr_slot = (uint32_t)(uintptr_t)(ll_wakeup_cb_t)raise_wakeup_fallback_pendsv;
    cb_ptr = *cb_ptr_slot;
    if (!is_probably_valid_cb_ptr(cb_ptr)) {
      return;
    }
  }

  __real_sl_btctrl_raise_events(events);
}
