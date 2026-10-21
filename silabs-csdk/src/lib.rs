#![no_std]

// C support code compiled from the Silicon Labs SDK for RAIL.
// Set `SILABS_SDK` to your local Simplicity SDK install before building.

unsafe extern "C" {
    pub fn silabs_rail_platform_init();
    pub fn silabs_rail_handle() -> *mut core::ffi::c_void;
    pub fn silabs_rail_on_event(rail_handle: *mut core::ffi::c_void, events: u64);
}
