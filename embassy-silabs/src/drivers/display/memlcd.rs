//! Memory LCD driver using EUSART in synchronous master mode.
//!
//! This driver provides both low-level SPI communication and high-level display driver
//! for Sharp/Silicon Labs memory LCD displays (LS013B7DH03 and similar).
//!
//! # High-level MemLcd Driver
//!
//! The [`MemLcd`] driver provides a complete solution with:
//! - Integrated framebuffer
//! - Automatic EXTCOMIN toggling (configurable)
//! - `embedded-graphics` `DrawTarget` support
//!
//! ```no_run,ignore
//! use embassy_silabs::drivers::display::memlcd::{MemLcd, MemLcdConfig, SpiConfig};
//! use embassy_silabs::gpio::Output;
//!
//! let spi_config = SpiConfig::default();
//! let mut lcd_config = MemLcdConfig::default();
//! lcd_config.extcomin_auto_toggle = true; // Enable auto EXTCOMIN toggling
//!
//! let mut display = MemLcd::new(
//!     p.EUSART1, p.PC_03, p.PC_01,    // SPI: eusart, sclk, mosi
//!     p.PC_08, p.PC_09, p.PC_06,       // Pins: cs, enable, extcomin
//!     spi_config,
//!     lcd_config,
//! );
//!
//! display.power_on();
//! display.clear_hw().await;
//!
//! // Auto EXTCOMIN is handled internally - just call toggle_extcomin_tick()
//! // periodically if auto_toggle is disabled, or let the driver handle it.
//! ```
//!
//! # Low-level SPI Driver
//!
//! For more control, use [`MemLcdSpi`] directly:
//!
//! ```no_run,ignore
//! use embassy_silabs::drivers::display::memlcd::{MemLcdSpi, SpiConfig};
//!
//! let config = SpiConfig::default();
//! let mut spi = MemLcdSpi::new(p.EUSART1, p.PC_00, p.PC_01, config);
//!
//! spi.tx(&[0x01, 0x02, 0x03]).unwrap();
//! spi.wait();
//! ```
#![warn(missing_docs)]

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use embassy_hal_internal::{Peri, PeripheralType};

use crate::chip::pac;
use crate::gpio::{AnyPin, Level, Output, Pin as GpioPin, SealedPin as GpioSealedPin};

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

/// SPI configuration for the memory LCD.
///
/// Renamed from `Config` to `SpiConfig` for clarity when used with [`MemLcdConfig`].
pub type SpiConfig = Config;

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

impl<'d, T: Instance> embedded_hal::spi::SpiBus for MemLcdSpi<'d, T> {
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        let r = T::regs();

        for word in words.iter_mut() {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            // Send dummy byte (0xFF is common for SPI reads)
            let tx_byte = if self.reverse_bits {
                reverse_bits_u8(0xFF)
            } else {
                0xFF
            };
            r.txdata().write(|w| w.0 = tx_byte as u32);

            // Wait for RX FIFO to have data
            while !r.status().read().rxfl() {}

            // Read received byte
            let rx_byte = r.rxdata().read().0 as u8;
            *word = if self.reverse_bits {
                reverse_bits_u8(rx_byte)
            } else {
                rx_byte
            };
        }

        Ok(())
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.tx(words)?;
        // Wait for transmission to complete
        self.wait();
        // Discard any received data
        self.rx_flush();
        Ok(())
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        let r = T::regs();

        // Handle mismatched buffer lengths - use the shorter one
        let len = read.len().min(write.len());

        for i in 0..len {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            // Send byte
            let tx_byte = if self.reverse_bits {
                reverse_bits_u8(write[i])
            } else {
                write[i]
            };
            r.txdata().write(|w| w.0 = tx_byte as u32);

            // Wait for RX FIFO to have data
            while !r.status().read().rxfl() {}

            // Read received byte
            let rx_byte = r.rxdata().read().0 as u8;
            read[i] = if self.reverse_bits {
                reverse_bits_u8(rx_byte)
            } else {
                rx_byte
            };
        }

        // If write buffer is longer, send remaining bytes and discard RX
        for &byte in &write[len..] {
            while !r.status().read().txfl() {}
            let tx_byte = if self.reverse_bits {
                reverse_bits_u8(byte)
            } else {
                byte
            };
            r.txdata().write(|w| w.0 = tx_byte as u32);
            while !r.status().read().rxfl() {}
            let _ = r.rxdata().read();
        }

