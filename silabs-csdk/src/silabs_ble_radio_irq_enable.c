#include "em_device.h"
#include "silabs_bgapi_debug.h"

static void silabs_ble_enable_irq(IRQn_Type irq)
{
  if ((int32_t)irq < 0) {
    return;
  }
  NVIC_ClearPendingIRQ(irq);
  NVIC_EnableIRQ(irq);
}

void silabs_ble_ensure_radio_irqs_enabled(void)
{
  static const IRQn_Type irqs[] = {
    EMUDG_IRQn,
    AGC_IRQn,
    BUFC_IRQn,
    FRC_PRI_IRQn,
    FRC_IRQn,
    MODEM_IRQn,
    PROTIMER_IRQn,
    RAC_RSM_IRQn,
    RAC_SEQ_IRQn,
    HOSTMAILBOX_IRQn,
    SYNTH_IRQn,
    SYSRTC_SEQ_IRQn,
    SYSRTC_APP_IRQn,
    RFECA0_IRQn,
    RFECA1_IRQn,
  };
  unsigned i;

  for (i = 0; i < (sizeof(irqs) / sizeof(irqs[0])); i++) {
    silabs_ble_enable_irq(irqs[i]);
  }
}

void silabs_bgapi_note_radio_irq_enable_done(void)
{
}
