//! RAIL radio support (Silicon Labs proprietary stack via FFI).
//!
//! Enable the `rail` feature and set `SILABS_SDK` to your local Simplicity SDK.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use silabs_rail_sys as rail;

static RAIL_EVENTS: AtomicU32 = AtomicU32::new(0);
static RAIL_INITIALIZED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    fn silabs_rail_platform_init();
    fn silabs_rail_handle() -> rail::RAIL_Handle_t;
    fn silabs_rail_init_stage() -> u32;
    fn silabs_rail_init_status() -> u32;
    fn silabs_rail_start_carrier_wave(channel: u16) -> u32;
    fn silabs_rail_stop_tx_stream() -> u32;
    fn silabs_rail_tx_packet(channel: u16, data: *const u8, len: u16) -> u32;
}

/// RAIL event bit: packet transmitted successfully.
pub const EVENT_TX_PACKET_SENT: u32 = 1 << 24;
/// RAIL event bit: transmit aborted.
pub const EVENT_TX_ABORTED: u32 = 1 << 26;
/// RAIL event bit: TX FIFO underflow.
pub const EVENT_TX_UNDERFLOW: u32 = 1 << 30;

/// Initialize the RAIL platform (HFXO, clocks, RAIL library).
///
/// Uses IEEE 802.15.4 2.4 GHz built-in PHY with a 39 MHz HFXO configuration
/// (BRD4186C). For proprietary PHY, export `rail_config.c` from Simplicity Studio
/// and set `SILABS_RAIL_CONFIG_DIR` when building `silabs-csdk`.
pub fn init() {
    if RAIL_INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }
    unsafe { silabs_rail_platform_init() };
}

/// Raw RAIL handle after [`init`].
pub fn handle() -> rail::RAIL_Handle_t {
    unsafe { silabs_rail_handle() }
}

/// Last completed standalone RAIL initialization stage.
pub fn init_stage() -> u32 {
    unsafe { silabs_rail_init_stage() }
}

/// Status returned by the standalone RAIL initialization sequence.
pub fn init_status() -> u32 {
    unsafe { silabs_rail_init_status() }
}

/// Start an unmodulated carrier on an IEEE 802.15.4 channel.
pub fn start_carrier_wave(channel: u16) -> Result<(), u32> {
    let status = unsafe { silabs_rail_start_carrier_wave(channel) };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

/// Stop a carrier or PN9 stream and return the radio to idle.
pub fn stop_tx_stream() -> Result<(), u32> {
    let status = unsafe { silabs_rail_stop_tx_stream() };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

/// Write `data` to the TX FIFO and start an immediate transmit on `channel`.
pub fn tx_packet(channel: u16, data: &[u8]) -> Result<(), u32> {
    let status = unsafe {
        silabs_rail_tx_packet(channel, data.as_ptr(), data.len() as u16)
    };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

/// Poll and clear accumulated RAIL events.
pub fn take_events() -> u32 {
    RAIL_EVENTS.swap(0, Ordering::AcqRel)
}

/// Start receiving on the default 802.15.4 channel 11.
pub fn start_rx_channel_11() -> Result<(), u32> {
    let handle = handle();
    let status =
        unsafe { rail::RAIL_StartRx(handle, 11, core::ptr::null()) };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

/// Transmit a packet buffer on channel 11 (legacy helper).
pub fn write_tx_fifo(data: &[u8]) -> Result<(), u32> {
    tx_packet(11, data)
}

#[unsafe(no_mangle)]
extern "C" fn silabs_rail_on_event(_rail_handle: rail::RAIL_Handle_t, events: rail::RAIL_Events_t) {
    RAIL_EVENTS.fetch_or(events as u32, Ordering::Relaxed);
}
