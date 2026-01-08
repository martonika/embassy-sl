//! Display drivers for Silicon Labs EFR32 series MCUs.
//!
//! This module contains drivers for various display types, starting with
//! memory LCD (MEMLCD) support using EUSART in SPI master mode.
//!
//! # Using with the LS013B7DH03 crate
//!
//! When the `memlcd-driver` feature is enabled, you can use the [`MemLcdSpi`](memlcd::MemLcdSpi)
//! driver with the [`ls013b7dh03`] crate for Sharp memory LCD displays:
//!
//! ```no_run,ignore
//! use embassy_silabs::drivers::display::memlcd::{MemLcdSpi, Config};
//! use embassy_silabs::drivers::display::ls013b7dh03::{Ls013b7dh03, BUF_SIZE};
//! use embassy_silabs::gpio::{Output, Level};
//!
//! // Create SPI driver (implements embedded_hal::spi::SpiBus)
//! let spi = MemLcdSpi::new(p.EUSART1, p.PC_00, p.PC_01, Config::default());
//!
//! // Create GPIO pins (implements embedded_hal::digital::OutputPin)
//! let cs = Output::new(p.PA_04, Level::Low);
//! let com_in = Output::new(p.PA_00, Level::Low);
//!
//! // Create display driver
//! let mut buffer = [0u8; BUF_SIZE];
//! let display = Ls013b7dh03::new(spi, cs, com_in, &mut buffer);
//!
//! // Use with embedded-graphics
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::primitives::{Circle, PrimitiveStyle};
//! use embedded_graphics::pixelcolor::BinaryColor;
//!
//! Circle::new(Point::new(50, 50), 20)
//!     .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
//!     .draw(&mut display)
//!     .unwrap();
//!
//! display.flush();
//! ```

pub mod memlcd;

/// Re-export the ls013b7dh03 crate when the `memlcd-driver` feature is enabled.
#[cfg(feature = "memlcd-driver")]
pub use ls013b7dh03;

/// Re-export the embedded-graphics crate when the `memlcd-driver` feature is enabled.
#[cfg(feature = "memlcd-driver")]
pub use embedded_graphics;
