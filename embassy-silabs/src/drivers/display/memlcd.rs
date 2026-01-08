//! Memory LCD SPI driver using EUSART in synchronous master mode.
//!
//! This driver provides SPI communication for Sharp/Silicon Labs memory LCD displays.
//! It configures the EUSART peripheral in SPI master mode with MSB-first transmission.
//!
//! # Example
//!
//! ```no_run
//! use embassy_silabs::drivers::display::memlcd::{MemLcdSpi, Config, ClockMode};
//!
//! let config = Config::default();
//! let mut spi = MemLcdSpi::new(p.EUSART1, p.PC_00, p.PC_01, config);
//!
//! spi.tx(&[0x01, 0x02, 0x03]).unwrap();
//! spi.wait();
//! ```
#![warn(missing_docs)]

use core::marker::PhantomData;

use embassy_hal_internal::{Peri, PeripheralType};

use crate::chip::pac;
use crate::gpio::{AnyPin, Pin as GpioPin, SealedPin as GpioSealedPin};

// GPIO peripheral access
use pac::GPIO;

// Re-export useful PAC types
pub use pac::eusart::vals::{Clkpha, Clkpol, Master, Msbf};

/// SPI clock mode combining polarity and phase.
///
/// Matches the C SDK's `eusart_ClockMode` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ClockMode {
    /// Clock idle low, sample on rising edge (CPOL=0, CPHA=0)
    #[default]
    Mode0,
    /// Clock idle low, sample on falling edge (CPOL=0, CPHA=1)
    Mode1,
    /// Clock idle high, sample on falling edge (CPOL=1, CPHA=0)
    Mode2,
    /// Clock idle high, sample on rising edge (CPOL=1, CPHA=1)
    Mode3,
}

impl ClockMode {
    /// Get the clock polarity for this mode.
    pub fn polarity(&self) -> Clkpol {
        match self {
            ClockMode::Mode0 | ClockMode::Mode1 => Clkpol::IDLELOW,
            ClockMode::Mode2 | ClockMode::Mode3 => Clkpol::IDLEHIGH,
        }
    }

    /// Get the clock phase for this mode.
    pub fn phase(&self) -> Clkpha {
        match self {
            ClockMode::Mode0 | ClockMode::Mode2 => Clkpha::SAMPLELEADING,
            ClockMode::Mode1 | ClockMode::Mode3 => Clkpha::SAMPLETRAILING,
        }
    }
}

/// Memory LCD SPI configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// Bit rate in bits per second.
    pub bitrate: u32,
    /// SPI clock mode (polarity and phase).
    pub clock_mode: ClockMode,
    /// Send MSB first (required for most memory LCDs).
    pub msb_first: bool,
    /// Reverse bits in each byte before transmission.
    /// Some memory LCDs require LSB-first bit order within each byte.
    pub reverse_bits: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bitrate: 1_100_000, // 1.1 MHz default for memory LCD
            clock_mode: ClockMode::Mode0,
            msb_first: true,
            reverse_bits: false,
        }
    }
}

/// Memory LCD SPI error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// TX FIFO overflow.
    TxOverflow,
}

/// Memory LCD SPI driver using EUSART in synchronous master mode.
pub struct MemLcdSpi<'d, T: Instance> {
    _phantom: PhantomData<&'d T>,
    reverse_bits: bool,
}

impl<'d, T: Instance> MemLcdSpi<'d, T> {
    /// Create a new memory LCD SPI driver.
    ///
    /// # Arguments
    /// * `eusart` - The EUSART peripheral instance
    /// * `sclk` - The SPI clock pin
    /// * `mosi` - The MOSI (TX) pin
    /// * `config` - SPI configuration
    pub fn new(
        _eusart: Peri<'d, T>,
        sclk: Peri<'d, impl GpioPin>,
        mosi: Peri<'d, impl GpioPin>,
        config: Config,
    ) -> Self {
        Self::new_inner(sclk.into(), mosi.into(), config)
    }

