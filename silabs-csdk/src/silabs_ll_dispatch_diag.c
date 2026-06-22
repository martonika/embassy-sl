#include <stdint.h>
#include <stddef.h>

#include "silabs_bgapi_debug.h"
#include "sl_status.h"

/* Host path from bluetooth_le_host/.../ubt/hci_adv.c */
typedef struct hci_command hci_command_t;
typedef struct hci_event hci_event_t;

extern hci_command_t *hci_command_init_shared(uint16_t opcode, uint8_t size);
extern hci_event_t *hci_command_shared_response(void);
extern void ll_hciCall(void (*cmd)(void));
extern void ll_hciCmdSetExtendedAdvertisingEnable(void);

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

static volatile uint32_t diag_hci_adv_enable_calls;
static volatile uint32_t diag_hci_adv_enable_on_calls;
static volatile uint32_t diag_hci_adv_enable_off_calls;
static volatile uint8_t diag_hci_adv_enable_last_num_sets;
static volatile uint8_t diag_hci_adv_enable_last_maxevents;
static volatile uint8_t diag_hci_adv_enable_last_handle;
static volatile uint32_t diag_usch_schedule_calls;
static volatile uint32_t diag_usch_schedule_req_calls;
static volatile uint8_t diag_usch_schedule_process_enabled = 1u;
static volatile uint8_t diag_usch_schedule_allow_real = 0u;
static volatile uint32_t diag_ll_adv_set_enable_calls;
static volatile uint32_t diag_ll_adv_set_enable_last;
static volatile uint32_t diag_ll_mbox_cb_calls;
static volatile uint32_t diag_usch_add_task_enter_calls;
static volatile uint32_t diag_usch_add_task_return_calls;
static volatile uint32_t diag_usch_add_task_skip_once_calls;
static volatile uint32_t diag_usch_add_task_last_task_ptr;
static volatile uint32_t diag_usch_add_task_last_next_ptr;
static volatile uint32_t diag_usch_add_task_last_schedule_ptr;
static volatile uint32_t diag_usch_add_task_last_start;
static volatile uint32_t diag_usch_add_task_last_init0;
static volatile uint32_t diag_usch_add_task_last_init1;
static volatile uint32_t diag_usch_add_task_last_min_runtime;
static volatile uint32_t diag_usch_add_task_last_max_runtime;
static volatile uint32_t diag_usch_add_task_last_handler_ptr;
static volatile uint8_t diag_usch_add_task_last_flags;
static volatile uint8_t diag_usch_add_task_last_priority;
static volatile uint16_t diag_usch_add_task_last_id;

uint32_t silabs_bgapi_hci_adv_enable_on_calls(void)
{
  return diag_hci_adv_enable_on_calls;
}

uint32_t silabs_bgapi_hci_adv_enable_off_calls(void)
{
  return diag_hci_adv_enable_off_calls;
}

uint8_t silabs_bgapi_hci_adv_enable_last_handle(void)
{
  return diag_hci_adv_enable_last_handle;
}

uint8_t silabs_bgapi_hci_adv_enable_last_num_sets(void)
{
  return diag_hci_adv_enable_last_num_sets;
}

uint32_t silabs_bgapi_hci_adv_enable_calls(void)
{
  return diag_hci_adv_enable_calls;
}

uint8_t silabs_bgapi_hci_adv_enable_last(void)
{
  return diag_hci_adv_enable_last_num_sets;
}

uint8_t silabs_bgapi_hci_adv_enable_last_maxevents(void)
{
  return diag_hci_adv_enable_last_maxevents;
}

uint32_t silabs_bgapi_usch_schedule_calls(void)
{
  return diag_usch_schedule_calls;
}

uint32_t silabs_bgapi_usch_schedule_req_calls(void)
{
  return diag_usch_schedule_req_calls;
}

void silabs_ble_scheduler_set_enabled(uint8_t enabled)
{
  diag_usch_schedule_process_enabled = enabled;
}

