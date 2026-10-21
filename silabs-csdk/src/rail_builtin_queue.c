#include "sl_rail_types.h"

/* EFR32: power-of-2 entries in [8, 512]. */
#define RAIL_RX_PACKET_QUEUE_ENTRIES 16

static sl_rail_packet_queue_entry_t builtin_rx_packet_queue[RAIL_RX_PACKET_QUEUE_ENTRIES];

const uint16_t sl_rail_builtin_rx_packet_queue_entries = RAIL_RX_PACKET_QUEUE_ENTRIES;
sl_rail_packet_queue_entry_t *const sl_rail_builtin_rx_packet_queue_ptr = builtin_rx_packet_queue;
