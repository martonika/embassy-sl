#include "em_cmu.h"
#include "em_device.h"
#include "sl_clock_manager.h"
#include "sl_interrupt_manager.h"
#include "sl_status.h"

sl_status_t sl_clock_manager_enable_bus_clock(sl_bus_clock_t clock)
{
  if (clock == SL_BUS_CLOCK_SYSRTC0) {
    CMU_ClockEnable(cmuClock_SYSRTC, true);
    CMU_ClockSelectSet(cmuClock_SYSRTC, cmuSelect_LFRCO);
  }
  return SL_STATUS_OK;
}

sl_status_t sl_clock_manager_get_clock_branch_frequency(sl_clock_branch_t clock_branch,
                                                        uint32_t *frequency)
{
  (void)clock_branch;
  if (frequency == NULL) {
    return SL_STATUS_NULL_POINTER;
  }
  *frequency = 32768U;
  return SL_STATUS_OK;
}

sl_status_t sl_clock_manager_get_clock_branch_precision(sl_clock_branch_t clock_branch,
                                                        uint16_t *precision)
{
  (void)clock_branch;
  if (precision == NULL) {
    return SL_STATUS_NULL_POINTER;
  }
  *precision = 500;
  return SL_STATUS_OK;
}

void sl_interrupt_manager_enable_irq(int32_t irq)
{
  if (irq >= 0) {
    NVIC_EnableIRQ((IRQn_Type)irq);
  }
}

void sl_interrupt_manager_clear_irq_pending(int32_t irq)
{
  if (irq >= 0) {
    NVIC_ClearPendingIRQ((IRQn_Type)irq);
  }
}
