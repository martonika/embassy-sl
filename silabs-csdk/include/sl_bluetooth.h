#ifndef SL_BLUETOOTH_H
#define SL_BLUETOOTH_H

#include <stdbool.h>
#include <stdint.h>

#if defined(SILABS_CSDK_BTMESH)
#include "sl_btmesh.h"
#define SL_BT_COMPONENT_ADVERTISERS SL_BTMESH_COMPONENT_ADVERTISERS
#else
#define SL_BT_COMPONENT_ADVERTISERS (1)
#endif

#define SL_BT_COMPONENT_CONNECTIONS (1)

#include "sl_bluetooth_config.h"
#include "sl_bt_stack_init.h"
#include "sl_bt_api.h"

void sl_bt_init(void);
void sl_bt_step(void);
bool sl_bt_can_process_event(uint32_t len);
void sl_bt_process_event(sl_bt_msg_t *evt);
void sl_bt_on_event(sl_bt_msg_t *evt);

#endif
