#include <stdint.h>

#include "silabs_bgapi_debug.h"
#include "sl_rail.h"
#include "sl_rail_ble.h"

extern sl_rail_status_t __real_sl_rail_start_tx(sl_rail_handle_t rail_handle,
                                                 uint16_t channel,
                                                 sl_rail_tx_options_t tx_options,
                                                 const sl_rail_scheduler_info_t *p_scheduler_info);
extern sl_rail_status_t __real_sl_rail_init(sl_rail_handle_t *p_rail_handle,
                                            const sl_rail_config_t *p_rail_config,
                                            sl_rail_init_complete_callback_t init_complete_callback);
extern sl_rail_status_t __real_sl_rail_ble_init(sl_rail_handle_t rail_handle);
extern sl_rail_status_t __real_sl_rail_start_scheduled_tx(sl_rail_handle_t rail_handle,
                                                           uint16_t channel,
                                                           sl_rail_tx_options_t tx_options,
                                                           const sl_rail_scheduled_tx_config_t *p_scheduled_tx_config,
                                                           const sl_rail_scheduler_info_t *p_scheduler_info);
extern sl_rail_status_t __real_sl_rail_ble_config_channel_radio_params(sl_rail_handle_t rail_handle,
                                                                        const sl_rail_ble_state_t *p_ble_state);

static volatile uint32_t diag_rail_start_tx_calls;
static volatile uint32_t diag_rail_start_tx_last_status;
static volatile uint32_t diag_rail_start_tx_last_channel;
static volatile uint32_t diag_rail_start_tx_last_sched_null;

static volatile uint32_t diag_rail_start_scheduled_tx_calls;
static volatile uint32_t diag_rail_start_scheduled_tx_last_status;
static volatile uint32_t diag_rail_start_scheduled_tx_last_channel;
static volatile uint32_t diag_rail_start_scheduled_tx_last_when;
static volatile uint32_t diag_rail_start_scheduled_tx_last_mode;
static volatile uint32_t diag_rail_start_scheduled_tx_last_sched_null;

static volatile uint32_t diag_rail_ble_cfg_calls;
static volatile uint32_t diag_rail_ble_cfg_last_status;
static volatile uint32_t diag_rail_ble_cfg_last_channel;
static volatile uint32_t diag_rail_ble_cfg_last_access_address;
static volatile uint32_t diag_rail_ble_cfg_last_crc_init;
static volatile uint32_t diag_rail_ble_cfg_last_radio_state;

static volatile uint32_t diag_rail_init_calls;
static volatile uint32_t diag_rail_init_last_status;
static volatile uint32_t diag_rail_ble_init_calls;
static volatile uint32_t diag_rail_ble_init_last_status;

uint32_t silabs_bgapi_rail_start_tx_calls(void) { return diag_rail_start_tx_calls; }
uint32_t silabs_bgapi_rail_start_tx_last_status(void) { return diag_rail_start_tx_last_status; }
uint32_t silabs_bgapi_rail_start_tx_last_channel(void) { return diag_rail_start_tx_last_channel; }
uint32_t silabs_bgapi_rail_start_tx_last_sched_null(void) { return diag_rail_start_tx_last_sched_null; }

uint32_t silabs_bgapi_rail_start_scheduled_tx_calls(void) { return diag_rail_start_scheduled_tx_calls; }
uint32_t silabs_bgapi_rail_start_scheduled_tx_last_status(void) { return diag_rail_start_scheduled_tx_last_status; }
uint32_t silabs_bgapi_rail_start_scheduled_tx_last_channel(void) { return diag_rail_start_scheduled_tx_last_channel; }
uint32_t silabs_bgapi_rail_start_scheduled_tx_last_when(void) { return diag_rail_start_scheduled_tx_last_when; }
uint32_t silabs_bgapi_rail_start_scheduled_tx_last_mode(void) { return diag_rail_start_scheduled_tx_last_mode; }
uint32_t silabs_bgapi_rail_start_scheduled_tx_last_sched_null(void) { return diag_rail_start_scheduled_tx_last_sched_null; }

