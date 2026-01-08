//! Serial Peripheral Interface (SPI) driver using EUSART.
//!
//! This driver provides SPI master functionality using the EUSART peripheral
//! in synchronous mode for Silicon Labs EFR32 series MCUs.
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::spi::{Spi, Config, ClockMode};
//!
//! let config = Config::default();
//! let mut spi = Spi::new(p.EUSART1, p.PC_03, p.PC_01, p.PC_00, config);
//!
//! // Write data
//! spi.blocking_write(&[0x01, 0x02, 0x03]).unwrap();
//!
//! // Read data
//! let mut buf = [0u8; 4];
//! spi.blocking_read(&mut buf).unwrap();
//!
//! // Full-duplex transfer
//! let mut tx = [0x01, 0x02];
//! let mut rx = [0u8; 2];
//! spi.blocking_transfer(&mut rx, &tx).unwrap();
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

impl From<embedded_hal::spi::Mode> for ClockMode {
    fn from(mode: embedded_hal::spi::Mode) -> Self {
        use embedded_hal::spi::{Phase, Polarity};
        match (mode.polarity, mode.phase) {
            (Polarity::IdleLow, Phase::CaptureOnFirstTransition) => ClockMode::Mode0,
            (Polarity::IdleLow, Phase::CaptureOnSecondTransition) => ClockMode::Mode1,
            (Polarity::IdleHigh, Phase::CaptureOnFirstTransition) => ClockMode::Mode2,
            (Polarity::IdleHigh, Phase::CaptureOnSecondTransition) => ClockMode::Mode3,
        }
    }
}

/// SPI configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// Bit rate in bits per second.
    pub bitrate: u32,
    /// SPI clock mode (polarity and phase).
    pub clock_mode: ClockMode,
    /// Send MSB first if true, LSB first if false.
    pub msb_first: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bitrate: 1_000_000, // 1 MHz default
            clock_mode: ClockMode::Mode0,
            msb_first: true,
        }
    }
}

/// SPI error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// TX FIFO overflow.
    TxOverflow,
    /// RX FIFO overflow.
    RxOverflow,
    /// Other error.
    Other,
}

/// SPI driver using EUSART in synchronous master mode.
pub struct Spi<'d, T: Instance> {
    _phantom: PhantomData<&'d T>,
}

impl<'d, T: Instance> Spi<'d, T> {
    /// Create a new SPI driver with MISO (full-duplex).
    ///
    /// # Arguments
    /// * `eusart` - The EUSART peripheral instance
    /// * `sclk` - The SPI clock pin
    /// * `mosi` - The MOSI (TX) pin
    /// * `miso` - The MISO (RX) pin
    /// * `config` - SPI configuration
    pub fn new(
        _eusart: Peri<'d, T>,
        sclk: Peri<'d, impl GpioPin>,
        mosi: Peri<'d, impl GpioPin>,
        miso: Peri<'d, impl GpioPin>,
        config: Config,
    ) -> Self {
        Self::new_inner(sclk.into(), mosi.into(), Some(miso.into()), config)
    }

    /// Create a new TX-only SPI driver (no MISO).
    ///
    /// # Arguments
    /// * `eusart` - The EUSART peripheral instance
    /// * `sclk` - The SPI clock pin
    /// * `mosi` - The MOSI (TX) pin
    /// * `config` - SPI configuration
    pub fn new_txonly(
        _eusart: Peri<'d, T>,
        sclk: Peri<'d, impl GpioPin>,
        mosi: Peri<'d, impl GpioPin>,
        config: Config,
    ) -> Self {
        Self::new_inner(sclk.into(), mosi.into(), None, config)
    }

