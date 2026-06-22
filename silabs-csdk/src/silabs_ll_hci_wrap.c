#include <stddef.h>
#include <stdint.h>

#include "silabs_ble_platform.h"
#include "silabs_ll_hci_post_service.h"

typedef void (*ll_hciCmdHandler)(void);

typedef struct {
  void *host_queue;
  void *shared_cmd;
  uint32_t reserved;
  ll_hciCmdHandler pending_handler;
} ll_hci_t;

extern ll_hci_t ll_hci;
extern void sli_ll_raise_events(uint32_t events);
void sl_bt_priority_handle(void);

/*
 * Wrap ll_hciCall with SDK-equivalent behavior:
 * - queue pending HCI command handler
 * - raise LL_EVENT_HCI_MESSAGE (0x80000000)
 * - execute command synchronously if still pending
 * Then optionally run post-service processing while enabled during boot.
 */
void __wrap_ll_hciCall(ll_hciCmdHandler cmd)
{
  ll_hci.pending_handler = cmd;
  sli_ll_raise_events(0x80000000u);
  __asm volatile("" ::: "memory");
  __asm volatile("" ::: "memory");
  __asm volatile("" ::: "memory");
  __asm volatile("" ::: "memory");
  if (ll_hci.pending_handler != NULL) {
    ll_hci.pending_handler();
    ll_hci.pending_handler = NULL;
  }
  silabs_service_linklayer_events();
  sl_bt_priority_handle();
}