uint32_t silabs_bgapi_rail_ble_cfg_calls(void) { return diag_rail_ble_cfg_calls; }
uint32_t silabs_bgapi_rail_ble_cfg_last_status(void) { return diag_rail_ble_cfg_last_status; }
uint32_t silabs_bgapi_rail_ble_cfg_last_channel(void) { return diag_rail_ble_cfg_last_channel; }
uint32_t silabs_bgapi_rail_ble_cfg_last_access_address(void) { return diag_rail_ble_cfg_last_access_address; }
uint32_t silabs_bgapi_rail_ble_cfg_last_crc_init(void) { return diag_rail_ble_cfg_last_crc_init; }
uint32_t silabs_bgapi_rail_ble_cfg_last_radio_state(void) { return diag_rail_ble_cfg_last_radio_state; }
uint32_t silabs_bgapi_rail_init_calls(void) { return diag_rail_init_calls; }
uint32_t silabs_bgapi_rail_init_last_status(void) { return diag_rail_init_last_status; }
uint32_t silabs_bgapi_rail_ble_init_calls(void) { return diag_rail_ble_init_calls; }
uint32_t silabs_bgapi_rail_ble_init_last_status(void) { return diag_rail_ble_init_last_status; }

sl_rail_status_t __wrap_sl_rail_init(sl_rail_handle_t *p_rail_handle,
                                     const sl_rail_config_t *p_rail_config,
                                     sl_rail_init_complete_callback_t init_complete_callback)
{
  sl_rail_status_t st = __real_sl_rail_init(p_rail_handle, p_rail_config, init_complete_callback);
  diag_rail_init_calls++;
  diag_rail_init_last_status = (uint32_t)st;
  return st;
}

sl_rail_status_t __wrap_sl_rail_ble_init(sl_rail_handle_t rail_handle)
{
  sl_rail_status_t st = __real_sl_rail_ble_init(rail_handle);
  diag_rail_ble_init_calls++;
  diag_rail_ble_init_last_status = (uint32_t)st;
  return st;
}

sl_rail_status_t __wrap_sl_rail_start_tx(sl_rail_handle_t rail_handle,
                                         uint16_t channel,
                                         sl_rail_tx_options_t tx_options,
                                         const sl_rail_scheduler_info_t *p_scheduler_info)
{
  sl_rail_status_t st = __real_sl_rail_start_tx(rail_handle, channel, tx_options, p_scheduler_info);
  diag_rail_start_tx_calls++;
  diag_rail_start_tx_last_status = (uint32_t)st;
  diag_rail_start_tx_last_channel = (uint32_t)channel;
  diag_rail_start_tx_last_sched_null = (p_scheduler_info == NULL) ? 1u : 0u;
  return st;
}

sl_rail_status_t __wrap_sl_rail_start_scheduled_tx(sl_rail_handle_t rail_handle,
                                                    uint16_t channel,
                                                    sl_rail_tx_options_t tx_options,
                                                    const sl_rail_scheduled_tx_config_t *p_scheduled_tx_config,
                                                    const sl_rail_scheduler_info_t *p_scheduler_info)
{
  sl_rail_status_t st = __real_sl_rail_start_scheduled_tx(rail_handle,
                                                           channel,
                                                           tx_options,
                                                           p_scheduled_tx_config,
                                                           p_scheduler_info);
  diag_rail_start_scheduled_tx_calls++;
  diag_rail_start_scheduled_tx_last_status = (uint32_t)st;
  diag_rail_start_scheduled_tx_last_channel = (uint32_t)channel;
  diag_rail_start_scheduled_tx_last_when = (p_scheduled_tx_config != NULL) ? p_scheduled_tx_config->when : 0u;
  diag_rail_start_scheduled_tx_last_mode = (p_scheduled_tx_config != NULL) ? (uint32_t)p_scheduled_tx_config->mode : 0u;
  diag_rail_start_scheduled_tx_last_sched_null = (p_scheduler_info == NULL) ? 1u : 0u;
  return st;
}

sl_rail_status_t __wrap_sl_rail_ble_config_channel_radio_params(sl_rail_handle_t rail_handle,
                                                                 const sl_rail_ble_state_t *p_ble_state)
{
  sl_rail_status_t st = __real_sl_rail_ble_config_channel_radio_params(rail_handle, p_ble_state);
  diag_rail_ble_cfg_calls++;
  diag_rail_ble_cfg_last_status = (uint32_t)st;
  if (p_ble_state != NULL) {
    diag_rail_ble_cfg_last_channel = (uint32_t)p_ble_state->logical_channel;
    diag_rail_ble_cfg_last_access_address = p_ble_state->access_address;
    diag_rail_ble_cfg_last_crc_init = p_ble_state->crc_init;
  }
  diag_rail_ble_cfg_last_radio_state = (uint32_t)sl_rail_get_radio_state(rail_handle);
  return st;
}
