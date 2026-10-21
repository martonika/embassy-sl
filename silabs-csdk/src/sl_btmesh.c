#include "silabs_bgapi_debug.h"
#include "sl_btmesh.h"
#include "sl_btmesh_memory_config.h"
#include "sl_component_catalog.h"
#include "sl_btmesh_provisionee.h"
#include "sl_status.h"

extern const mesh_memory_config_t __mesh_memory_config;
sl_status_t mesh_memory_config_init(const mesh_memory_config_t *config);

static const struct sli_bgapi_class *const btmesh_class_table[] = {
  SL_BTMESH_BGAPI_CLASS(health_server),
  SL_BTMESH_BGAPI_CLASS(proxy),
  SL_BTMESH_BGAPI_CLASS(proxy_server),
  SL_BTMESH_BGAPI_CLASS(node),
  NULL
};

void mesh_advertiser_legacy(void);

void sl_btmesh_init(void)
{
  sl_status_t status = sl_btmesh_init_classes(btmesh_class_table);
  silabs_bgapi_note_mesh_init_classes_status(status);
  (void)mesh_memory_config_init(&__mesh_memory_config);
  mesh_advertiser_legacy();
}

void sl_btmesh_process_event(sl_btmesh_msg_t *evt)
{
  sl_btmesh_provisionee_on_event(evt);
  sl_btmesh_on_event(evt);
}

__attribute__((weak)) bool sl_btmesh_can_process_event(uint32_t len)
{
  (void)len;
  return true;
}

void sl_btmesh_step(void)
{
  sl_btmesh_msg_t evt;
  uint32_t event_len = sl_btmesh_event_pending_len();
  if ((event_len == 0) || (!sl_btmesh_can_process_event(event_len))) {
    return;
  }

  sl_status_t status = sl_btmesh_pop_event(&evt);
  if (status != SL_STATUS_OK) {
    return;
  }
  sl_btmesh_process_event(&evt);
}
