#include "em_cmu.h"
#include "em_device.h"

/*
 * Embassy time driver is disabled for bt-empty so RAIL can own SYSRTC SEQ.
 * Still start the free-running counter for sl_sleeptimer and polling delays.
 */
void silabs_sysrtc_start_counter(void)
{
  CMU_ClockEnable(cmuClock_SYSRTC, true);
  CMU_ClockSelectSet(cmuClock_SYSRTC, cmuSelect_LFRCO);

  if ((SYSRTC0->EN & SYSRTC_EN_EN) == 0U) {
    SYSRTC0->EN = SYSRTC_EN_EN;
  }
  SYSRTC0->CMD = SYSRTC_CMD_START;
}
