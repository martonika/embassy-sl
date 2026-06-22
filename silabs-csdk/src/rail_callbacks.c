#include "rail.h"
#include "sl_rail_types.h"

void silabs_rail_on_event(RAIL_Handle_t rail_handle, RAIL_Events_t events);

__attribute__((weak)) void silabs_rail_on_event(RAIL_Handle_t rail_handle, RAIL_Events_t events)
{
    (void)rail_handle;
    (void)events;
}

RAIL_Status_t RAILCb_SetupRxFifo(RAIL_Handle_t railHandle)
{
    (void)railHandle;
    return RAIL_STATUS_NO_ERROR;
}

void RAILCb_ConfigFrameTypeLength(RAIL_Handle_t railHandle,
                                  const RAIL_FrameType_t *frameType)
{
    (void)railHandle;
    (void)frameType;
}

void sli_rail_util_on_event(sl_rail_handle_t rail_handle, sl_rail_events_t events)
{
    silabs_rail_on_event((RAIL_Handle_t)rail_handle, (RAIL_Events_t)events);
}
