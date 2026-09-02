#include <stdint.h>
#include <stddef.h>
#include <string.h>

#include "sl_status.h"

/*
 * Functional Embassy glue (not diagnostics):
 * - Re-drive HCI ext-adv enable/params through ll_hciCall (libble_host LTO elides it)
 * - Gate usch_ScheduleProcess during deferred adv start to avoid init spin
 */

typedef struct hci_command hci_command_t;
typedef struct hci_event hci_event_t;

extern hci_command_t *hci_command_init_shared(uint16_t opcode, uint8_t size);
extern hci_event_t *hci_command_shared_response(void);
extern void ll_hciCall(void (*cmd)(void));
extern void ll_hciCmdSetExtendedAdvertisingEnable(void);
extern void ll_hciCmdSetExtendedAdvertisingParameters(void);
extern void __real_usch_ScheduleProcess(void);

#define HCI_Le_Set_Extended_Advertising_Parameters 0x2036u
#define HCI_Le_Set_Extended_Advertising_Enable 0x2039u
#define HCI_ERROR(X) ((X) ? ((sl_status_t)(X) | SL_STATUS_BLUETOOTH_CTRL_SPACE) : SL_STATUS_OK)

typedef struct {
  uint16_t opcode;
  uint8_t param_len;
  uint8_t enable;
  uint8_t number_of_sets;
  uint8_t set_handle;
  uint8_t duration_lo;
  uint8_t duration_hi;
  uint8_t max_events;
} silabs_hci_ext_adv_enable_cmd_t;

typedef struct __attribute__((packed)) {
  uint16_t opcode;
  uint8_t param_len;
  uint8_t handle;
  uint16_t event_properties;
  uint8_t interval_min[3];
  uint8_t interval_max[3];
  uint8_t primary_channel_map;
  uint8_t own_address_type;
  uint8_t peer_address_type;
  uint8_t peer_address[6];
  uint8_t filter_policy;
  int8_t tx_power;
  uint8_t primary_phy;
  uint8_t secondary_max_skip;
  uint8_t secondary_phy;
  uint8_t sid;
  uint8_t scan_request_notification_enable;
} silabs_hci_ext_adv_params_cmd_t;

static volatile uint8_t usch_schedule_process_enabled = 1u;
static volatile uint8_t usch_schedule_allow_real = 0u;

void silabs_ble_scheduler_set_enabled(uint8_t enabled)
{
  usch_schedule_process_enabled = enabled;
}

uint8_t silabs_ble_scheduler_enabled_read(void)
{
  return usch_schedule_process_enabled;
}

void silabs_ble_schedule_allow_real(uint8_t allow)
{
  usch_schedule_allow_real = allow;
}

uint8_t silabs_ble_schedule_allow_real_read(void)
{
  return usch_schedule_allow_real;
}

void silabs_ble_try_real_schedule_once(void)
{
  usch_schedule_allow_real = 1u;
  /* Call through the wrap so gating flags apply. */
  extern void usch_ScheduleProcess(void);
  usch_ScheduleProcess();
}

sl_status_t __wrap_hci_le_set_extended_advertising_enable(uint8_t handle,
                                                          uint8_t enable,
                                                          uint16_t duration,
                                                          uint8_t maxevents)
{
  const uint8_t number_of_sets = 1u;
  silabs_hci_ext_adv_enable_cmd_t *cmd;

  cmd = (silabs_hci_ext_adv_enable_cmd_t *)hci_command_init_shared(
    HCI_Le_Set_Extended_Advertising_Enable,
    (uint8_t)(2u + 4u * number_of_sets));
  cmd->enable = enable;
  cmd->number_of_sets = number_of_sets;
  cmd->set_handle = handle;
  cmd->duration_lo = (uint8_t)(duration & 0xffu);
  cmd->duration_hi = (uint8_t)(duration >> 8);
  cmd->max_events = maxevents;

  ll_hciCall(ll_hciCmdSetExtendedAdvertisingEnable);
  {
    const uint8_t *evt = (const uint8_t *)hci_command_shared_response();
    return HCI_ERROR(evt[5]);
  }
}

sl_status_t __wrap_hci_le_set_extended_advertising_parameters(
  uint8_t handle,
  uint16_t event_properties,
  uint32_t min_interval,
  uint32_t max_interval,
  uint8_t channel_map,
  uint8_t own_bdaddr_type,
  uint8_t peer_bdaddr_type,
  uint8_t *peer_bdaddr,
  uint8_t filter,
  int8_t tx_power,
  uint8_t primary_phy,
  uint8_t secondary_phy,
  uint8_t scan_request_notification_enable)
{
  silabs_hci_ext_adv_params_cmd_t *cmd;
  uint8_t peer[6] = { 0, 0, 0, 0, 0, 0 };

  if (peer_bdaddr != NULL) {
    memcpy(peer, peer_bdaddr, 6);
  }

  cmd = (silabs_hci_ext_adv_params_cmd_t *)hci_command_init_shared(
    HCI_Le_Set_Extended_Advertising_Parameters,
    (uint8_t)(sizeof(silabs_hci_ext_adv_params_cmd_t) - 3u));

  cmd->handle = handle;
  cmd->event_properties = event_properties;
  cmd->interval_min[0] = (uint8_t)(min_interval & 0xffu);
  cmd->interval_min[1] = (uint8_t)((min_interval >> 8) & 0xffu);
  cmd->interval_min[2] = (uint8_t)((min_interval >> 16) & 0xffu);
  cmd->interval_max[0] = (uint8_t)(max_interval & 0xffu);
  cmd->interval_max[1] = (uint8_t)((max_interval >> 8) & 0xffu);
  cmd->interval_max[2] = (uint8_t)((max_interval >> 16) & 0xffu);
  cmd->primary_channel_map = channel_map;
  cmd->own_address_type = own_bdaddr_type;
  cmd->peer_address_type = peer_bdaddr_type;
  memcpy(cmd->peer_address, peer, 6);
  cmd->filter_policy = filter;
  cmd->tx_power = tx_power;
  cmd->primary_phy = primary_phy;
  cmd->secondary_max_skip = 0;
  cmd->secondary_phy = secondary_phy;
  cmd->sid = handle;
  cmd->scan_request_notification_enable = scan_request_notification_enable;

  ll_hciCall(ll_hciCmdSetExtendedAdvertisingParameters);
  {
    const uint8_t *evt = (const uint8_t *)hci_command_shared_response();
    return HCI_ERROR(evt[5]);
  }
}

void __wrap_usch_ScheduleProcess(void)
{
  if (usch_schedule_process_enabled == 0u || usch_schedule_allow_real == 0u) {
    return;
  }
  __real_usch_ScheduleProcess();
}
