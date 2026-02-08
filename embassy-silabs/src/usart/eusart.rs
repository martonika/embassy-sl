//! Enhanced Universal Synchronous/Asynchronous Receiver/Transmitter (EUSART)
//!
//! This driver provides async UART functionality for Silicon Labs EFR32 series MCUs.
//!
//! # Example
//!
//! ```no_run
//! use embassy_silabs::eusart::{Eusart, Config};
//!
//! let config = Config::default();
//! let mut uart = Eusart::new(p.EUSART0, p.PA_05, p.PA_06, Irqs, config);
//!
//! uart.blocking_write(b"Hello, World!").unwrap();
//! ```
#![macro_use]
#![warn(missing_docs)]

use core::fmt;
use core::future::poll_fn;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU8, Ordering};
use core::task::Poll;

use embassy_hal_internal::{Peri, PeripheralType};
use embassy_sync::waitqueue::AtomicWaker;

use crate::chip::pac;
use crate::gpio::{AnyPin, Pin as GpioPin, SealedPin as GpioSealedPin};
use crate::interrupt;
use crate::interrupt::typelevel::Interrupt;

// GPIO peripheral access
#[cfg(feature = "_ns")]
use pac::GPIO_NS as GPIO;
#[cfg(not(feature = "_ns"))]
use pac::GPIO_S as GPIO;

// Re-export PAC types for configuration
pub use pac::eusart::regs::{Cfg0, Cfg1, Framecfg};
pub use pac::eusart::vals::{Databits, Ovs, Parity, Stopbits};

/// EUSART configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// Baud rate in bits per second.
    pub baudrate: u32,
    /// Number of data bits.
    pub data_bits: Databits,
    /// Parity mode.
    pub parity: Parity,
    /// Number of stop bits.
    pub stop_bits: Stopbits,
    /// Oversampling rate. Use `Ovs::X16` for standard UART.
    pub oversampling: Ovs,
    /// Enable hardware flow control (CTS/RTS).
    pub hw_flow_control: bool,
    /// Disable majority voting (for noisy environments, keep false).
    pub majority_vote_disable: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baudrate: 115_200,
            data_bits: Databits::EIGHT,
            parity: Parity::NONE,
            stop_bits: Stopbits::ONE,
            oversampling: Ovs::X16,
            hw_flow_control: false,
            majority_vote_disable: false,
        }
    }
}

/// EUSART error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// Framing error - stop bit not detected.
    Framing,
    /// Parity error - parity check failed.
    Parity,
    /// RX FIFO overflow - data lost.
    Overflow,
    /// RX FIFO underflow - read when empty.
    Underflow,
    /// TX FIFO overflow.
    TxOverflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Framing => write!(f, "Framing error"),
            Error::Parity => write!(f, "Parity error"),
            Error::Overflow => write!(f, "RX FIFO overflow"),
            Error::Underflow => write!(f, "RX FIFO underflow"),
            Error::TxOverflow => write!(f, "TX FIFO overflow"),
        }
    }
}

impl core::error::Error for Error {}

/// Internal state shared between driver instances.
pub(crate) struct State {
    pub(crate) rx_waker: AtomicWaker,
    pub(crate) tx_waker: AtomicWaker,
    pub(crate) tx_rx_refcount: AtomicU8,
}

impl State {
    pub(crate) const fn new() -> Self {
        Self {
            rx_waker: AtomicWaker::new(),
            tx_waker: AtomicWaker::new(),
            tx_rx_refcount: AtomicU8::new(0),
        }
    }
}

/// Interrupt handler for EUSART.
pub struct InterruptHandler<T: Instance> {
    _phantom: PhantomData<T>,
}

impl<T: Instance> interrupt::typelevel::Handler<T::RxInterrupt> for InterruptHandler<T> {
    unsafe fn on_interrupt() {
        let r = T::regs();
        let s = T::state();

        let if_flags = r.if_().read();

        // Check for RX-related interrupts
        if if_flags.rxfl() || if_flags.perr() || if_flags.ferr() || if_flags.rxof() {
            // Disable the interrupts we're handling
            r.ien().modify(|w| {
                w.set_rxfl(false);
                w.set_perr(false);
                w.set_ferr(false);
                w.set_rxof(false);
            });
            s.rx_waker.wake();
        }
    }
}

/// TX interrupt handler (separate from RX on Silicon Labs).
pub struct TxInterruptHandler<T: Instance> {
    _phantom: PhantomData<T>,
}

