//! Bluetooth Low Energy (Silicon Labs BGAPI stack).
//!
//! Enable the `ble` feature and set `SILABS_SDK` to your local Simplicity SDK.
//! Uses single-protocol RAIL (not multiprotocol).

use core::sync::atomic::{AtomicBool, Ordering};

static BLE_INITIALIZED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" {
    fn silabs_ble_platform_init();
    fn silabs_ble_platform_init_step(step: u32);
    fn silabs_ble_step();
    fn silabs_ble_init_stage_read() -> u32;
}

/// Last completed stage in [`silabs_ble_platform_init`] (for post-mortem debug).
pub fn init_stage() -> u32 {
    unsafe { silabs_ble_init_stage_read() }
}

/// Number of [`init_step`] calls required for the current feature set.
#[cfg(feature = "btmesh")]
pub const INIT_STEPS: u32 = 12;
#[cfg(all(feature = "ble", not(feature = "btmesh")))]
pub const INIT_STEPS: u32 = 9;

/// Run one platform init step (see `INIT_STEPS`).
pub fn init_step(step: u32) {
    unsafe { silabs_ble_platform_init_step(step) };
}

/// Mark platform init complete (after stepping through [`INIT_STEPS`] manually).
pub fn mark_initialized() {
    BLE_INITIALIZED.store(true, Ordering::Release);
}

/// Initialize HFXO, memory manager, and the BLE host/controller stack.
pub fn init() {
    if BLE_INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }
    for step in 1..=INIT_STEPS {
        init_step(step);
    }
}

/// Pump the BLE host stack (call regularly from an Embassy task).
pub fn step() {
    unsafe { silabs_ble_step() };
}
