#include <stdint.h>

#include "silabs_ll_hci_post_service.h"

static volatile uint8_t silabs_ll_hci_post_service_enabled = 0;

void silabs_ll_hci_post_service_set(uint8_t enabled)
{
  silabs_ll_hci_post_service_enabled = enabled;
}

uint8_t silabs_ll_hci_post_service_get(void)
{
  return silabs_ll_hci_post_service_enabled;
}