        // If read buffer is longer, send dummy bytes and read remaining
        for word in &mut read[len..] {
            while !r.status().read().txfl() {}
            let tx_byte = if self.reverse_bits {
                reverse_bits_u8(0xFF)
            } else {
                0xFF
            };
            r.txdata().write(|w| w.0 = tx_byte as u32);
            while !r.status().read().rxfl() {}
            let rx_byte = r.rxdata().read().0 as u8;
            *word = if self.reverse_bits {
                reverse_bits_u8(rx_byte)
            } else {
                rx_byte
            };
        }

        Ok(())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        let r = T::regs();

        for word in words.iter_mut() {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            // Send byte
            let tx_byte = if self.reverse_bits {
                reverse_bits_u8(*word)
            } else {
                *word
            };
            r.txdata().write(|w| w.0 = tx_byte as u32);

            // Wait for RX FIFO to have data
            while !r.status().read().rxfl() {}

            // Read received byte back into the same location
            let rx_byte = r.rxdata().read().0 as u8;
            *word = if self.reverse_bits {
                reverse_bits_u8(rx_byte)
            } else {
                rx_byte
            };
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.wait();
        Ok(())
    }
}

// ============================================================================
// High-level MemLcd driver
// ============================================================================

/// LS013B7DH03 Memory LCD display dimensions
pub const DISPLAY_WIDTH: usize = 128;
/// LS013B7DH03 Memory LCD display height  
pub const DISPLAY_HEIGHT: usize = 128;
/// Framebuffer size in bytes (1 bit per pixel)
pub const FRAMEBUFFER_SIZE: usize = DISPLAY_WIDTH / 8 * DISPLAY_HEIGHT;

// Display commands
const CMD_UPDATE: u8 = 0x01;
const CMD_ALL_CLEAR: u8 = 0x04;

// SCS timing (from datasheet)
const SCS_SETUP_US: u32 = 6;
const SCS_HOLD_US: u32 = 2;

/// Configuration for the high-level MemLcd driver.
#[derive(Clone)]
#[non_exhaustive]
pub struct MemLcdConfig {
    /// Enable automatic EXTCOMIN toggling.
    ///
    /// When enabled, call [`MemLcd::extcomin_auto_tick`] in your main loop or
    /// periodically from a timer. The driver will toggle EXTCOMIN at the
    /// configured frequency.
    ///
    /// When disabled, you must call [`MemLcd::toggle_extcomin`] manually
    /// at approximately 60Hz to prevent display static buildup.
    ///
    /// Default: `true`
    pub extcomin_auto_toggle: bool,

    /// EXTCOMIN toggle frequency in Hz.
    ///
    /// The display requires EXTCOMIN to be toggled at approximately 60Hz
    /// to prevent static buildup. This setting controls the auto-toggle
    /// frequency when `extcomin_auto_toggle` is enabled.
    ///
    /// Default: `60` (60 Hz)
    pub extcomin_frequency_hz: u32,
}

impl Default for MemLcdConfig {
    fn default() -> Self {
        Self {
            extcomin_auto_toggle: true,
            extcomin_frequency_hz: 60,
        }
    }
}

/// Static flag to control EXTCOMIN auto-toggle from the background task
static EXTCOMIN_AUTO_ENABLED: AtomicBool = AtomicBool::new(false);

/// High-level Memory LCD driver with integrated framebuffer and EXTCOMIN handling.
///
/// This driver provides a complete solution for Sharp LS013B7DH03 and similar
/// memory LCD displays. It includes:
///
/// - Integrated framebuffer
/// - Automatic or manual EXTCOMIN toggling
/// - Power management (enable/disable)
/// - `embedded-graphics` `DrawTarget` support (when feature enabled)
///
/// # EXTCOMIN Handling
///
/// The display requires the EXTCOMIN signal to be toggled at ~60Hz to prevent
/// static image burn-in. This driver supports two modes:
///
/// 1. **External task** (recommended): Use [`new_without_extcomin`](Self::new_without_extcomin)
///    and spawn [`extcomin_task_owned`] separately with its own pin.
///
/// 2. **Integrated**: Use [`new`](Self::new) and call [`toggle_extcomin`](Self::toggle_extcomin)
///    periodically from your code.
pub struct MemLcd<'d, T: Instance> {
    spi: MemLcdSpi<'d, T>,
    cs: Output<'d>,
    enable: Output<'d>,
    extcomin: Option<Output<'d>>,
    /// Framebuffer: 1 bit per pixel, 128x128 = 2048 bytes
    /// Note: 0 = black (pixel on), 1 = white (pixel off) for this display
    framebuffer: [u8; FRAMEBUFFER_SIZE],
    config: MemLcdConfig,
}

