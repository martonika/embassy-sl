#include "sl_rail.h"
#include "sl_rail_ieee802154.h"
#include "sl_rail_util_protocol.h"
#include "sl_rail_util_protocol_config.h"

static sl_rail_status_t protocol_config_ieee802154_2p4_ghz(sl_rail_handle_t handle)
{
    sl_rail_status_t status;
    sl_rail_ieee802154_config_t config = {
        .p_addresses = NULL,
        .ack_config = {
            .enable = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_AUTO_ACK_ENABLE,
            .ack_timeout_us = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_AUTO_ACK_TIMEOUT_US,
            .rx_transitions = {
                .success = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_AUTO_ACK_RX_TRANSITION_STATE,
                .error = SL_RAIL_RF_STATE_IDLE,
            },
            .tx_transitions = {
                .success = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_AUTO_ACK_TX_TRANSITION_STATE,
                .error = SL_RAIL_RF_STATE_IDLE,
            },
        },
        .timings = {
            .idle_to_tx = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_IDLE_TO_TX_US,
            .idle_to_rx = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_IDLE_TO_RX_US,
            .rx_to_tx = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_RX_TO_TX_US,
            .tx_to_rx = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_TX_TO_RX_US,
            .rxsearch_timeout =
                (SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_RX_SEARCH_TIMEOUT_AFTER_IDLE_ENABLE
                     ? SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_RX_SEARCH_TIMEOUT_AFTER_IDLE_US
                     : 0),
            .tx_to_rxsearch_timeout =
                (SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_RX_SEARCH_TIMEOUT_AFTER_TX_ENABLE
                     ? SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_TIMING_RX_SEARCH_TIMEOUT_AFTER_TX_US
                     : 0),
        },
        .frames_mask = (0U
                        | (SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_ACCEPT_BEACON_FRAME_ENABLE
                               ? SL_RAIL_IEEE802154_ACCEPT_BEACON_FRAMES
                               : 0U)
                        | (SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_ACCEPT_DATA_FRAME_ENABLE
                               ? SL_RAIL_IEEE802154_ACCEPT_DATA_FRAMES
                               : 0U)
                        | (SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_ACCEPT_ACK_FRAME_ENABLE
                               ? SL_RAIL_IEEE802154_ACCEPT_ACK_FRAMES
                               : 0U)
                        | (SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_ACCEPT_COMMAND_FRAME_ENABLE
                               ? SL_RAIL_IEEE802154_ACCEPT_COMMAND_FRAMES
                               : 0U)),
        .promiscuous_mode = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_PROMISCUOUS_MODE_ENABLE,
        .is_pan_coordinator = SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_PAN_COORDINATOR_ENABLE,
        .default_frame_pending_in_outgoing_acks =
            SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ_DEFAULT_FRAME_PENDING_STATE,
    };

    status = sl_rail_ieee802154_init(handle, &config);
    if (status == SL_RAIL_STATUS_NO_ERROR) {
        status = sl_rail_ieee802154_config_2p4_ghz_radio(handle);
    }
    if (status != SL_RAIL_STATUS_NO_ERROR) {
        (void)sl_rail_ieee802154_deinit(handle);
    } else {
        (void)sl_rail_set_pti_protocol(handle, SL_RAIL_PTI_PROTOCOL_802154);
    }
    return status;
}

sl_rail_status_t sl_rail_util_protocol_config(sl_rail_handle_t handle,
                                              sl_rail_util_protocol_type_t protocol)
{
    switch (protocol) {
    case SL_RAIL_UTIL_PROTOCOL_IEEE802154_2P4GHZ:
        return protocol_config_ieee802154_2p4_ghz(handle);
    default:
        return SL_RAIL_STATUS_INVALID_PARAMETER;
    }
}
