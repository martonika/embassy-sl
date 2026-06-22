#include <stdint.h>

#include "sl_btctrl_linklayer.h"
#include "sl_btctrl_linklayer_defs.h"

extern sl_status_t __real_sl_btctrl_init_functional(struct sl_btctrl_config *config);

/*
 * Reference sl_btctrl_init.c sets SL_BTCTRL_CONFIG_FLAG_SYNCHRONIZE_TO_SLEEP_CLOCK
 * when SL_CATALOG_POWER_MANAGER_PRESENT. Embassy uses a minimal power stub, so set
 * the same controller flag here to match reference baremetal behavior.
 */
sl_status_t __wrap_sl_btctrl_init_functional(struct sl_btctrl_config *config)
{
  config->flags |= SL_BTCTRL_CONFIG_FLAG_SYNCHRONIZE_TO_SLEEP_CLOCK;
  return __real_sl_btctrl_init_functional(config);
}