impl<'d, T: Instance> MemLcd<'d, T> {
    /// Create a new high-level MemLcd driver with integrated EXTCOMIN control.
    ///
    /// Use this constructor if you want to manage EXTCOMIN toggling via the
    /// [`toggle_extcomin`](Self::toggle_extcomin) method.
    ///
    /// # Arguments
    /// * `eusart` - The EUSART peripheral instance
    /// * `sclk` - SPI clock pin
    /// * `mosi` - SPI MOSI (data) pin
    /// * `cs` - Chip select pin (active high)
    /// * `enable` - Display enable pin (DISP_ENABLE)
    /// * `extcomin` - COM inversion signal pin (DISP_EXTCOMIN)
    /// * `spi_config` - SPI configuration
    /// * `config` - MemLcd configuration
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        eusart: Peri<'d, T>,
        sclk: Peri<'d, impl GpioPin>,
        mosi: Peri<'d, impl GpioPin>,
        cs: Peri<'d, impl GpioPin>,
        enable: Peri<'d, impl GpioPin>,
        extcomin: Peri<'d, impl GpioPin>,
        spi_config: Config,
        config: MemLcdConfig,
    ) -> Self {
        // Set the global flag for the background task
        EXTCOMIN_AUTO_ENABLED.store(config.extcomin_auto_toggle, Ordering::SeqCst);

        Self {
            spi: MemLcdSpi::new(eusart, sclk, mosi, spi_config),
            cs: Output::new(cs, Level::Low),
            enable: Output::new(enable, Level::Low),
            extcomin: Some(Output::new(extcomin, Level::Low)),
            framebuffer: [0xFF; FRAMEBUFFER_SIZE], // Start with white (all bits set)
            config,
        }
    }

    /// Create a new high-level MemLcd driver without EXTCOMIN control.
    ///
    /// Use this constructor when you want to manage EXTCOMIN separately,
    /// for example by spawning [`extcomin_task_owned`] as a background task.
    ///
    /// # Arguments
    /// * `eusart` - The EUSART peripheral instance
    /// * `sclk` - SPI clock pin
    /// * `mosi` - SPI MOSI (data) pin
    /// * `cs` - Chip select pin (active high)
    /// * `enable` - Display enable pin (DISP_ENABLE)
    /// * `spi_config` - SPI configuration
    /// * `config` - MemLcd configuration (extcomin settings are ignored)
    pub fn new_without_extcomin(
        eusart: Peri<'d, T>,
        sclk: Peri<'d, impl GpioPin>,
        mosi: Peri<'d, impl GpioPin>,
        cs: Peri<'d, impl GpioPin>,
        enable: Peri<'d, impl GpioPin>,
        spi_config: Config,
        config: MemLcdConfig,
    ) -> Self {
        // Set the global flag for the background task
        EXTCOMIN_AUTO_ENABLED.store(config.extcomin_auto_toggle, Ordering::SeqCst);

        Self {
            spi: MemLcdSpi::new(eusart, sclk, mosi, spi_config),
            cs: Output::new(cs, Level::Low),
            enable: Output::new(enable, Level::Low),
            extcomin: None,
            framebuffer: [0xFF; FRAMEBUFFER_SIZE], // Start with white (all bits set)
            config,
        }
    }

    /// Power on the display by setting DISP_ENABLE high.
    pub fn power_on(&mut self) {
        self.enable.set_high();
    }

    /// Power off the display by setting DISP_ENABLE low.
    ///
    /// This gives control back to the board controller on dev boards.
    pub fn power_off(&mut self) {
        self.enable.set_low();
    }

    /// Check if the display is powered on.
    pub fn is_powered(&self) -> bool {
        self.enable.is_set_high()
    }

    /// Toggle the EXTCOMIN signal.
    ///
    /// Call this at approximately 60Hz to prevent display static buildup.
    /// Only works if the driver was created with [`new`](Self::new).
    /// Does nothing if created with [`new_without_extcomin`](Self::new_without_extcomin).
    pub fn toggle_extcomin(&mut self) {
        if let Some(ref mut extcomin) = self.extcomin {
            extcomin.toggle();
        }
    }

    /// Enable or disable automatic EXTCOMIN toggling.
    ///
    /// This can be changed at runtime. When enabled, the background task
    /// spawned with [`extcomin_task`] will toggle the pin automatically.
    pub fn set_extcomin_auto_toggle(&mut self, enabled: bool) {
        self.config.extcomin_auto_toggle = enabled;
        EXTCOMIN_AUTO_ENABLED.store(enabled, Ordering::SeqCst);
    }

    /// Check if automatic EXTCOMIN toggling is enabled.
    pub fn is_extcomin_auto_toggle(&self) -> bool {
        self.config.extcomin_auto_toggle
    }

    /// Get the configured EXTCOMIN frequency in Hz.
    pub fn extcomin_frequency_hz(&self) -> u32 {
        self.config.extcomin_frequency_hz
    }

    /// Clear the entire display using hardware clear command.
    ///
    /// This sends the clear command to the display and also clears
    /// the internal framebuffer to white.
    pub fn clear_hw(&mut self) {
        // Assert CS
        self.cs.set_high();

        // SCS setup time - busy wait for microseconds
        cortex_m::asm::delay(SCS_SETUP_US * 20); // ~20 cycles per us at 20MHz

        // Send clear command (2 bytes: command + dummy)
        let cmd: [u8; 2] = [CMD_ALL_CLEAR, 0x00];
        self.spi.tx(&cmd).unwrap();
        self.spi.wait();

        // SCS hold time
        cortex_m::asm::delay(SCS_HOLD_US * 20);

        // Deassert CS
        self.cs.set_low();

        // Flush any RX garbage
        self.spi.rx_flush();

        // Also clear the framebuffer to white
        self.framebuffer.fill(0xFF);
    }

    /// Clear the framebuffer to white without updating the display.
    ///
    /// Call [`flush`](Self::flush) to update the display with the cleared buffer.
    pub fn clear_buffer(&mut self) {
        self.framebuffer.fill(0xFF);
    }

    /// Fill the framebuffer with a solid color without updating the display.
    ///
    /// * `white` - If true, fill with white (0xFF). If false, fill with black (0x00).
    ///
    /// Call [`flush`](Self::flush) to update the display with the filled buffer.
    pub fn fill_buffer(&mut self, white: bool) {
        let pattern = if white { 0xFF } else { 0x00 };
        self.framebuffer.fill(pattern);
    }

    /// Flush the framebuffer to the display.
    ///
    /// This transfers the entire framebuffer content to the display.
    pub fn flush_display(&mut self) {
        let row_len = DISPLAY_WIDTH / 8; // 16 bytes per row

        // Assert CS
        self.cs.set_high();

        // SCS setup time
        cortex_m::asm::delay(SCS_SETUP_US * 20);

        // Send update command with first line address
        let mut line_addr: u8 = 1;
        let cmd: [u8; 2] = [CMD_UPDATE, line_addr];
        self.spi.tx(&cmd).unwrap();

        for row in 0..DISPLAY_HEIGHT {
            // Send pixel data for this line
            let start = row * row_len;
            let end = start + row_len;
            self.spi.tx(&self.framebuffer[start..end]).unwrap();

            // Send dummy data or next line address
            if row == DISPLAY_HEIGHT - 1 {
                // Last line: send dummy bytes
                let dummy: [u8; 2] = [0xFF, 0xFF];
                self.spi.tx(&dummy).unwrap();
            } else {
                // Next line address
                line_addr += 1;
                let next_line: [u8; 2] = [0xFF, line_addr];
                self.spi.tx(&next_line).unwrap();
            }
        }

        self.spi.wait();

        // SCS hold time
        cortex_m::asm::delay(SCS_HOLD_US * 20);

        // Deassert CS
        self.cs.set_low();

        // Flush RX
        self.spi.rx_flush();
    }

    /// Get a reference to the internal framebuffer.
    pub fn framebuffer(&self) -> &[u8; FRAMEBUFFER_SIZE] {
        &self.framebuffer
    }

    /// Get a mutable reference to the internal framebuffer.
    ///
    /// Use this for direct framebuffer manipulation. Call [`flush`](Self::flush)
    /// after modifying to update the display.
    pub fn framebuffer_mut(&mut self) -> &mut [u8; FRAMEBUFFER_SIZE] {
        &mut self.framebuffer
    }

    /// Get a reference to the underlying SPI driver.
    pub fn spi(&self) -> &MemLcdSpi<'d, T> {
        &self.spi
    }

    /// Get a mutable reference to the underlying SPI driver.
    pub fn spi_mut(&mut self) -> &mut MemLcdSpi<'d, T> {
        &mut self.spi
    }

    /// Get a mutable reference to the EXTCOMIN output pin.
    ///
    /// Use this if you need direct control over the EXTCOMIN pin,
    /// for example to set up a hardware timer toggle.
    ///
    /// Returns `None` if the driver was created with
    /// [`new_without_extcomin`](Self::new_without_extcomin).
    pub fn extcomin_pin_mut(&mut self) -> Option<&mut Output<'d>> {
        self.extcomin.as_mut()
    }

    /// Check if this driver has an integrated EXTCOMIN pin.
    pub fn has_extcomin(&self) -> bool {
        self.extcomin.is_some()
    }
}

