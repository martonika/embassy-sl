/*
 * Embassy override: found before SDK bluetooth_le_host/config via include path.
 * include_next loads the SDK header, then we override NVM3 identity address.
 */
#include_next "sl_bluetooth_config.h"

#undef SL_BT_CONFIG_SET_CUSTOM_ADDRESS_FROM_NVM3
#define SL_BT_CONFIG_SET_CUSTOM_ADDRESS_FROM_NVM3 (0)
