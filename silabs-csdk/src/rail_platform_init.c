#include <string.h>

#include "em_cmu.h"
#include "em_emu.h"
#include "em_device.h"
#include "sl_device_init_hfxo.h"
#include "sl_device_init_dcdc.h"
#include "sl_device_init_emu.h"
#include "sl_status.h"
#include "sl_rail.h"
#include "sl_rail_util_protocol.h"
#include "sl_rail_util_compatible_pa.h"
#include "sli_rail_util_callbacks.h"

extern const uint16_t sl_rail_builtin_rx_packet_queue_entries;
extern sl_rail_packet_queue_entry_t *const sl_rail_builtin_rx_packet_queue_ptr;

#define RX_FIFO_BYTES 512
#define TX_FIFO_BYTES 512

static sl_rail_handle_t rail_handle = SL_RAIL_EFR32_HANDLE;
static volatile uint32_t rail_init_stage;
static volatile uint32_t rail_init_status;

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
    sl_rail_config_t rail_init_config;

    memset(&rail_init_config, 0, sizeof(rail_init_config));
    rail_init_config.events_callback = &sli_rail_util_on_event;
    rail_init_config.rx_packet_queue_entries = sl_rail_builtin_rx_packet_queue_entries;
    rail_init_config.p_rx_packet_queue = sl_rail_builtin_rx_packet_queue_ptr;
    rail_init_config.rx_fifo_bytes = RX_FIFO_BYTES;
    rail_init_config.p_rx_fifo_buffer = sli_rx_fifo_buffer;
    rail_init_config.tx_fifo_bytes = TX_FIFO_BYTES;
    rail_init_config.tx_fifo_init_bytes = 0;
    rail_init_config.p_tx_fifo_buffer = sli_tx_fifo_buffer;

    /* Substages 51..56 identify which RAIL call returns INVALID_PARAMETER. */
    rail_init_stage = 51;
    status = sl_rail_init(&rail_handle, &rail_init_config, NULL);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    rail_init_stage = 52;
    (void)sl_rail_config_channels(
        rail_handle,
        NULL,
        (sl_rail_radio_config_changed_callback_t)&sli_rail_util_on_channel_config_change);

    rail_init_stage = 53;
    status = sl_rail_util_protocol_config(rail_handle,
                                          SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    rail_init_stage = 54;
    status = sl_rail_util_pa_post_init(rail_handle, SL_RAIL_TX_PA_MODE_2P4_GHZ);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    rail_init_stage = 55;
    status = sl_rail_set_tx_power_dbm(rail_handle, 0);
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        return status;
    }

    rail_init_stage = 56;
    status = sl_rail_config_events(rail_handle,
                                   SL_RAIL_EVENTS_ALL,
                                   SL_RAIL_EVENT_RX_PACKET_RECEIVED
                                   | SL_RAIL_EVENT_TX_PACKET_SENT
                                   | SL_RAIL_EVENT_TX_ABORTED
                                   | SL_RAIL_EVENT_TX_UNDERFLOW
                                   | SL_RAIL_EVENT_RX_PACKET_ABORTED
                                   | SL_RAIL_EVENT_RX_FIFO_OVERFLOW);
    return status;
}

static void enable_radio_irqs(void)
{
    static const IRQn_Type irqs[] = {
        EMUDG_IRQn,
        AGC_IRQn,
        BUFC_IRQn,
        FRC_PRI_IRQn,
        FRC_IRQn,
        MODEM_IRQn,
        PROTIMER_IRQn,
        RAC_RSM_IRQn,
        RAC_SEQ_IRQn,
        HOSTMAILBOX_IRQn,
        SYNTH_IRQn,
        SYSRTC_SEQ_IRQn,
        RFECA0_IRQn,
        RFECA1_IRQn,
    };
    unsigned i;

    for (i = 0; i < (sizeof(irqs) / sizeof(irqs[0])); i++) {
        NVIC_ClearPendingIRQ(irqs[i]);
        NVIC_EnableIRQ(irqs[i]);
    }
}

void silabs_rail_platform_init(void)
{
    sl_status_t platform_status;

    rail_init_stage = 1;
    platform_status = sl_device_init_dcdc();
    if (platform_status != SL_STATUS_OK) {
        rail_init_status = (uint32_t)platform_status;
        return;
    }

    rail_init_stage = 2;
    platform_status = sl_device_init_hfxo();
    if (platform_status != SL_STATUS_OK) {
        rail_init_status = (uint32_t)platform_status;
        return;
    }

    rail_init_stage = 3;
    platform_clock_init();
    platform_status = sl_device_init_emu();
    if (platform_status != SL_STATUS_OK) {
        rail_init_status = (uint32_t)platform_status;
        return;
    }

    rail_init_stage = 4;
    sl_rail_util_pa_init();
#if !defined(SILABS_CSDK_BLE)
    rail_init_status = (uint32_t)rail_init_radio();
    if (rail_init_status != (uint32_t)SL_RAIL_STATUS_NO_ERROR) {
        return;
    }
    enable_radio_irqs();
#endif
    rail_init_stage = 6;
}

sl_rail_handle_t silabs_rail_handle(void)
{
    return rail_handle;
}

uint32_t silabs_rail_init_stage(void)
{
    return rail_init_stage;
}

uint32_t silabs_rail_init_status(void)
{
    return rail_init_status;
}

uint32_t silabs_rail_start_carrier_wave(uint16_t channel)
{
    if (rail_init_stage != 6u
        || rail_init_status != (uint32_t)SL_RAIL_STATUS_NO_ERROR) {
        return (uint32_t)SL_RAIL_STATUS_INVALID_STATE;
    }
    return (uint32_t)sl_rail_start_tx_stream(rail_handle,
                                             channel,
                                             SL_RAIL_STREAM_CARRIER_WAVE,
                                             SL_RAIL_TX_OPTIONS_DEFAULT);
}

uint32_t silabs_rail_stop_tx_stream(void)
{
    return (uint32_t)sl_rail_stop_tx_stream(rail_handle);
}

uint32_t silabs_rail_tx_packet(uint16_t channel, const uint8_t *data, uint16_t len)
{
    uint16_t written;

    if (rail_init_stage != 6u
        || rail_init_status != (uint32_t)SL_RAIL_STATUS_NO_ERROR
        || data == NULL
        || len == 0u) {
        return (uint32_t)SL_RAIL_STATUS_INVALID_CALL;
    }

    written = sl_rail_write_tx_fifo(rail_handle, data, len, true);
    if (written != len) {
        return (uint32_t)SL_RAIL_STATUS_INVALID_PARAMETER;
    }

    return (uint32_t)sl_rail_start_tx(rail_handle,
                                      channel,
                                      SL_RAIL_TX_OPTIONS_DEFAULT,
                                      NULL);
}