    fn new_inner(
        sclk: Peri<'d, AnyPin>,
        mosi: Peri<'d, AnyPin>,
        miso: Option<Peri<'d, AnyPin>>,
        config: Config,
    ) -> Self {
        let r = T::regs();

        // Reset EUSART to known state
        eusart_reset(r);

        // Configure CFG2 for SPI master mode
        r.cfg2().write(|w| {
            w.set_master(Master::MASTER);
            w.set_clkpol(config.clock_mode.polarity());
            w.set_clkpha(config.clock_mode.phase());
            w.set_forceload(true);
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
        let ref_freq = 20_000_000u32; // Assume 20 MHz (TODO: get from CMU)
        let sdiv = (ref_freq / config.bitrate).saturating_sub(1);
        let sdiv = sdiv.min(255) as u8;

        // Must disable peripheral before modifying CFG2.SDIV
        r.en().write(|w| w.set_en(false));
        while r.en().read().disabling() {}

        r.cfg2().modify(|w| {
            w.set_sdiv(sdiv);
        });

        // Re-enable peripheral
        r.en().write(|w| w.set_en(true));

        // Configure GPIO pins
        sclk.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
        mosi.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
        if let Some(ref miso_pin) = miso {
            miso_pin.mode_w(pac::gpio::vals::PortMode::INPUT);
        }

        // Configure GPIO routing
        configure_spi_pins::<T>(&sclk, &mosi, &miso);

        // Enable TX and RX
        eusart_sync(r, 0xFF);
        r.cmd().write(|w| {
            w.set_txen(true);
            w.set_rxen(true);
        });
        eusart_sync(r, 0x18);

        // Wait for TX/RX to be enabled
        while !r.status().read().txens() || !r.status().read().rxens() {}

        // Wait for idle
        while !r.status().read().txidle() {}

        Self {
            _phantom: PhantomData,
        }
    }

    /// Blocking write data to the SPI bus.
    pub fn blocking_write(&mut self, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();

        for &byte in data {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            r.txdata().write(|w| w.0 = byte as u32);

            // Wait for TX complete
            while !r.status().read().txc() {}

            // Discard received byte
            if r.status().read().rxfl() {
                let _ = r.rxdata().read();
            }
        }

        Ok(())
    }

    /// Blocking read data from the SPI bus.
    ///
    /// Transmits zeros and reads the received data.
    pub fn blocking_read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();

        for byte in buffer.iter_mut() {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            // Send dummy byte (0x00)
            r.txdata().write(|w| w.0 = 0);

            // Wait for RX data
            while !r.status().read().rxfl() {}

            *byte = r.rxdata().read().rxdata() as u8;
        }

        Ok(())
    }

    /// Blocking transfer data (full-duplex).
    pub fn blocking_transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Error> {
        let r = T::regs();

        let len = read.len().max(write.len());

        for i in 0..len {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            // Send byte or dummy
            let tx_byte = write.get(i).copied().unwrap_or(0);
            r.txdata().write(|w| w.0 = tx_byte as u32);

            // Wait for RX data
            while !r.status().read().rxfl() {}

            let rx_byte = r.rxdata().read().rxdata() as u8;
            if let Some(b) = read.get_mut(i) {
                *b = rx_byte;
            }
        }

        Ok(())
    }

    /// Blocking transfer in place (same buffer for read and write).
    pub fn blocking_transfer_in_place(&mut self, data: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();

        for byte in data.iter_mut() {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            r.txdata().write(|w| w.0 = *byte as u32);

            // Wait for RX data
            while !r.status().read().rxfl() {}

            *byte = r.rxdata().read().rxdata() as u8;
        }

        Ok(())
    }

    /// Flush the TX FIFO, waiting for all data to be transmitted.
    pub fn blocking_flush(&mut self) -> Result<(), Error> {
        let r = T::regs();

        // Wait for TX complete
        while !r.status().read().txc() {}

        Ok(())
    }

    /// Flush the RX FIFO by reading and discarding all data.
    pub fn rx_flush(&mut self) {
        let r = T::regs();

        while r.status().read().rxfl() {
            let _ = r.rxdata().read();
        }
    }
}

impl<'d, T: Instance> Drop for Spi<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();
        eusart_disable(r);
        deconfigure_pins::<T>();
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
        r.cmd().write(|w| {
            w.set_txdis(true);
            w.set_rxdis(true);
        });

