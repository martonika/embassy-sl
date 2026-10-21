#include "silabs_btmesh_factory_reset.h"

#include "nvm3_default.h"
#include "nvm3.h"
#include "silabs_ble_platform.h"
#include "sl_btmesh_api.h"

void silabs_btmesh_factory_reset_pump(uint32_t steps)
{
  for (uint32_t i = 0; i < steps; i++) {
    silabs_ble_step();
  }
}

silabs_btmesh_factory_reset_result_t silabs_btmesh_perform_node_reset(void)
{
  silabs_btmesh_factory_reset_result_t result = {
    .node_reset = sl_btmesh_node_reset(),
    .nvm_erase = SL_STATUS_OK,
  };
  return result;
}

silabs_btmesh_factory_reset_result_t silabs_btmesh_perform_full_reset(void)
{
  silabs_btmesh_factory_reset_result_t result = {
    .node_reset = sl_btmesh_node_reset(),
    .nvm_erase = SL_STATUS_OK,
  };

#if defined(SILABS_CSDK_BTMESH)
  {
    Ecode_t ec = nvm3_eraseAll(nvm3_defaultHandle);
    result.nvm_erase = (ec == ECODE_NVM3_OK) ? SL_STATUS_OK : SL_STATUS_FAIL;
  }
#endif

  return result;
}