impl<T: Instance> interrupt::typelevel::Handler<T::TxInterrupt> for TxInterruptHandler<T> {
    unsafe fn on_interrupt() {
        let r = T::regs();
        let s = T::state();

        let if_flags = r.if_().read();

        // Check for TX-related interrupts
        if if_flags.txfl() || if_flags.txc() {
            r.ien().modify(|w| {
                w.set_txfl(false);
                w.set_txc(false);
            });
            s.tx_waker.wake();
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

/// Calculate and set the clock divider for the desired baud rate.
///
/// Based on the formula from the reference manual:
/// CLKDIV = 256 * (fEUSART / (oversample * baudrate) - 1)
///
/// Or simplified for integer math:
/// CLKDIV/8 = 32 * fEUSART / (oversample * baudrate) - 32
fn set_baudrate(r: pac::eusart::Eusart, ref_freq: u32, baudrate: u32, ovs: Ovs) {
    let oversample: u32 = match ovs {
        Ovs::X16 => 16,
        Ovs::X8 => 8,
        Ovs::X6 => 6,
        Ovs::X4 => 4,
        Ovs::DISABLE => 1, // No oversampling (LF mode)
        _ => 16,
    };

    // Use integer division with rounding
    // clkdiv = (32 * refFreq) / (baudrate * oversample) - 32, then *8
    let clkdiv = if oversample > 0 {
        let div_intermediate = (32 * ref_freq) / (baudrate * oversample);
        let clkdiv = (div_intermediate.saturating_sub(32)) * 8;
        clkdiv & 0x007F_FFF8 // Mask to valid range (20-bit, lower 3 bits reserved)
    } else {
        0
    };

    // Wait for any pending sync
    eusart_sync(r, 0x01); // SYNCBUSY_DIV

    r.clkdiv().write(|w| w.set_div(clkdiv));

    // Wait for sync to complete
    eusart_sync(r, 0x01);
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

// ============================================================================
// EUSART Driver
// ============================================================================

/// EUSART driver supporting both TX and RX.
pub struct Eusart<'d, T: Instance> {
    tx: EusartTx<'d, T>,
    rx: EusartRx<'d, T>,
}

impl<'d, T: Instance> Eusart<'d, T> {
    /// Create a new EUSART driver without hardware flow control.
    pub fn new(
        eusart: Peri<'d, T>,
        rx: Peri<'d, impl GpioPin>,
        tx: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::RxInterrupt, InterruptHandler<T>>
        + interrupt::typelevel::Binding<T::TxInterrupt, TxInterruptHandler<T>>
        + 'd,
        config: Config,
    ) -> Self {
        Self::new_inner(eusart, rx.into(), tx.into(), None, None, config)
    }

    /// Create a new EUSART driver with hardware flow control (CTS/RTS).
    pub fn new_with_rtscts(
        eusart: Peri<'d, T>,
        rx: Peri<'d, impl GpioPin>,
        tx: Peri<'d, impl GpioPin>,
        cts: Peri<'d, impl GpioPin>,
        rts: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::RxInterrupt, InterruptHandler<T>>
        + interrupt::typelevel::Binding<T::TxInterrupt, TxInterruptHandler<T>>
        + 'd,
        config: Config,
    ) -> Self {
        Self::new_inner(
            eusart,
            rx.into(),
            tx.into(),
            Some(cts.into()),
            Some(rts.into()),
            config,
        )
    }

    fn new_inner(
        eusart: Peri<'d, T>,
        rx: Peri<'d, AnyPin>,
        tx: Peri<'d, AnyPin>,
        cts: Option<Peri<'d, AnyPin>>,
        rts: Option<Peri<'d, AnyPin>>,
        config: Config,
    ) -> Self {
        let r = T::regs();

        // Reset to known state
        eusart_reset(r);

        // Configure frame format
        r.framecfg().write(|w| {
            w.set_databits(config.data_bits);
            w.set_parity(config.parity);
            w.set_stopbits(config.stop_bits);
        });

        // Configure CFG0
        r.cfg0().write(|w| {
            w.set_ovs(config.oversampling);
            w.set_mvdis(config.majority_vote_disable);
        });

        // Configure CFG1 for flow control
        if config.hw_flow_control && cts.is_some() {
            r.cfg1().modify(|w| {
                w.set_ctsen(pac::eusart::vals::Ctsen::ENABLE);
            });
        }

        // Enable the peripheral
        r.en().write(|w| w.set_en(true));

        // Set baud rate (must be done after enable)
        // TODO: Get actual clock frequency from CMU
        let ref_freq = 20_000_000; // Assume 20 MHz for now
        set_baudrate(r, ref_freq, config.baudrate, config.oversampling);

        // Configure GPIO pins
        configure_rx_pin::<T>(&rx, &rts);
        configure_tx_pin::<T>(&tx, &cts);

        // Enable RX and TX
        eusart_sync(r, 0xFF);
        r.cmd().write(|w| {
            w.set_rxen(true);
            w.set_txen(true);
        });
        eusart_sync(r, 0x18); // Wait for RXEN/TXEN

        // Wait for RX/TX to be enabled
        while !r.status().read().rxens() || !r.status().read().txens() {}

        // Wait for idle
        while !r.status().read().rxidle() || !r.status().read().txidle() {}

        // Enable interrupts
        T::RxInterrupt::unpend();
        T::TxInterrupt::unpend();
        unsafe {
            T::RxInterrupt::enable();
            T::TxInterrupt::enable();
        }

        let s = T::state();
        s.tx_rx_refcount.store(2, Ordering::Relaxed);

        Self {
            tx: EusartTx {
                _p: unsafe { eusart.clone_unchecked() },
            },
            rx: EusartRx { _p: eusart },
        }
    }

    /// Split the EUSART into separate TX and RX halves.
    pub fn split(self) -> (EusartTx<'d, T>, EusartRx<'d, T>) {
        (self.tx, self.rx)
    }

    /// Write bytes to the EUSART, blocking until complete.
    pub fn blocking_write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.tx.blocking_write(data)
    }

    /// Read bytes from the EUSART, blocking until the buffer is filled.
    pub fn blocking_read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        self.rx.blocking_read(buffer)
    }

    /// Asynchronously write bytes to the EUSART.
    pub async fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.tx.write(data).await
    }