        eusart_sync(r, 0x18);

        while r.status().read().txens() || r.status().read().rxens() {}

        r.en().write(|w| w.set_en(false));

        while r.en().read().disabling() {}
    }
}

/// Reset the EUSART to default state.
fn eusart_reset(r: pac::eusart::Eusart) {
    eusart_disable(r);

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

    r.if_().write(|w| w.0 = 0xFFFF_FFFF);

    r.clkdiv().write(|_| {});
}

/// Configure SPI pins for the EUSART.
fn configure_spi_pins<T: Instance>(
    sclk: &Peri<'_, AnyPin>,
    mosi: &Peri<'_, AnyPin>,
    miso: &Option<Peri<'_, AnyPin>>,
) {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    let sclk_port = sclk.pin_port() / 16;
    let sclk_pin = sclk.pin_port() % 16;
    let mosi_port = mosi.pin_port() / 16;
    let mosi_pin = mosi.pin_port() % 16;

    match T::index() {
        0 => {
            gpio.eusart0_txroute().write(|w| {
                w.set_port(mosi_port);
                w.set_pin(mosi_pin);
            });
            gpio.eusart0_sclkroute().write(|w| {
                w.set_port(sclk_port);
                w.set_pin(sclk_pin);
            });
            gpio.eusart0_routeen().modify(|w| {
                w.set_txpen(true);
                w.set_sclkpen(true);
            });

            if let Some(miso_pin) = miso {
                let miso_port = miso_pin.pin_port() / 16;
                let miso_pin_num = miso_pin.pin_port() % 16;
                gpio.eusart0_rxroute().write(|w| {
                    w.set_port(miso_port);
                    w.set_pin(miso_pin_num);
                });
                gpio.eusart0_routeen().modify(|w| w.set_rxpen(true));
            }
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
            gpio.eusart1_routeen().modify(|w| {
                w.set_txpen(true);
                w.set_sclkpen(true);
            });

            if let Some(miso_pin) = miso {
                let miso_port = miso_pin.pin_port() / 16;
                let miso_pin_num = miso_pin.pin_port() % 16;
                gpio.eusart1_rxroute().write(|w| {
                    w.set_port(miso_port);
                    w.set_pin(miso_pin_num);
                });
                gpio.eusart1_routeen().modify(|w| w.set_rxpen(true));
            }
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
                w.set_rxpen(false);
            });
        }
        1 => {
            gpio.eusart1_routeen().write(|w| {
                w.set_txpen(false);
                w.set_sclkpen(false);
                w.set_rxpen(false);
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
    fn index() -> u8;
}

/// EUSART peripheral instance trait for SPI.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {}

// ============================================================================
// embedded-hal trait implementations
// ============================================================================

impl embedded_hal::spi::Error for Error {
    fn kind(&self) -> embedded_hal::spi::ErrorKind {
        match self {
            Error::TxOverflow | Error::RxOverflow => embedded_hal::spi::ErrorKind::Overrun,
            Error::Other => embedded_hal::spi::ErrorKind::Other,
        }
    }
}

impl<'d, T: Instance> embedded_hal::spi::ErrorType for Spi<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_hal::spi::SpiBus for Spi<'d, T> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.blocking_read(words)
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.blocking_write(words)
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.blocking_transfer(read, write)
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.blocking_transfer_in_place(words)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.blocking_flush()
    }
}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for SPI peripherals (EUSART).
#[macro_export]
macro_rules! impl_spi {
    ($type:ident, $pac_type:ident, $index:expr) => {
        impl $crate::spi::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::eusart::Eusart {
                $crate::pac::$pac_type
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::spi::Instance for $crate::peripherals::$type {}
    };
}
