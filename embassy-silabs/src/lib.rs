#![no_std]

pub mod gpio;

#[cfg_attr(feature = "_xg24", path = "chips/efr32xg24.rs")]
mod chip;

// Reexports
pub use chip::{Peripherals, peripherals};
pub use embassy_hal_internal::{Peri, PeripheralType};

/// Initialize the `embassy-silabs` HAL with the provided configuration.
///
/// This returns the peripheral singletons that can be used for creating drivers.
///
/// This should only be called once at startup, otherwise it panics.
pub fn init() -> Peripherals {
    // Do this first, so that it panics if user is calling `init` a second time
    // before doing anything important.
    let peripherals = Peripherals::take();

    peripherals
}