    /// Asynchronously read bytes from the EUSART.
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        self.rx.read(buffer).await
    }

    /// Flush the TX FIFO, waiting for all data to be transmitted.
    pub fn blocking_flush(&mut self) -> Result<(), Error> {
        self.tx.blocking_flush()
    }
}

// ============================================================================
// TX-only driver
// ============================================================================

/// Transmitter half of the EUSART driver.
pub struct EusartTx<'d, T: Instance> {
    _p: Peri<'d, T>,
}

impl<'d, T: Instance> EusartTx<'d, T> {
    /// Blocking write to the EUSART.
    pub fn blocking_write(&mut self, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();

        for &byte in data {
            // Wait for TX FIFO to have space
            while !r.status().read().txfl() {}

            r.txdata().write(|w| w.0 = byte as u32);
        }

        Ok(())
    }

    /// Asynchronously write bytes.
    pub async fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        for &byte in data {
            // Wait for TX FIFO space using interrupt
            poll_fn(|cx| {
                s.tx_waker.register(cx.waker());

                if r.status().read().txfl() {
                    Poll::Ready(())
                } else {
                    // Enable TXFL interrupt
                    r.ien().modify(|w| w.set_txfl(true));
                    Poll::Pending
                }
            })
            .await;

            r.txdata().write(|w| w.0 = byte as u32);
        }

        Ok(())
    }

    /// Wait for all data to be transmitted.
    pub fn blocking_flush(&mut self) -> Result<(), Error> {
        let r = T::regs();

        // Wait for TX complete
        while !r.status().read().txc() {}

        Ok(())
    }

    /// Asynchronously wait for all data to be transmitted.
    pub async fn flush(&mut self) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        poll_fn(|cx| {
            s.tx_waker.register(cx.waker());

            if r.status().read().txc() {
                Poll::Ready(())
            } else {
                r.ien().modify(|w| w.set_txc(true));
                Poll::Pending
            }
        })
        .await;

        Ok(())
    }
}

impl<'d, T: Instance> Drop for EusartTx<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();
        let s = T::state();

        if s.tx_rx_refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            eusart_disable(r);
            deconfigure_pins::<T>();
        }
    }
}

// ============================================================================
// RX-only driver
// ============================================================================

/// Receiver half of the EUSART driver.
pub struct EusartRx<'d, T: Instance> {
    _p: Peri<'d, T>,
}

impl<'d, T: Instance> EusartRx<'d, T> {
    /// Check and clear error flags.
    fn check_errors(&self) -> Result<(), Error> {
        let r = T::regs();
        let if_flags = r.if_().read();

        if if_flags.perr() {
            r.if_().write(|w| w.set_perr(true)); // Clear flag
            return Err(Error::Parity);
        }
        if if_flags.ferr() {
            r.if_().write(|w| w.set_ferr(true));
            return Err(Error::Framing);
        }
        if if_flags.rxof() {
            r.if_().write(|w| w.set_rxof(true));
            return Err(Error::Overflow);
        }

        Ok(())
    }

