#include <stdint.h>
#include <stddef.h>

#include "em_device.h"
#include "silabs_bgapi_debug.h"
#include "sli_bt_host_adaptation.h"

extern void __real_sl_btctrl_raise_events(uint32_t events);
extern void *sli_ll_tasklet_ptr;
extern void (*sli_bt_host_adaptation_compatibility_linklayer_wakeup)(void);
extern sli_bt_linklayer_wakeup_t *const sli_bt_host_adaptation_linklayer_wakeup;
void sli_ll_shm_save(void *address);
void *sli_ll_shm_get(void);

typedef void (*diag_ll_wakeup_cb_t)(void);

static volatile uint32_t diag_ll_raise_cb_ptr_last;
static volatile uint32_t diag_ll_raise_skipped_null_cb;
static volatile uint32_t diag_ll_raise_skipped_invalid_cb;
static volatile uint32_t diag_ll_raise_cb_ptr_first_nonzero;
static volatile uint32_t diag_ll_raise_cb_compat_ptr_last;
static volatile uint32_t diag_ll_raise_cb_host_adapt_ptr_last;
static volatile uint32_t diag_ll_raise_cb_original_last;
static volatile uint32_t diag_ll_raise_cb_trampoline_installs;
static volatile uint32_t diag_ll_raise_cb_trampoline_enter;
static volatile uint32_t diag_ll_raise_cb_trampoline_exit;
static volatile uint32_t diag_ll_raise_cb_fallback_pendsv_calls;

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

static uint32_t cb_to_u32(diag_ll_wakeup_cb_t cb)
{
  return (uint32_t)(uintptr_t)cb;
}

static void diag_ll_raise_wakeup_fallback_pendsv(void)
{
  SCB->ICSR = SCB_ICSR_PENDSVSET_Msk;
}

uint32_t silabs_bgapi_ll_raise_cb_ptr_last(void)
{
  return diag_ll_raise_cb_ptr_last;
}

uint32_t silabs_bgapi_ll_raise_skipped_null_cb(void)
{
  return diag_ll_raise_skipped_null_cb;
}

uint32_t silabs_bgapi_ll_raise_skipped_invalid_cb(void)
{
  return diag_ll_raise_skipped_invalid_cb;
}

uint32_t silabs_bgapi_ll_raise_cb_ptr_first_nonzero(void)
{
  return diag_ll_raise_cb_ptr_first_nonzero;
}

uint32_t silabs_bgapi_ll_raise_cb_compat_ptr_last(void)
{
  return diag_ll_raise_cb_compat_ptr_last;
}

uint32_t silabs_bgapi_ll_raise_cb_host_adapt_ptr_last(void)
{
  return diag_ll_raise_cb_host_adapt_ptr_last;
}

uint32_t silabs_bgapi_ll_raise_cb_original_last(void)
{
  return diag_ll_raise_cb_original_last;
}

uint32_t silabs_bgapi_ll_raise_cb_trampoline_installs(void)
{
  return diag_ll_raise_cb_trampoline_installs;
}

uint32_t silabs_bgapi_ll_raise_cb_trampoline_enter(void)
{
  return diag_ll_raise_cb_trampoline_enter;
}

uint32_t silabs_bgapi_ll_raise_cb_trampoline_exit(void)
{
  return diag_ll_raise_cb_trampoline_exit;
}

uint32_t silabs_bgapi_ll_raise_cb_fallback_pendsv_calls(void)
{
  return diag_ll_raise_cb_fallback_pendsv_calls;
}

void __wrap_sl_btctrl_raise_events(uint32_t events)
{
  volatile uint32_t *cb_ptr_slot =
    (volatile uint32_t *)(uintptr_t)&sli_bt_host_adaptation_compatibility_linklayer_wakeup;
  uint32_t cb_ptr = *cb_ptr_slot;
  uint32_t compat_cb_ptr = (uint32_t)(uintptr_t)sli_bt_host_adaptation_compatibility_linklayer_wakeup;
  uint32_t host_adapt_cb_ptr = (uint32_t)(uintptr_t)sli_bt_host_adaptation_linklayer_wakeup;

  diag_ll_raise_cb_compat_ptr_last = compat_cb_ptr;
  diag_ll_raise_cb_host_adapt_ptr_last = host_adapt_cb_ptr;
  diag_ll_raise_cb_ptr_last = cb_ptr;
  diag_ll_raise_cb_original_last = cb_ptr;
  if ((diag_ll_raise_cb_ptr_first_nonzero == 0u) && (cb_ptr != 0u)) {
    diag_ll_raise_cb_ptr_first_nonzero = cb_ptr;
  }
  silabs_bgapi_note_ll_raise(events);

  if (sli_ll_tasklet_ptr == 0) {
    sli_ll_shm_save(sli_ll_shm_get());
  }

  // Log-only wrap: do not install trampolines on a valid callback. Only recover
  // when the slot is null/invalid so sl_btctrl_raise_events can still PendSV.
  if (cb_ptr == 0u) {
    diag_ll_raise_skipped_null_cb++;
    *cb_ptr_slot = cb_to_u32(diag_ll_raise_wakeup_fallback_pendsv);
    diag_ll_raise_cb_fallback_pendsv_calls++;
    cb_ptr = *cb_ptr_slot;
  }
  if (!is_probably_valid_cb_ptr(cb_ptr)) {
    diag_ll_raise_skipped_invalid_cb++;
    *cb_ptr_slot = cb_to_u32(diag_ll_raise_wakeup_fallback_pendsv);
    diag_ll_raise_cb_fallback_pendsv_calls++;
    cb_ptr = *cb_ptr_slot;
    if (!is_probably_valid_cb_ptr(cb_ptr)) {
      return;
    }
  }

  __real_sl_btctrl_raise_events(events);
}
