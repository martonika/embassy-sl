#include <stddef.h>
#include <stdint.h>

#include "silabs_ll_hci_post_service.h"

typedef void (*ll_hciCmdHandler)(void);

/* Must match ll_hci.c layout (hostToLL, sharedCmd, sharedRsp, call). */
typedef struct {
  void *hostToLL;
  void *sharedCmd;
  void *sharedRsp;
  volatile ll_hciCmdHandler call;
} ll_hci_t;

extern ll_hci_t ll_hci;
extern void sli_ll_raise_events(uint32_t events);
extern void sl_bt_priority_handle(void);

/*
 * Silicon Labs ll_hci.c: queue the command handler, raise LL_EVENT_HCI_MESSAGE,
 * then invoke synchronously. Required for hci_le_set_extended_advertising_enable
 * and all other host->controller HCI commands on single-chip.
 */
void ll_hciCall(ll_hciCmdHandler cmd)
{
  ll_hci.call = cmd;
  sli_ll_raise_events(0x80000000u);
  __asm volatile("" ::: "memory");
  __asm volatile("" ::: "memory");
  __asm volatile("" ::: "memory");
  __asm volatile("" ::: "memory");
  if (ll_hci.call != NULL) {
    ll_hci.call();
    ll_hci.call = NULL;
  }
  if (silabs_ll_hci_post_service_get() != 0u) {
    sl_bt_priority_handle();
  }
}