// ============================================================================
// embedded-graphics DrawTarget implementation
// ============================================================================

impl<T: Instance> embedded_graphics_core::geometry::OriginDimensions for MemLcd<'_, T> {
    fn size(&self) -> embedded_graphics_core::geometry::Size {
        embedded_graphics_core::geometry::Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)
    }
}

impl<T: Instance> embedded_graphics_core::draw_target::DrawTarget for MemLcd<'_, T> {
    type Color = embedded_graphics_core::pixelcolor::BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics_core::Pixel<Self::Color>>,
    {
        use embedded_graphics_core::pixelcolor::BinaryColor;
        use embedded_graphics_core::Pixel;

        for Pixel(point, color) in pixels {
            // Check bounds
            if point.x >= 0
                && point.x < DISPLAY_WIDTH as i32
                && point.y >= 0
                && point.y < DISPLAY_HEIGHT as i32
            {
                let x = point.x as usize;
                let y = point.y as usize;

                // Calculate byte index and bit position
                let byte_idx = y * (DISPLAY_WIDTH / 8) + (x / 8);
                let bit_idx = x % 8;

                // For this display: 0 = black (on), 1 = white (off)
                // BinaryColor::On = black, BinaryColor::Off = white
                match color {
                    BinaryColor::On => {
                        // Set pixel to black (clear bit)
                        self.framebuffer[byte_idx] &= !(1 << bit_idx);
                    }
                    BinaryColor::Off => {
                        // Set pixel to white (set bit)
                        self.framebuffer[byte_idx] |= 1 << bit_idx;
                    }
                }
            }
        }
        Ok(())
    }
}

