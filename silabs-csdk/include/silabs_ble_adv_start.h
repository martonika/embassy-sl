#ifndef SILABS_BLE_ADV_START_H
#define SILABS_BLE_ADV_START_H

#include "sl_status.h"

void silabs_ble_post_adv_start_pump(uint32_t rounds);
void silabs_ble_startup_ll_pump(void);
void silabs_ble_on_connectable_adv_started(sl_status_t sc);

#endif
