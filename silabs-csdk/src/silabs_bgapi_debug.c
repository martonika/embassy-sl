#include "silabs_bgapi_debug.h"
#include "silabs_ble_platform.h"
#include "silabs_ll_hci_post_service.h"
#include "sl_btctrl_linklayer.h"
#include "sli_bgapi.h"
#include "sli_bt_api.h"
#include "sl_bt_api.h"
#include "sl_status.h"

#if defined(SILABS_CSDK_BTMESH)
#include "sl_btmesh_api.h"
#endif

static volatile uint32_t bgapi_bt_step_count;
static volatile uint32_t bgapi_bt_event_count;
static volatile uint32_t bgapi_bt_last_event_id;
static volatile uint8_t bgapi_system_boot_seen;
static volatile uint32_t bgapi_post_init_pump_steps;
static volatile uint32_t bgapi_bt_start_status;
static volatile uint32_t bgapi_mesh_init_classes_status;
static volatile uint32_t bgapi_mesh_node_init_status;
static volatile uint8_t bgapi_ble_identity_addr[6];
static volatile uint8_t bgapi_ble_identity_addr_type;
static volatile uint8_t bgapi_ble_on_air_adv_addr[6];
static volatile uint8_t bgapi_ble_on_air_adv_addr_type;
static volatile uint8_t bgapi_ble_on_air_adv_addr_valid;
static volatile uint32_t bgapi_ble_adv_setup_status;
static volatile uint32_t bgapi_ble_identity_status;
static volatile uint8_t bgapi_ble_system_boot_handler_done;
static volatile uint32_t bgapi_ble_on_event_called;
static volatile uint32_t bgapi_ble_scan_request_count;
static volatile uint8_t bgapi_ble_adv_start_pump_done;

extern void sli_ll_shm_save(void *address);
extern void *sli_ll_shm_get(void);

uint32_t silabs_bgapi_bt_peek_len(void)
{
  return (uint32_t)sli_bgapi_device_peek_event_len(sli_bt_bgapi_device);
}

uint32_t silabs_bgapi_bt_step_count(void)
{
  return bgapi_bt_step_count;
}

uint32_t silabs_bgapi_bt_event_count(void)
{
  return bgapi_bt_event_count;
}

uint32_t silabs_bgapi_bt_last_event_id(void)
{
  return bgapi_bt_last_event_id;
}

uint8_t silabs_bgapi_system_boot_seen(void)
{
  return bgapi_system_boot_seen;
}

uint32_t silabs_bgapi_post_init_pump_steps(void)
{
  return bgapi_post_init_pump_steps;
}

uint32_t silabs_bgapi_bt_start_status(void)
{
  return bgapi_bt_start_status;
}

uint32_t silabs_bgapi_mesh_init_classes_status(void)
{
  return bgapi_mesh_init_classes_status;
}

uint32_t silabs_bgapi_mesh_node_init_status(void)
{
  return bgapi_mesh_node_init_status;
}

uint32_t silabs_bgapi_ble_adv_setup_status(void)
{
  return bgapi_ble_adv_setup_status;
}

uint32_t silabs_bgapi_ble_identity_status(void)
{
  return bgapi_ble_identity_status;
}

uint8_t silabs_bgapi_ble_system_boot_handler_done(void)
{
  return bgapi_ble_system_boot_handler_done;
}

uint32_t silabs_bgapi_ble_on_event_called(void)
{
  return bgapi_ble_on_event_called;
}

void silabs_bgapi_ble_identity_address_read(uint8_t out[6])
{
  for (unsigned i = 0; i < 6; i++) {
    out[i] = bgapi_ble_identity_addr[i];
  }
}

uint8_t silabs_bgapi_ble_identity_address_type(void)
{
  return bgapi_ble_identity_addr_type;
}

void silabs_bgapi_ble_on_air_adv_address_read(uint8_t out[6])
{
  for (unsigned i = 0; i < 6; i++) {
    out[i] = bgapi_ble_on_air_adv_addr[i];
  }
}

uint8_t silabs_bgapi_ble_on_air_adv_address_type(void)
{
  return bgapi_ble_on_air_adv_addr_type;
}

uint8_t silabs_bgapi_ble_on_air_adv_address_valid(void)
{
  return bgapi_ble_on_air_adv_addr_valid;
}

