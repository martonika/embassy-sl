#include "em_cmu.h"
#include "em_emu.h"
#include "sl_device_init_hfxo.h"
#include "sl_device_init_dcdc.h"
#include "sl_device_init_emu.h"
#include "sl_rail.h"
#include "sl_rail_util_protocol.h"
#include "sli_rail_util_callbacks.h"

extern const uint16_t sl_rail_builtin_rx_packet_queue_entries;
extern sl_rail_packet_queue_entry_t *const sl_rail_builtin_rx_packet_queue_ptr;

#define RX_FIFO_BYTES 512
#define TX_FIFO_BYTES 128

static sl_rail_handle_t rail_handle = SL_RAIL_EFR32_HANDLE;

SL_RAIL_DECLARE_FIFO_BUFFER(sli_rx_fifo_buffer, RX_FIFO_BYTES);
SL_RAIL_DECLARE_FIFO_BUFFER(sli_tx_fifo_buffer, TX_FIFO_BYTES);

static void platform_clock_init(void)
{
    CMU_ClockSelectSet(cmuClock_SYSCLK, cmuSelect_HFXO);
    CMU_ClockEnable(cmuClock_PRS, true);
    CMU_ClockEnable(cmuClock_LDMA, true);
}

static sl_rail_status_t rail_init_radio(void)
{
    sl_rail_status_t status;
    sl_rail_config_t rail_init_config = {
        .events_callback = &sli_rail_util_on_event,
        .rx_packet_queue_entries = sl_rail_builtin_rx_packet_queue_entries,
        .p_rx_packet_queue = sl_rail_builtin_rx_packet_queue_ptr,
        .rx_fifo_bytes = RX_FIFO_BYTES,
        .p_rx_fifo_buffer = sli_rx_fifo_buffer,
        .tx_fifo_bytes = TX_FIFO_BYTES,
        .p_tx_fifo_buffer = sli_tx_fifo_buffer,
    };

    status = sl_rail_init(&rail_handle, &rail_init_config, NULL);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    status = sl_rail_config_channels(rail_handle, NULL, NULL);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    status = sl_rail_util_protocol_config(rail_handle,
                                          SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    status = sl_rail_config_events(rail_handle,
                                   SL_RAIL_EVENTS_ALL,
                                   SL_RAIL_EVENT_RX_PACKET_RECEIVED
                                   | SL_RAIL_EVENT_TX_PACKET_SENT
                                   | SL_RAIL_EVENT_TX_ABORTED
                                   | SL_RAIL_EVENT_RX_PACKET_ABORTED
                                   | SL_RAIL_EVENT_RX_FIFO_OVERFLOW);
  return status;
}

void silabs_rail_platform_init(void)
{
    sl_device_init_hfxo();
    sl_device_init_dcdc();
    sl_device_init_emu();
    platform_clock_init();
#if !defined(SILABS_CSDK_BLE)
    (void)rail_init_radio();
#endif
}

sl_rail_handle_t silabs_rail_handle(void)
{
    return rail_handle;
}