uint8_t silabs_ble_scheduler_enabled_read(void)
{
  return diag_usch_schedule_process_enabled;
}

void silabs_ble_schedule_allow_real(uint8_t allow)
{
  diag_usch_schedule_allow_real = allow;
}

uint8_t silabs_ble_schedule_allow_real_read(void)
{
  return diag_usch_schedule_allow_real;
}

uint32_t silabs_bgapi_ll_adv_set_enable_calls(void)
{
  return diag_ll_adv_set_enable_calls;
}

uint32_t silabs_bgapi_ll_adv_set_enable_last(void)
{
  return diag_ll_adv_set_enable_last;
}

uint32_t silabs_bgapi_ll_mbox_cb_calls(void)
{
  return diag_ll_mbox_cb_calls;
}

uint32_t silabs_bgapi_usch_add_task_enter_calls(void)
{
  return diag_usch_add_task_enter_calls;
}

uint32_t silabs_bgapi_usch_add_task_return_calls(void)
{
  return diag_usch_add_task_return_calls;
}

uint32_t silabs_bgapi_usch_add_task_skip_once_calls(void)
{
  return diag_usch_add_task_skip_once_calls;
}

uint32_t silabs_bgapi_usch_add_task_last_task_ptr(void)
{
  return diag_usch_add_task_last_task_ptr;
}

uint32_t silabs_bgapi_usch_add_task_last_next_ptr(void)
{
  return diag_usch_add_task_last_next_ptr;
}

uint32_t silabs_bgapi_usch_add_task_last_schedule_ptr(void)
{
  return diag_usch_add_task_last_schedule_ptr;
}

uint32_t silabs_bgapi_usch_add_task_last_start(void)
{
  return diag_usch_add_task_last_start;
}

uint32_t silabs_bgapi_usch_add_task_last_init0(void)
{
  return diag_usch_add_task_last_init0;
}

uint32_t silabs_bgapi_usch_add_task_last_init1(void)
{
  return diag_usch_add_task_last_init1;
}

uint32_t silabs_bgapi_usch_add_task_last_min_runtime(void)
{
  return diag_usch_add_task_last_min_runtime;
}

uint32_t silabs_bgapi_usch_add_task_last_max_runtime(void)
{
  return diag_usch_add_task_last_max_runtime;
}

uint32_t silabs_bgapi_usch_add_task_last_handler_ptr(void)
{
  return diag_usch_add_task_last_handler_ptr;
}

uint8_t silabs_bgapi_usch_add_task_last_flags(void)
{
  return diag_usch_add_task_last_flags;
}

uint8_t silabs_bgapi_usch_add_task_last_priority(void)
{
  return diag_usch_add_task_last_priority;
}

uint16_t silabs_bgapi_usch_add_task_last_id(void)
{
  return diag_usch_add_task_last_id;
}

/*
 * gap_adv.c:
 *   disable: hci_le_set_extended_advertising_enable(handle, 0, 0, 0)
 *   enable:  hci_le_set_extended_advertising_enable(handle, 1, duration, maxevents)
 * Second argument is the HCI enable flag (not number_of_sets).
 */

void __real_usch_ScheduleProcess(void);
void __real_usch_ScheduleReqCB(void);
sl_status_t __real_sli_ll_adv_set_advertising_enable(void *adv,
                                                      uint8_t enable,
                                                      uint16_t duration,
                                                      uint8_t maxevents);
void __real_sli_ll_mbox_message_cb(void);
void __real_ll_execTimingInit(void *timing, uint32_t ticks);
void __real_usch_AddTask(void *task);

typedef struct {
  void *next;
  uint32_t start;
  uint32_t init_time_word;
  uint32_t min_runtime;
  uint32_t max_runtime;
  uint8_t flags;
  uint8_t priority;
  uint16_t id;
  void *handler;
  void *schedule;
} diag_usch_task_t;

