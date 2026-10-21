#ifndef SILABS_BTMESH_FACTORY_RESET_H
#define SILABS_BTMESH_FACTORY_RESET_H

#include <stdint.h>
#include "sl_status.h"

typedef struct {
  sl_status_t node_reset;
  sl_status_t nvm_erase;
} silabs_btmesh_factory_reset_result_t;

/// Mesh node reset only (does not erase NVM3).
silabs_btmesh_factory_reset_result_t silabs_btmesh_perform_node_reset(void);

/// Node reset plus NVM3 erase (SDK "full factory reset").
silabs_btmesh_factory_reset_result_t silabs_btmesh_perform_full_reset(void);

/// Pump BLE + mesh BGAPI events (call after reset commands, before reboot).
void silabs_btmesh_factory_reset_pump(uint32_t steps);

#endif
