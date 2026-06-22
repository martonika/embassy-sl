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
}

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

/// Transmit a packet buffer.
pub fn write_tx_fifo(data: &[u8]) -> Result<(), u32> {
    let handle = handle();
    let written = unsafe {
        rail::RAIL_WriteTxFifo(
            handle,
            data.as_ptr() as *mut u8,
            data.len() as u16,
            true,
        )
    };
    if written == 0 {
        return Err(u32::MAX);
    }
    let status = unsafe {
        rail::RAIL_StartTx(handle, 11, 0, core::ptr::null())
    };
    if status == 0 {
        Ok(())
    } else {
        Err(status)
    }
}

#[unsafe(no_mangle)]
extern "C" fn silabs_rail_on_event(_rail_handle: rail::RAIL_Handle_t, events: rail::RAIL_Events_t) {
    RAIL_EVENTS.fetch_or(events as u32, Ordering::Relaxed);
}
