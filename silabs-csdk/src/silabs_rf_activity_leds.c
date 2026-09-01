#include "silabs_rf_activity_leds.h"

#include "em_cmu.h"
#include "em_gpio.h"
#include "em_prs.h"
#include "sl_rf_activity_led_config.h"

void silabs_rf_activity_leds_init(void)
{
  CMU_ClockEnable(cmuClock_GPIO, true);
  CMU_ClockEnable(cmuClock_PRS, true);

  GPIO_PinModeSet(SL_RF_ACTIVITY_LED_TX_PORT,
                  SL_RF_ACTIVITY_LED_TX_PIN,
                  gpioModePushPull,
                  0);
  PRS_ConnectSignal(SL_RF_ACTIVITY_LED_TX_PRS_CH, prsTypeAsync, PRS_RACL_PAEN);
  PRS_PinOutput(SL_RF_ACTIVITY_LED_TX_PRS_CH,
                prsTypeAsync,
                SL_RF_ACTIVITY_LED_TX_PORT,
                SL_RF_ACTIVITY_LED_TX_PIN);

  GPIO_PinModeSet(SL_RF_ACTIVITY_LED_RX_PORT,
                  SL_RF_ACTIVITY_LED_RX_PIN,
                  gpioModePushPull,
                  0);
  PRS_ConnectSignal(SL_RF_ACTIVITY_LED_RX_PRS_CH, prsTypeAsync, PRS_RACL_LNAEN);
  PRS_PinOutput(SL_RF_ACTIVITY_LED_RX_PRS_CH,
                prsTypeAsync,
                SL_RF_ACTIVITY_LED_RX_PORT,
                SL_RF_ACTIVITY_LED_RX_PIN);
}
