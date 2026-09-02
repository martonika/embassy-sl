#ifndef SILABS_BGAPI_DEBUG_H
#define SILABS_BGAPI_DEBUG_H

#include <stdint.h>
#include "sl_status.h"

/* Lightweight status notes used by init / adv start (not RF/RAIL instrumentation). */

uint32_t silabs_bgapi_bt_peek_len(void);
uint32_t silabs_bgapi_bt_step_count(void);
uint32_t silabs_bgapi_bt_event_count(void);
uint32_t silabs_bgapi_bt_last_event_id(void);
uint8_t silabs_bgapi_system_boot_seen(void);
uint32_t silabs_bgapi_post_init_pump_steps(void);
uint32_t silabs_bgapi_bt_start_status(void);
uint32_t silabs_bgapi_mesh_init_classes_status(void);
uint32_t silabs_bgapi_mesh_node_init_status(void);
uint32_t silabs_bgapi_ble_adv_setup_status(void);
uint32_t silabs_bgapi_ble_identity_status(void);
uint8_t silabs_bgapi_ble_system_boot_handler_done(void);
uint32_t silabs_bgapi_ble_on_event_called(void);

void silabs_bgapi_ble_identity_address_read(uint8_t out[6]);
uint8_t silabs_bgapi_ble_identity_address_type(void);
void silabs_bgapi_ble_on_air_adv_address_read(uint8_t out[6]);
uint8_t silabs_bgapi_ble_on_air_adv_address_type(void);
uint8_t silabs_bgapi_ble_on_air_adv_address_valid(void);

void silabs_bgapi_note_bt_event(uint32_t event_id);
void silabs_bgapi_note_bt_step(void);
void silabs_ble_force_shm_link(void);
void silabs_bgapi_note_bt_start_status(sl_status_t status);
void silabs_bgapi_note_mesh_init_classes_status(sl_status_t status);
void silabs_bgapi_note_mesh_node_init_status(sl_status_t status);
void silabs_bgapi_note_ble_identity_address(const uint8_t address[6], uint8_t addr_type);
void silabs_bgapi_note_ble_on_air_adv_address(const uint8_t address[6], uint8_t addr_type);
void silabs_bgapi_note_ble_scan_request(void);
uint32_t silabs_bgapi_ble_scan_request_count(void);
uint8_t silabs_bgapi_ble_adv_start_pump_done(void);
void silabs_bgapi_note_ble_adv_start_pump_done(void);
void silabs_bgapi_note_ble_identity_status(sl_status_t status);
void silabs_bgapi_note_ble_adv_setup_status(uint32_t status);
void silabs_bgapi_note_ble_system_boot_handler_done(uint8_t done);
void silabs_bgapi_note_ble_on_event_called(void);

sl_status_t silabs_btmesh_deferred_node_init(void);

#endif