void silabs_bgapi_note_ble_on_air_adv_address(const uint8_t address[6], uint8_t addr_type)
{
  for (unsigned i = 0; i < 6; i++) {
    bgapi_ble_on_air_adv_addr[i] = address[i];
  }
  bgapi_ble_on_air_adv_addr_type = addr_type;
  bgapi_ble_on_air_adv_addr_valid = 1u;
}

void silabs_bgapi_note_ble_identity_address(const uint8_t address[6], uint8_t addr_type)
{
  for (unsigned i = 0; i < 6; i++) {
    bgapi_ble_identity_addr[i] = address[i];
  }
  bgapi_ble_identity_addr_type = addr_type;
}

void silabs_bgapi_note_ble_adv_setup_status(uint32_t status)
{
  bgapi_ble_adv_setup_status = status;
}

void silabs_bgapi_note_ble_identity_status(sl_status_t status)
{
  bgapi_ble_identity_status = (uint32_t)status;
}

void silabs_bgapi_note_ble_system_boot_handler_done(uint8_t done)
{
  bgapi_ble_system_boot_handler_done = done;
}

void silabs_bgapi_note_ble_on_event_called(void)
{
  bgapi_ble_on_event_called++;
}

uint32_t silabs_bgapi_ble_scan_request_count(void)
{
  return bgapi_ble_scan_request_count;
}

void silabs_bgapi_note_ble_scan_request(void)
{
  bgapi_ble_scan_request_count++;
}

uint8_t silabs_bgapi_ble_adv_start_pump_done(void)
{
  return bgapi_ble_adv_start_pump_done;
}

void silabs_bgapi_note_ble_adv_start_pump_done(void)
{
  bgapi_ble_adv_start_pump_done = 1u;
}

void silabs_bgapi_note_bt_start_status(sl_status_t status)
{
  bgapi_bt_start_status = (uint32_t)status;
}

void silabs_bgapi_note_mesh_init_classes_status(sl_status_t status)
{
  bgapi_mesh_init_classes_status = (uint32_t)status;
}

void silabs_bgapi_note_mesh_node_init_status(sl_status_t status)
{
  bgapi_mesh_node_init_status = (uint32_t)status;
}

#if defined(SILABS_CSDK_BTMESH)
sl_status_t silabs_btmesh_deferred_node_init(void)
{
  sl_status_t status = sl_btmesh_node_init();
  silabs_bgapi_note_mesh_node_init_status(status);
  return status;
}
#endif

void silabs_bgapi_note_bt_event(uint32_t event_id)
{
  bgapi_bt_event_count++;
  bgapi_bt_last_event_id = event_id;
  if (event_id == (uint32_t)sl_bt_evt_system_boot_id) {
    bgapi_system_boot_seen = 1;
  }
}

void silabs_bgapi_note_bt_step(void)
{
  bgapi_bt_step_count++;
}

void silabs_ble_force_shm_link(void)
{
  sli_ll_shm_save(sli_ll_shm_get());
}

static volatile uint8_t silabs_ble_post_init_pump_active;

uint8_t silabs_ble_post_init_pump_active_read(void)
{
  return silabs_ble_post_init_pump_active;
}

void silabs_ble_post_stack_init_pump_step(void)
{
  if (silabs_ble_post_init_pump_active == 0u) {
    silabs_ble_post_init_pump_active = 1u;
    silabs_ble_scheduler_set_enabled(0u);
    silabs_ll_hci_post_service_set(1u);
  }
  bgapi_post_init_pump_steps++;
  silabs_ble_step_phase_write(1000u + bgapi_post_init_pump_steps);
  silabs_ble_step();
}

void silabs_ble_post_stack_init_pump_finish(void)
{
  silabs_ble_post_init_pump_active = 0u;
  silabs_ll_hci_post_service_set(0u);
  /* Queue adv enable without running usch_ScheduleProcess (can spin). */
  silabs_ble_scheduler_set_enabled(0u);
  silabs_ble_schedule_allow_real(0u);
  silabs_ble_set_skip_post_adv_pump(1u);
  silabs_ble_step_phase_write(48u);
  silabs_ble_finish_pending_adv_start();
  silabs_ble_set_skip_post_adv_pump(0u);
  silabs_ble_step_phase_write(49u);
}

void silabs_ble_post_stack_init_pump(uint32_t max_steps)
{
  for (uint32_t i = 0; i < max_steps; i++) {
    silabs_ble_post_stack_init_pump_step();
  }
  silabs_ble_post_stack_init_pump_finish();
}
