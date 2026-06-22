#ifndef SILABS_BLE_PLATFORM_H
#define SILABS_BLE_PLATFORM_H

#include <stdint.h>
#include "sl_status.h"

void silabs_ble_step(void);
void silabs_ble_step_app_timer(void);
void silabs_ble_step_sleeptimer(void);
void silabs_ble_step_bt_host(void);
void silabs_service_linklayer_events(void);
void silabs_ble_step_bt_pump(void);
void silabs_ble_step_bt_event(void);
void silabs_sl_bt_run_pump(void);
void silabs_ble_set_aggressive_ubt_run(uint8_t enable);
void silabs_ble_set_hci_unbounded(uint8_t enable);
void silabs_ble_set_use_real_ubt_run(uint8_t enable);
void silabs_ble_hci_drain(void);
void silabs_ble_before_sync_bgapi_command(void);
void silabs_ubt_run_pumped(void);
void silabs_ble_hci_pump_begin(void);
void silabs_ble_hci_pump_end(void);
void silabs_ble_adv_tick(void);
uint8_t silabs_ble_adv_in_progress(void);
uint8_t silabs_ble_adv_fsm_read(void);
void silabs_ble_post_stack_init_pump(uint32_t max_steps);
void silabs_ble_post_stack_init_pump_step(void);
void silabs_ble_post_stack_init_pump_finish(void);
uint8_t silabs_ble_post_init_pump_active_read(void);
void silabs_ble_set_skip_post_adv_pump(uint8_t skip);
void silabs_ble_scheduler_set_enabled(uint8_t enabled);
uint8_t silabs_ble_scheduler_enabled_read(void);
void silabs_ble_schedule_allow_real(uint8_t allow);
uint8_t silabs_ble_schedule_allow_real_read(void);
uint32_t silabs_ble_step_phase_read(void);
void silabs_ble_step_phase_write(uint32_t phase);
sl_status_t silabs_ble_start_connectable_advertising(void);
uint8_t silabs_ble_adv_handle_read(void);
uint8_t silabs_ble_pending_adv_start_read(void);
void silabs_ble_pending_adv_start_clear(void);
void silabs_ble_finish_pending_adv_start(void);

void silabs_ble_ensure_radio_irqs_enabled(void);

#endif