    fn new_inner(sclk: Peri<'d, AnyPin>, mosi: Peri<'d, AnyPin>, config: Config) -> Self {
        let r = T::regs();

        // Reset EUSART to known state
        eusart_reset(r);

        // Configure CFG2 for SPI master mode
        r.cfg2().write(|w| {
            w.set_master(Master::MASTER);
            w.set_clkpol(config.clock_mode.polarity());
            w.set_clkpha(config.clock_mode.phase());
            w.set_forceload(true); // Enable force load by default
        });

        // Configure CFG0 for synchronous mode
        r.cfg0().write(|w| {
            w.set_sync(pac::eusart::vals::Sync::SYNC);
            if config.msb_first {
                w.set_msbf(Msbf::ENABLE);
            } else {
                w.set_msbf(Msbf::DISABLE);
            }
        });

        // Configure frame format - 8 data bits for SPI
        r.framecfg().write(|w| {
            w.set_databits(pac::eusart::vals::Databits::EIGHT);
        });

        // Enable the peripheral
        r.en().write(|w| w.set_en(true));

        // Set baudrate for synchronous mode
        // In sync mode, SDIV in CFG2 controls the clock divider
        // bitrate = refFreq / (SDIV + 1)
        // So SDIV = (refFreq / bitrate) - 1
        let ref_freq = 20_000_000u32; // Assume 20 MHz for now (TODO: get from CMU)
        let sdiv = (ref_freq / config.bitrate).saturating_sub(1);
        let sdiv = sdiv.min(255) as u8; // SDIV is 8-bit

        // In sync mode, must disable peripheral before modifying CFG2.SDIV
        // (per Silicon Labs reference manual and C SDK EUSART_BaudrateSet)
        r.en().write(|w| w.set_en(false));
        while r.en().read().disabling() {}

        r.cfg2().modify(|w| {
            w.set_sdiv(sdiv);
        });

        // Re-enable peripheral after SDIV modification
        r.en().write(|w| w.set_en(true));

        // Configure GPIO pins as push-pull outputs
        sclk.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
        mosi.mode_w(pac::gpio::vals::PortMode::PUSHPULL);

        // Configure GPIO routing
        configure_spi_pins::<T>(&sclk, &mosi);

        // Enable TX (and RX for internal operation, though we don't use it)
        eusart_sync(r, 0xFF);
        r.cmd().write(|w| {
            w.set_txen(true);
            w.set_rxen(true);
        });
        eusart_sync(r, 0x18); // Wait for TXEN/RXEN

        // Wait for TX/RX to be enabled
        while !r.status().read().txens() || !r.status().read().rxens() {}

        // Wait for idle
        while !r.status().read().txidle() {}

        Self {
            _phantom: PhantomData,
            reverse_bits: config.reverse_bits,
        }
    }

    /// Transmit data to the memory LCD.
    ///
    /// Note: This function loads data into the TX FIFO. Use [`wait`](Self::wait)
    /// to ensure transmission is complete before toggling chip select.
    pub fn tx(&mut self, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();

        for &byte in data {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            let tx_byte = if self.reverse_bits {
                reverse_bits_u8(byte)
            } else {
                byte
            };

            r.txdata().write(|w| w.0 = tx_byte as u32);
        }

        Ok(())
    }

    /// Wait for all data to be transmitted (TX complete).
    ///
    /// Call this after [`tx`](Self::tx) to ensure all data has been shifted out
    /// before deasserting chip select.
    pub fn wait(&self) {
        let r = T::regs();

        // Wait for TX complete flag
        while !r.status().read().txc() {}
    }

    /// Flush the RX FIFO by reading and discarding all data.
    ///
    /// In SPI mode, data is received for every byte transmitted.
    /// Call this to clear the RX FIFO if needed.
    pub fn rx_flush(&self) {
        let r = T::regs();

        // Read data until RXFIFO is empty
        while r.status().read().rxfl() {
            let _ = r.rxdata().read();
        }
    }

    /// Shutdown the SPI peripheral.
    ///
    /// Disables the EUSART and turns off its clock.
    pub fn shutdown(self) {
        let r = T::regs();
        eusart_disable(r);
        deconfigure_pins::<T>();
    }

    /// Prepare for entering EM2/EM3 low power mode.
    ///
    /// Disables GPIO routing to prevent floating pins during sleep.
    pub fn enter_em23(&self) {
        let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

        match T::index() {
            0 => {
                gpio.eusart0_routeen().modify(|w| {
                    w.set_txpen(false);
                    w.set_sclkpen(false);
                });
            }
            1 => {
                gpio.eusart1_routeen().modify(|w| {
                    w.set_txpen(false);
                    w.set_sclkpen(false);
                });
            }
            _ => {}
        }
    }