// ============================================================================
// EXTCOMIN background task
// ============================================================================

/// Background task for automatic EXTCOMIN toggling.
///
/// Spawn this task to automatically toggle the EXTCOMIN signal at the
/// configured frequency. The task respects the `extcomin_auto_toggle`
/// setting and can be paused/resumed by calling
/// [`MemLcd::set_extcomin_auto_toggle`].
///
/// # Example
///
/// ```no_run,ignore
/// use embassy_silabs::drivers::display::memlcd::{MemLcd, extcomin_task};
///
/// #[embassy_executor::main]
/// async fn main(spawner: Spawner) {
///     let mut display = MemLcd::new(...);
///     
///     // Get the EXTCOMIN pin for the background task
///     let extcomin_pin = display.extcomin_pin_mut();
///     
///     // Spawn the EXTCOMIN toggle task
///     spawner.spawn(extcomin_task(extcomin_pin, 60)).unwrap();
/// }
/// ```
#[cfg(feature = "_time-driver")]
pub async fn extcomin_task(extcomin: &'static mut Output<'static>, frequency_hz: u32) {
    use embassy_time::{Duration, Ticker};

    // Toggle twice per cycle (rising and falling edge)
    let toggle_freq = frequency_hz * 2;
    let mut ticker = Ticker::every(Duration::from_hz(toggle_freq as u64));

    loop {
        ticker.next().await;

        // Only toggle if auto-toggle is enabled
        if EXTCOMIN_AUTO_ENABLED.load(Ordering::SeqCst) {
            extcomin.toggle();
        }
    }
}

/// Create an EXTCOMIN toggle task that owns its pin.
///
/// This is an alternative to [`extcomin_task`] that takes ownership of
/// a separately created Output pin, which can be easier to use in some cases.
///
/// # Example
///
/// ```no_run,ignore
/// use embassy_silabs::drivers::display::memlcd::extcomin_task_owned;
/// use embassy_silabs::gpio::{Output, Level};
///
/// let extcomin = Output::new(p.PC_06, Level::Low);
/// spawner.spawn(extcomin_task_owned(extcomin, 60)).unwrap();
/// ```
#[cfg(feature = "_time-driver")]
#[embassy_executor::task]
pub async fn extcomin_task_owned(mut extcomin: Output<'static>, frequency_hz: u32) {
    use embassy_time::{Duration, Ticker};

    // Toggle twice per cycle (rising and falling edge)
    let toggle_freq = frequency_hz * 2;
    let mut ticker = Ticker::every(Duration::from_hz(toggle_freq as u64));

    loop {
        ticker.next().await;

        // Only toggle if auto-toggle is enabled
        if EXTCOMIN_AUTO_ENABLED.load(Ordering::SeqCst) {
            extcomin.toggle();
        }
    }
}