sl_status_t __wrap_hci_le_set_extended_advertising_enable(uint8_t handle,
                                                          uint8_t enable,
                                                          uint16_t duration,
                                                          uint8_t maxevents)
{
  const uint8_t number_of_sets = 1u;
  silabs_hci_ext_adv_enable_cmd_t *cmd;

  diag_hci_adv_enable_calls++;
  diag_hci_adv_enable_last_num_sets = enable;
  diag_hci_adv_enable_last_maxevents = maxevents;
  diag_hci_adv_enable_last_handle = handle;
  if (enable != 0u) {
    diag_hci_adv_enable_on_calls++;
  } else {
    diag_hci_adv_enable_off_calls++;
  }

  /*
   * Prebuilt libble_host.a (LTO) elides ll_hciCall between
   * hci_command_init_shared and hci_command_shared_response. Re-run the
   * source path from hci_adv.c + ll_hci_adv.c so the controller handler runs.
   */
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

void __wrap_usch_ScheduleProcess(void)
{
  diag_usch_schedule_calls++;
  if (diag_usch_schedule_process_enabled == 0u) {
    return;
  }
  if (diag_usch_schedule_allow_real == 0u) {
    return;
  }
  __real_usch_ScheduleProcess();
}

void __wrap_usch_ScheduleReqCB(void)
{
  diag_usch_schedule_req_calls++;
  __real_usch_ScheduleReqCB();
}

sl_status_t __wrap_sli_ll_adv_set_advertising_enable(void *adv,
                                                      uint8_t enable,
                                                      uint16_t duration,
                                                      uint8_t maxevents)
{
  sl_status_t status;

  diag_ll_adv_set_enable_calls++;
  status = __real_sli_ll_adv_set_advertising_enable(adv, enable, duration, maxevents);
  diag_ll_adv_set_enable_last = (uint32_t)status;
  return status;
}

void __wrap_sli_ll_mbox_message_cb(void)
{
  diag_ll_mbox_cb_calls++;
  __real_sli_ll_mbox_message_cb();
}

void __wrap_ll_execTimingInit(void *timing, uint32_t ticks)
{
  __real_ll_execTimingInit(timing, ticks);
}

void __wrap_usch_AddTask(void *task)
{
  diag_usch_task_t *t = (diag_usch_task_t *)task;
  diag_usch_add_task_enter_calls++;
  if (t != NULL) {
    diag_usch_add_task_last_task_ptr = (uint32_t)(uintptr_t)t;
    diag_usch_add_task_last_next_ptr = (uint32_t)(uintptr_t)t->next;
    diag_usch_add_task_last_schedule_ptr = (uint32_t)(uintptr_t)t->schedule;
    diag_usch_add_task_last_start = t->start;
    diag_usch_add_task_last_init0 = t->init_time_word;
    diag_usch_add_task_last_init1 = 0;
    diag_usch_add_task_last_min_runtime = t->min_runtime;
    diag_usch_add_task_last_max_runtime = t->max_runtime;
    diag_usch_add_task_last_handler_ptr = (uint32_t)(uintptr_t)t->handler;
    diag_usch_add_task_last_flags = t->flags;
    diag_usch_add_task_last_priority = t->priority;
    diag_usch_add_task_last_id = t->id;
  } else {
    diag_usch_add_task_last_task_ptr = 0;
    diag_usch_add_task_last_next_ptr = 0;
    diag_usch_add_task_last_schedule_ptr = 0;
    diag_usch_add_task_last_start = 0;
    diag_usch_add_task_last_init0 = 0;
    diag_usch_add_task_last_init1 = 0;
    diag_usch_add_task_last_min_runtime = 0;
    diag_usch_add_task_last_max_runtime = 0;
    diag_usch_add_task_last_handler_ptr = 0;
    diag_usch_add_task_last_flags = 0;
    diag_usch_add_task_last_priority = 0;
    diag_usch_add_task_last_id = 0;
  }

  __real_usch_AddTask(task);
  diag_usch_add_task_return_calls++;
}