    /// Blocking read from the EUSART.
    pub fn blocking_read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();

        for byte in buffer.iter_mut() {
            // Wait for RX FIFO to have data
            while !r.status().read().rxfl() {
                self.check_errors()?;
            }

            *byte = r.rxdata().read().rxdata() as u8;
        }

        Ok(())
    }

    /// Asynchronously read bytes.
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        for byte in buffer.iter_mut() {
            poll_fn(|cx| {
                s.rx_waker.register(cx.waker());

                // Check for errors first
                let if_flags = r.if_().read();
                if if_flags.perr() || if_flags.ferr() || if_flags.rxof() {
                    return Poll::Ready(());
                }

                if r.status().read().rxfl() {
                    Poll::Ready(())
                } else {
                    // Enable RXFL interrupt
                    r.ien().modify(|w| {
                        w.set_rxfl(true);
                        w.set_perr(true);
                        w.set_ferr(true);
                        w.set_rxof(true);
                    });
                    Poll::Pending
                }
            })
            .await;

            self.check_errors()?;
            *byte = r.rxdata().read().rxdata() as u8;
        }

        Ok(())
    }
}

impl<'d, T: Instance> Drop for EusartRx<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();
        let s = T::state();

        if s.tx_rx_refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            eusart_disable(r);
            deconfigure_pins::<T>();
        }
    }
}

// ============================================================================
// Pin configuration
// ============================================================================

/// Configure RX pin and optional RTS pin for the EUSART.
fn configure_rx_pin<T: Instance>(rx: &Peri<'_, AnyPin>, rts: &Option<Peri<'_, AnyPin>>) {
    // SAFETY: GPIO peripheral is a singleton, and we're only configuring routes
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    // Get port (0=A, 1=B, 2=C, 3=D) and pin number
    let rx_port = rx.pin_port() / 16;
    let rx_pin = rx.pin_port() % 16;

    // Configure RX pin as input
    rx.mode_w(pac::gpio::vals::PortMode::INPUT);

    // Configure GPIO routing based on EUSART instance
    match T::index() {
        0 => {
            // Set RX route: port and pin
            gpio.eusart0_rxroute().write(|w| {
                w.set_port(rx_port);
                w.set_pin(rx_pin);
            });
            // Enable RX pin in route enable register
            gpio.eusart0_routeen().modify(|w| w.set_rxpen(true));

            // Configure RTS if provided
            if let Some(rts_pin) = rts {
                let rts_port = rts_pin.pin_port() / 16;
                let rts_pin_num = rts_pin.pin_port() % 16;

                // RTS is an output
                rts_pin.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
                rts_pin.set_high();

                gpio.eusart0_rtsroute().write(|w| {
                    w.set_port(rts_port);
                    w.set_pin(rts_pin_num);
                });
                gpio.eusart0_routeen().modify(|w| w.set_rtspen(true));
            }
        }
        1 => {
            gpio.eusart1_rxroute().write(|w| {
                w.set_port(rx_port);
                w.set_pin(rx_pin);
            });
            gpio.eusart1_routeen().modify(|w| w.set_rxpen(true));

            if let Some(rts_pin) = rts {
                let rts_port = rts_pin.pin_port() / 16;
                let rts_pin_num = rts_pin.pin_port() % 16;

                rts_pin.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
                rts_pin.set_high();

                gpio.eusart1_rtsroute().write(|w| {
                    w.set_port(rts_port);
                    w.set_pin(rts_pin_num);
                });
                gpio.eusart1_routeen().modify(|w| w.set_rtspen(true));
            }
        }
        _ => {}
    }
}

