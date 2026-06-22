#include "sl_device_init_clocks.h"
#include "em_cmu.h"
#include "sl_status.h"

sl_status_t sl_device_init_clocks(void)
{
  CMU_ClockSelectSet(cmuClock_SYSCLK, cmuSelect_HFXO);
#if defined(_CMU_EM01GRPACLKCTRL_MASK)
  CMU_ClockSelectSet(cmuClock_EM01GRPACLK, cmuSelect_HFXO);
#endif
#if defined(_CMU_EM01GRPBCLKCTRL_MASK)
  CMU_ClockSelectSet(cmuClock_EM01GRPBCLK, cmuSelect_HFXO);
#endif
#if defined(_CMU_EM01GRPCCLKCTRL_MASK)
  CMU_ClockSelectSet(cmuClock_EM01GRPCCLK, cmuSelect_HFXO);
#endif
  CMU_ClockSelectSet(cmuClock_EM23GRPACLK, cmuSelect_LFRCO);
  CMU_ClockSelectSet(cmuClock_EM4GRPACLK, cmuSelect_LFRCO);
#if defined(RTCC_PRESENT)
  CMU_ClockSelectSet(cmuClock_RTCC, cmuSelect_LFRCO);
#endif
#if defined(SYSRTC_PRESENT)
  CMU_ClockEnable(cmuClock_SYSRTC, true);
  CMU_ClockSelectSet(cmuClock_SYSRTC, cmuSelect_LFRCO);
#endif
  CMU_ClockSelectSet(cmuClock_WDOG0, cmuSelect_LFRCO);
#if defined(WDOG_COUNT) && (WDOG_COUNT > 1)
  CMU_ClockSelectSet(cmuClock_WDOG1, cmuSelect_LFRCO);
#endif

  CMU_ClockEnable(cmuClock_PRS, true);
  CMU_ClockEnable(cmuClock_LDMA, true);

  return SL_STATUS_OK;
}
