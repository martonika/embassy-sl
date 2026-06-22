#ifndef SL_BTMESH_H
#define SL_BTMESH_H

#include <stdbool.h>
#include <stdint.h>
#include "sl_btmesh_config.h"
#include "sl_bt_api.h"
#include "sl_btmesh_api.h"
#include "sl_btmesh_stack_init.h"
#include "sl_btmesh_bgapi.h"

#define SL_BTMESH_COMPONENT_ADVERTISERS (3 + SL_BTMESH_CONFIG_MAX_NETKEYS)
#define SL_BTMESH_FEATURE_BITMASK (3)

void sl_btmesh_init(void);
void sl_btmesh_step(void);
bool sl_btmesh_can_process_event(uint32_t len);
void sl_btmesh_process_event(sl_btmesh_msg_t *evt);
void sl_btmesh_on_event(sl_btmesh_msg_t *evt);

#endif