/// Configure TX pin and optional CTS pin for the EUSART.
fn configure_tx_pin<T: Instance>(tx: &Peri<'_, AnyPin>, cts: &Option<Peri<'_, AnyPin>>) {
    // SAFETY: GPIO peripheral is a singleton, and we're only configuring routes
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    // Get port and pin number
    let tx_port = tx.pin_port() / 16;
    let tx_pin = tx.pin_port() % 16;

    // Configure TX pin as push-pull output, initially high (idle)
    tx.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
    tx.set_high();

    // Configure GPIO routing based on EUSART instance
    match T::index() {
        0 => {
            gpio.eusart0_txroute().write(|w| {
                w.set_port(tx_port);
                w.set_pin(tx_pin);
            });
            gpio.eusart0_routeen().modify(|w| w.set_txpen(true));

            // Configure CTS if provided
            if let Some(cts_pin) = cts {
                let cts_port = cts_pin.pin_port() / 16;
                let cts_pin_num = cts_pin.pin_port() % 16;

                // CTS is an input
                cts_pin.mode_w(pac::gpio::vals::PortMode::INPUT);

                gpio.eusart0_ctsroute().write(|w| {
                    w.set_port(cts_port);
                    w.set_pin(cts_pin_num);
                });
                // Note: CTS doesn't need a pin enable - it's always enabled when CTSEN is set in CFG1
            }
        }
        1 => {
            gpio.eusart1_txroute().write(|w| {
                w.set_port(tx_port);
                w.set_pin(tx_pin);
            });
            gpio.eusart1_routeen().modify(|w| w.set_txpen(true));

            if let Some(cts_pin) = cts {
                let cts_port = cts_pin.pin_port() / 16;
                let cts_pin_num = cts_pin.pin_port() % 16;

                cts_pin.mode_w(pac::gpio::vals::PortMode::INPUT);

                gpio.eusart1_ctsroute().write(|w| {
                    w.set_port(cts_port);
                    w.set_pin(cts_pin_num);
                });
            }
        }
        _ => {}
    }
}

/// Deconfigure pins when the EUSART is dropped.
fn deconfigure_pins<T: Instance>() {
    // SAFETY: GPIO peripheral is a singleton, and we're only configuring routes
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    match T::index() {
        0 => {
            // Disable all EUSART0 routes
            gpio.eusart0_routeen().write(|w| {
                w.set_rxpen(false);
                w.set_txpen(false);
                w.set_rtspen(false);
            });
        }
        1 => {
            gpio.eusart1_routeen().write(|w| {
                w.set_rxpen(false);
                w.set_txpen(false);
                w.set_rtspen(false);
            });
        }
        _ => {}
    }
}

// ============================================================================
// Instance trait and implementations
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::eusart::Eusart;
    fn state() -> &'static State;
    /// Returns the EUSART instance index (0 for EUSART0, 1 for EUSART1, etc.)
    fn index() -> u8;
}

/// EUSART peripheral instance trait.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {
    /// RX interrupt for this peripheral.
    type RxInterrupt: interrupt::typelevel::Interrupt;
    /// TX interrupt for this peripheral.
    type TxInterrupt: interrupt::typelevel::Interrupt;
}

// ============================================================================
// embedded-io trait implementations
// ============================================================================

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Error::Framing | Error::Parity => embedded_io::ErrorKind::InvalidData,
            Error::Overflow | Error::Underflow | Error::TxOverflow => {
                embedded_io::ErrorKind::OutOfMemory
            }
        }
    }
}

impl<'d, T: Instance> embedded_io::ErrorType for Eusart<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_io::ErrorType for EusartTx<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_io::ErrorType for EusartRx<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_io::Write for Eusart<'d, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.blocking_write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.blocking_flush()
    }
}

impl<'d, T: Instance> embedded_io::Write for EusartTx<'d, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.blocking_write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.blocking_flush()
    }
}

impl<'d, T: Instance> embedded_io::Read for EusartRx<'d, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.blocking_read(buf)?;
        Ok(buf.len())
    }
}

impl<'d, T: Instance> embedded_io_async::Write for Eusart<'d, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Eusart::write(self, buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.tx.flush().await
    }
}

impl<'d, T: Instance> embedded_io_async::Write for EusartTx<'d, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        EusartTx::write(self, buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        EusartTx::flush(self).await
    }
}

impl<'d, T: Instance> embedded_io_async::Read for EusartRx<'d, T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        EusartRx::read(self, buf).await?;
        Ok(buf.len())
    }
}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for EUSART peripherals.
#[macro_export]
macro_rules! impl_eusart {
    ($type:ident, $pac_type:ident, $rx_irq:ident, $tx_irq:ident, $index:expr) => {
        impl $crate::usart::eusart::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::eusart::Eusart {
                $crate::pac::$pac_type
            }
            fn state() -> &'static $crate::usart::eusart::State {
                static STATE: $crate::usart::eusart::State = $crate::usart::eusart::State::new();
                &STATE
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::usart::eusart::Instance for $crate::peripherals::$type {
            type RxInterrupt = $crate::interrupt::typelevel::$rx_irq;
            type TxInterrupt = $crate::interrupt::typelevel::$tx_irq;
        }
    };
}
