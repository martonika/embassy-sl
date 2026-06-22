/*
 * cortex-m-rt device.x names vectors without the _IRQHandler suffix (e.g. MODEM).
 * RAIL provides MODEM_IRQHandler. Thin wrappers wire vector table entries and
 * count IRQ entry for RF bring-up diagnostics.
 */
#include "silabs_bgapi_debug.h"

#define SILABS_IRQ_VECTOR_WRAPPER(name, note_fn)     \
  void name##_IRQHandler(void);                      \
  void name(void)                                    \
  {                                                  \
    note_fn();                                       \
    name##_IRQHandler();                             \
  }

SILABS_IRQ_VECTOR_WRAPPER(AGC, silabs_bgapi_note_irq_agc);
SILABS_IRQ_VECTOR_WRAPPER(BUFC, silabs_bgapi_note_irq_bufc);
SILABS_IRQ_VECTOR_WRAPPER(EMUDG, silabs_bgapi_note_irq_emudg);
SILABS_IRQ_VECTOR_WRAPPER(FRC_PRI, silabs_bgapi_note_irq_frc_pri);
SILABS_IRQ_VECTOR_WRAPPER(FRC, silabs_bgapi_note_irq_frc);
SILABS_IRQ_VECTOR_WRAPPER(MODEM, silabs_bgapi_note_irq_modem);
SILABS_IRQ_VECTOR_WRAPPER(PROTIMER, silabs_bgapi_note_irq_protimer);
SILABS_IRQ_VECTOR_WRAPPER(RAC_RSM, silabs_bgapi_note_irq_rac_rsm);
SILABS_IRQ_VECTOR_WRAPPER(RAC_SEQ, silabs_bgapi_note_irq_rac_seq);
SILABS_IRQ_VECTOR_WRAPPER(HOSTMAILBOX, silabs_bgapi_note_irq_hostmailbox);
SILABS_IRQ_VECTOR_WRAPPER(SYNTH, silabs_bgapi_note_irq_synth);
SILABS_IRQ_VECTOR_WRAPPER(RFECA0, silabs_bgapi_note_irq_rfeca0);
SILABS_IRQ_VECTOR_WRAPPER(RFECA1, silabs_bgapi_note_irq_rfeca1);
SILABS_IRQ_VECTOR_WRAPPER(SYSRTC_SEQ, silabs_bgapi_note_irq_sysrtc_seq);
SILABS_IRQ_VECTOR_WRAPPER(SYSRTC_APP, silabs_bgapi_note_irq_sysrtc_app);
