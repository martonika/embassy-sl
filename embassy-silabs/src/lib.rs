#![no_std]

mod macros;
mod time_driver;

pub mod drivers;
pub mod gpio;
//pub mod timer_32b;
pub mod time;
pub mod usart;

// This mod MUST go last, so that it sees all the `impl_foo!` macros
#[cfg_attr(feature = "_xg24", path = "chips/efr32xg24.rs")]
mod chip;

// Reexports
pub use chip::pac;
pub use chip::{Peripherals, peripherals};
pub use embassy_hal_internal::{Peri, PeripheralType};

pub use crate::chip::interrupt;
#[cfg(feature = "rt")]
pub use crate::pac::NVIC_PRIO_BITS;

/// Initialize the `embassy-silabs` HAL with the provided configuration.
///
/// This returns the peripheral singletons that can be used for creating drivers.
///
/// This should only be called once at startup, otherwise it panics.
pub fn init() -> Peripherals {
    // Do this first, so that it panics if user is calling `init` a second time
    // before doing anything important.
    let peripherals = Peripherals::take();

    // Enable clocks
    pac::CMU.clken0().modify(|w| {
        w.set_gpio(true);
        w.set_sysrtc0(true);
        w.set_lfrco(true);
        w.set_usart0(true);
    });
    // EUSART clocks are in CLKEN1
    pac::CMU.clken1().modify(|w| {
        w.set_eusart0(true);
        w.set_eusart1(true);
    });

    time_driver::init(crate::interrupt::Priority::P0);
    

    peripherals
}