    /// Exit from EM2/EM3 low power mode.
    ///
    /// Re-enables the EUSART and GPIO routing.
    pub fn exit_em23(&self) {
        let r = T::regs();
        let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

        // Re-enable EUSART
        r.en().write(|w| w.set_en(true));
        r.cmd().write(|w| {
            w.set_txen(true);
            w.set_rxen(true);
        });

        // Wait for TX/RX to be enabled
        while !r.status().read().txens() || !r.status().read().rxens() {}

        // Re-enable GPIO routing
        match T::index() {
            0 => {
                gpio.eusart0_routeen().modify(|w| {
                    w.set_txpen(true);
                    w.set_sclkpen(true);
                });
            }
            1 => {
                gpio.eusart1_routeen().modify(|w| {
                    w.set_txpen(true);
                    w.set_sclkpen(true);
                });
            }
            _ => {}
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Wait for SYNCBUSY register to clear specified bits.
#[inline]
fn eusart_sync(r: pac::eusart::Eusart, mask: u32) {
    while (r.syncbusy().read().0 & mask) != 0 {}
}

/// Properly disable the EUSART peripheral.
fn eusart_disable(r: pac::eusart::Eusart) {
    if r.en().read().en() {
        // 1. Disable TX and RX
        r.cmd().write(|w| {
            w.set_txdis(true);
            w.set_rxdis(true);
        });

        // 2. Wait for sync
        eusart_sync(r, 0x18); // RXDIS | TXDIS bits

        // 3. Wait for TX/RX to actually disable
        while r.status().read().txens() || r.status().read().rxens() {}

        // 4. Clear the enable bit
        r.en().write(|w| w.set_en(false));

        // 5. Wait for disabling to complete
        while r.en().read().disabling() {}
    }
}

/// Reset the EUSART to default state.
fn eusart_reset(r: pac::eusart::Eusart) {
    eusart_disable(r);

    // Reset all configuration registers to defaults
    r.cfg2().write(|_| {});
    r.cfg1().write(|_| {});
    r.cfg0().write(|_| {});
    r.framecfg().write(|_| {});
    r.dtxdatcfg().write(|_| {});
    r.timingcfg().write(|_| {});
    r.irhfcfg().write(|_| {});
    r.irlfcfg().write(|_| {});
    r.startframecfg().write(|_| {});
    r.sigframecfg().write(|_| {});
    r.trigctrl().write(|_| {});
    r.ien().write(|_| {});

    // Clear all interrupt flags
    r.if_().write(|w| w.0 = 0xFFFF_FFFF);

    r.clkdiv().write(|_| {});
}

/// Reverse bits in a byte (for LSB-first displays).
#[inline]
fn reverse_bits_u8(mut byte: u8) -> u8 {
    byte = (byte & 0xF0) >> 4 | (byte & 0x0F) << 4;
    byte = (byte & 0xCC) >> 2 | (byte & 0x33) << 2;
    byte = (byte & 0xAA) >> 1 | (byte & 0x55) << 1;
    byte
}

/// Configure SPI pins (SCLK and MOSI/TX) for the EUSART.
fn configure_spi_pins<T: Instance>(sclk: &Peri<'_, AnyPin>, mosi: &Peri<'_, AnyPin>) {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    let sclk_port = sclk.pin_port() / 16;
    let sclk_pin = sclk.pin_port() % 16;
    let mosi_port = mosi.pin_port() / 16;
    let mosi_pin = mosi.pin_port() % 16;

    match T::index() {
        0 => {
            // Set TX (MOSI) route
            gpio.eusart0_txroute().write(|w| {
                w.set_port(mosi_port);
                w.set_pin(mosi_pin);
            });
            // Set SCLK route
            gpio.eusart0_sclkroute().write(|w| {
                w.set_port(sclk_port);
                w.set_pin(sclk_pin);
            });
            // Enable TX and SCLK pins
            gpio.eusart0_routeen().write(|w| {
                w.set_txpen(true);
                w.set_sclkpen(true);
            });
        }
        1 => {
            gpio.eusart1_txroute().write(|w| {
                w.set_port(mosi_port);
                w.set_pin(mosi_pin);
            });
            gpio.eusart1_sclkroute().write(|w| {
                w.set_port(sclk_port);
                w.set_pin(sclk_pin);
            });
            gpio.eusart1_routeen().write(|w| {
                w.set_txpen(true);
                w.set_sclkpen(true);
            });
        }
        _ => {}
    }
}

/// Deconfigure pins when the driver is dropped.
fn deconfigure_pins<T: Instance>() {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    match T::index() {
        0 => {
            gpio.eusart0_routeen().write(|w| {
                w.set_txpen(false);
                w.set_sclkpen(false);
            });
        }
        1 => {
            gpio.eusart1_routeen().write(|w| {
                w.set_txpen(false);
                w.set_sclkpen(false);
            });
        }
        _ => {}
    }
}

// ============================================================================
// Instance trait
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::eusart::Eusart;
    /// Returns the EUSART instance index (0 for EUSART0, 1 for EUSART1, etc.)
    fn index() -> u8;
}

/// EUSART peripheral instance trait for memory LCD SPI.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for EUSART peripherals used with memory LCD.
#[macro_export]
macro_rules! impl_memlcd_spi {
    ($type:ident, $pac_type:ident, $index:expr) => {
        impl $crate::drivers::display::memlcd::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::eusart::Eusart {
                $crate::pac::$pac_type
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::drivers::display::memlcd::Instance for $crate::peripherals::$type {}
    };
}

// ============================================================================
// embedded-hal SPI trait implementations
// ============================================================================

impl embedded_hal::spi::Error for Error {
    fn kind(&self) -> embedded_hal::spi::ErrorKind {
        match self {
            Error::TxOverflow => embedded_hal::spi::ErrorKind::Overrun,
        }
    }
}

impl<'d, T: Instance> embedded_hal::spi::ErrorType for MemLcdSpi<'d, T> {
    type Error = Error;
}

