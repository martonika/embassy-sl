//! Universal Synchronous/Asynchronous Receiver/Transmitter (USART)
//!
//! This driver provides async UART functionality for Silicon Labs EFR32 series MCUs.
//!
//! # Example
//!
//! ```no_run
//! use embassy_silabs::usart::{Usart, Config};
//!
//! let config = Config::default();
//! let mut uart = Usart::new(p.USART0, p.PA_05, p.PA_06, Irqs, config);
//!
//! uart.blocking_write(b"Hello, World!").unwrap();
//! ```
#![macro_use]
#![warn(missing_docs)]

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
pub use pac::usart::vals::{Databits, Ovs, Parity, Stopbits};

/// USART configuration.
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

/// USART error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// Framing error - stop bit not detected.
    Framing,
    /// Parity error - parity check failed.
    Parity,
    /// RX buffer overflow - data lost.
    Overflow,
}

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

/// Interrupt handler for USART RX.
pub struct RxInterruptHandler<T: Instance> {
    _phantom: PhantomData<T>,
}

impl<T: Instance> interrupt::typelevel::Handler<T::RxInterrupt> for RxInterruptHandler<T> {
    unsafe fn on_interrupt() {
        let r = T::regs();
        let s = T::state();

        let if_flags = r.if_().read();

        // Check for RX-related interrupts
        if if_flags.rxdatav() || if_flags.perr() || if_flags.ferr() || if_flags.rxof() {
            // Disable the interrupts we're handling
            r.ien().modify(|w| {
                w.set_rxdatav(false);
                w.set_perr(false);
                w.set_ferr(false);
                w.set_rxof(false);
            });
            s.rx_waker.wake();
        }
    }
}

/// TX interrupt handler for USART.
pub struct TxInterruptHandler<T: Instance> {
    _phantom: PhantomData<T>,
}

impl<T: Instance> interrupt::typelevel::Handler<T::TxInterrupt> for TxInterruptHandler<T> {
    unsafe fn on_interrupt() {
        let r = T::regs();
        let s = T::state();

        let if_flags = r.if_().read();

        // Check for TX-related interrupts
        if if_flags.txbl() || if_flags.txc() {
            r.ien().modify(|w| {
                w.set_txbl(false);
                w.set_txc(false);
            });
            s.tx_waker.wake();
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Calculate and set the clock divider for the desired baud rate.
///
/// Based on the formula from the reference manual:
/// CLKDIV = 256 * (fHFPERCLK/(oversample * br) - 1)
///
/// Simplified for integer math:
/// CLKDIV/8 = 32 * fHFPERCLK / (oversample * br) - 32
fn set_baudrate(r: pac::usart::Usart, ref_freq: u32, baudrate: u32, ovs: Ovs) {
    let oversample: u32 = match ovs {
        Ovs::X16 => 16,
        Ovs::X8 => 8,
        Ovs::X6 => 6,
        Ovs::X4 => 4,
    };

    // Use integer division with rounding
    // CLKDIV/8 = (32 * refFreq) / (baudrate * oversample) - 32
    let clkdiv = if oversample > 0 {
        let div_intermediate = (32 * ref_freq + (baudrate * oversample / 2)) / (baudrate * oversample);
        let clkdiv = div_intermediate.saturating_sub(32) * 8;
        clkdiv & 0x001F_FFF8 // Mask to valid range (20-bit, lower 3 bits reserved)
    } else {
        0
    };

    // Set oversampling in CTRL register
    r.ctrl().modify(|w| w.set_ovs(ovs));

    // Set clock divider
    r.clkdiv().write(|w| w.set_div(clkdiv));
}

/// Disable the USART peripheral.
fn usart_disable(r: pac::usart::Usart) {
    if r.en().read().en() {
        // Disable TX and RX
        r.cmd().write(|w| {
            w.set_txdis(true);
            w.set_rxdis(true);
        });

        // Wait for TX/RX to actually disable
        while r.status().read().txens() || r.status().read().rxens() {}

        // Clear the enable bit
        r.en().write(|w| w.set_en(false));
    }
}

/// Reset the USART to default state.
fn usart_reset(r: pac::usart::Usart) {
    // Enable peripheral first to allow CMD writes
    r.en().write(|w| w.set_en(true));

    // Disable TX, RX, and master mode
    r.cmd().write(|w| {
        w.set_rxdis(true);
        w.set_txdis(true);
        w.set_masterdis(true);
        w.set_rxblockdis(true);
        w.set_txtridis(true);
        w.set_cleartx(true);
        w.set_clearrx(true);
    });

    // Reset configuration registers
    r.ctrl().write(|_| {});
    r.frame().write(|_| {});
    r.trigctrl().write(|_| {});
    r.clkdiv().write(|_| {});
    r.ien().write(|_| {});
    r.timing().write(|_| {});
    r.ctrlx().write(|_| {});

    // Clear all interrupt flags
    r.if_().write(|w| w.0 = 0xFFFF_FFFF);

    // Disable the peripheral
    r.en().write(|w| w.set_en(false));
}

// ============================================================================
// USART Driver
// ============================================================================

/// USART driver supporting both TX and RX.
pub struct Usart<'d, T: Instance> {
    tx: UsartTx<'d, T>,
    rx: UsartRx<'d, T>,
}

impl<'d, T: Instance> Usart<'d, T> {
    /// Create a new USART driver without hardware flow control.
    pub fn new(
        usart: Peri<'d, T>,
        rx: Peri<'d, impl GpioPin>,
        tx: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::RxInterrupt, RxInterruptHandler<T>>
            + interrupt::typelevel::Binding<T::TxInterrupt, TxInterruptHandler<T>>
            + 'd,
        config: Config,
    ) -> Self {
        Self::new_inner(usart, rx.into(), tx.into(), None, None, config)
    }

    /// Create a new USART driver with hardware flow control (CTS/RTS).
    pub fn new_with_rtscts(
        usart: Peri<'d, T>,
        rx: Peri<'d, impl GpioPin>,
        tx: Peri<'d, impl GpioPin>,
        cts: Peri<'d, impl GpioPin>,
        rts: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::RxInterrupt, RxInterruptHandler<T>>
            + interrupt::typelevel::Binding<T::TxInterrupt, TxInterruptHandler<T>>
            + 'd,
        config: Config,
    ) -> Self {
        Self::new_inner(
            usart,
            rx.into(),
            tx.into(),
            Some(cts.into()),
            Some(rts.into()),
            config,
        )
    }

    fn new_inner(
        usart: Peri<'d, T>,
        rx: Peri<'d, AnyPin>,
        tx: Peri<'d, AnyPin>,
        cts: Option<Peri<'d, AnyPin>>,
        rts: Option<Peri<'d, AnyPin>>,
        config: Config,
    ) -> Self {
        let r = T::regs();

        // Reset to known state
        usart_reset(r);

        // Enable peripheral
        r.en().write(|w| w.set_en(true));

        // Configure frame format
        r.frame().write(|w| {
            w.set_databits(config.data_bits);
            w.set_parity(config.parity);
            w.set_stopbits(config.stop_bits);
        });

        // Configure CTRL - majority vote disable
        if config.majority_vote_disable {
            r.ctrl().modify(|w| w.set_mvdis(true));
        }

        // Configure hardware flow control
        if config.hw_flow_control && cts.is_some() {
            r.ctrlx().modify(|w| w.set_ctsen(pac::usart::vals::Ctsen::ENABLE));
        }

        // Set baud rate
        // TODO: Get actual clock frequency from CMU
        let ref_freq = 20_000_000; // Assume 20 MHz for now
        set_baudrate(r, ref_freq, config.baudrate, config.oversampling);

        // Configure GPIO pins
        configure_rx_pin::<T>(&rx, &rts);
        configure_tx_pin::<T>(&tx, &cts);

        // Enable RX and TX
        r.cmd().write(|w| {
            w.set_rxen(true);
            w.set_txen(true);
        });

        // Wait for RX/TX to be enabled
        while !r.status().read().rxens() || !r.status().read().txens() {}

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
            tx: UsartTx {
                _p: unsafe { usart.clone_unchecked() },
            },
            rx: UsartRx { _p: usart },
        }
    }

    /// Split the USART into separate TX and RX halves.
    pub fn split(self) -> (UsartTx<'d, T>, UsartRx<'d, T>) {
        (self.tx, self.rx)
    }

    /// Write bytes to the USART, blocking until complete.
    pub fn blocking_write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.tx.blocking_write(data)
    }

    /// Read bytes from the USART, blocking until the buffer is filled.
    pub fn blocking_read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        self.rx.blocking_read(buffer)
    }

    /// Asynchronously write bytes to the USART.
    pub async fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        self.tx.write(data).await
    }

    /// Asynchronously read bytes from the USART.
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        self.rx.read(buffer).await
    }

    /// Flush the TX buffer, waiting for all data to be transmitted.
    pub fn blocking_flush(&mut self) -> Result<(), Error> {
        self.tx.blocking_flush()
    }
}

// ============================================================================
// TX-only driver
// ============================================================================

/// Transmitter half of the USART driver.
pub struct UsartTx<'d, T: Instance> {
    _p: Peri<'d, T>,
}

impl<'d, T: Instance> UsartTx<'d, T> {
    /// Blocking write to the USART.
    pub fn blocking_write(&mut self, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();

        for &byte in data {
            // Wait for TX buffer to have space (TXBL = TX Buffer Level)
            while !r.status().read().txbl() {}

            r.txdata().write(|w| w.set_txdata(byte));
        }

        Ok(())
    }

    /// Asynchronously write bytes.
    pub async fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        for &byte in data {
            // Wait for TX buffer space using interrupt
            poll_fn(|cx| {
                s.tx_waker.register(cx.waker());

                if r.status().read().txbl() {
                    Poll::Ready(())
                } else {
                    // Enable TXBL interrupt
                    r.ien().modify(|w| w.set_txbl(true));
                    Poll::Pending
                }
            })
            .await;

            r.txdata().write(|w| w.set_txdata(byte));
        }

        Ok(())
    }

    /// Wait for all data to be transmitted.
    pub fn blocking_flush(&mut self) -> Result<(), Error> {
        let r = T::regs();

        // Wait for TX complete (TXC)
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

impl<'d, T: Instance> Drop for UsartTx<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();
        let s = T::state();

        if s.tx_rx_refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            usart_disable(r);
            deconfigure_pins::<T>();
        }
    }
}

// ============================================================================
// RX-only driver
// ============================================================================

/// Receiver half of the USART driver.
pub struct UsartRx<'d, T: Instance> {
    _p: Peri<'d, T>,
}

impl<'d, T: Instance> UsartRx<'d, T> {
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

    /// Blocking read from the USART.
    pub fn blocking_read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();

        for byte in buffer.iter_mut() {
            // Wait for RX data to be available (RXDATAV)
            while !r.status().read().rxdatav() {
                self.check_errors()?;
            }

            *byte = r.rxdata().read().rxdata();
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

                if r.status().read().rxdatav() {
                    Poll::Ready(())
                } else {
                    // Enable RXDATAV interrupt
                    r.ien().modify(|w| {
                        w.set_rxdatav(true);
                        w.set_perr(true);
                        w.set_ferr(true);
                        w.set_rxof(true);
                    });
                    Poll::Pending
                }
            })
            .await;

            self.check_errors()?;
            *byte = r.rxdata().read().rxdata();
        }

        Ok(())
    }
}

impl<'d, T: Instance> Drop for UsartRx<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();
        let s = T::state();

        if s.tx_rx_refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            usart_disable(r);
            deconfigure_pins::<T>();
        }
    }
}

// ============================================================================
// Pin configuration
// ============================================================================

/// Configure RX pin and optional RTS pin for the USART.
fn configure_rx_pin<T: Instance>(rx: &Peri<'_, AnyPin>, rts: &Option<Peri<'_, AnyPin>>) {
    // SAFETY: GPIO peripheral is a singleton, and we're only configuring routes
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    // Get port (0=A, 1=B, 2=C, 3=D) and pin number
    let rx_port = rx.pin_port() / 16;
    let rx_pin = rx.pin_port() % 16;

    // Configure RX pin as input
    rx.mode_w(pac::gpio::vals::PortMode::INPUT);

    // Configure GPIO routing based on USART instance
    // Currently only USART0 exists on xG24
    if T::index() == 0 {
        // Set RX route: port and pin
        gpio.usart0_rxroute().write(|w| {
            w.set_port(rx_port);
            w.set_pin(rx_pin);
        });
        // Enable RX pin in route enable register
        gpio.usart0_routeen().modify(|w| w.set_rxpen(true));

        // Configure RTS if provided
        if let Some(rts_pin) = rts {
            let rts_port = rts_pin.pin_port() / 16;
            let rts_pin_num = rts_pin.pin_port() % 16;

            // RTS is an output
            rts_pin.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
            rts_pin.set_high();

            gpio.usart0_rtsroute().write(|w| {
                w.set_port(rts_port);
                w.set_pin(rts_pin_num);
            });
            gpio.usart0_routeen().modify(|w| w.set_rtspen(true));
        }
    }
}

/// Configure TX pin and optional CTS pin for the USART.
fn configure_tx_pin<T: Instance>(tx: &Peri<'_, AnyPin>, cts: &Option<Peri<'_, AnyPin>>) {
    // SAFETY: GPIO peripheral is a singleton, and we're only configuring routes
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    // Get port and pin number
    let tx_port = tx.pin_port() / 16;
    let tx_pin = tx.pin_port() % 16;

    // Configure TX pin as push-pull output, initially high (idle)
    tx.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
    tx.set_high();

    // Configure GPIO routing based on USART instance
    if T::index() == 0 {
        gpio.usart0_txroute().write(|w| {
            w.set_port(tx_port);
            w.set_pin(tx_pin);
        });
        gpio.usart0_routeen().modify(|w| w.set_txpen(true));

        // Configure CTS if provided
        if let Some(cts_pin) = cts {
            let cts_port = cts_pin.pin_port() / 16;
            let cts_pin_num = cts_pin.pin_port() % 16;

            // CTS is an input
            cts_pin.mode_w(pac::gpio::vals::PortMode::INPUT);

            gpio.usart0_ctsroute().write(|w| {
                w.set_port(cts_port);
                w.set_pin(cts_pin_num);
            });
            // Note: CTS doesn't need a pin enable - it's always enabled when CTSEN is set
        }
    }
}

/// Deconfigure pins when the USART is dropped.
fn deconfigure_pins<T: Instance>() {
    // SAFETY: GPIO peripheral is a singleton, and we're only configuring routes
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    if T::index() == 0 {
        // Disable all USART0 routes
        gpio.usart0_routeen().write(|w| {
            w.set_rxpen(false);
            w.set_txpen(false);
            w.set_rtspen(false);
        });
    }
}

// ============================================================================
// Instance trait and implementations
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::usart::Usart;
    fn state() -> &'static State;
    /// Returns the USART instance index (0 for USART0, etc.)
    fn index() -> u8;
}

/// USART peripheral instance trait.
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
            Error::Overflow => embedded_io::ErrorKind::OutOfMemory,
        }
    }
}

impl<'d, T: Instance> embedded_io::ErrorType for Usart<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_io::ErrorType for UsartTx<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_io::ErrorType for UsartRx<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_io::Write for Usart<'d, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.blocking_write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.blocking_flush()
    }
}

impl<'d, T: Instance> embedded_io::Write for UsartTx<'d, T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.blocking_write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.blocking_flush()
    }
}

impl<'d, T: Instance> embedded_io::Read for UsartRx<'d, T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.blocking_read(buf)?;
        Ok(buf.len())
    }
}

impl<'d, T: Instance> embedded_io_async::Write for Usart<'d, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Usart::write(self, buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.tx.flush().await
    }
}

impl<'d, T: Instance> embedded_io_async::Write for UsartTx<'d, T> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        UsartTx::write(self, buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        UsartTx::flush(self).await
    }
}

impl<'d, T: Instance> embedded_io_async::Read for UsartRx<'d, T> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        UsartRx::read(self, buf).await?;
        Ok(buf.len())
    }
}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for USART peripherals.
#[macro_export]
macro_rules! impl_usart {
    ($type:ident, $pac_type:ident, $rx_irq:ident, $tx_irq:ident, $index:expr) => {
        impl $crate::usart::usart::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::usart::Usart {
                $crate::pac::$pac_type
            }
            fn state() -> &'static $crate::usart::usart::State {
                static STATE: $crate::usart::usart::State = $crate::usart::usart::State::new();
                &STATE
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::usart::usart::Instance for $crate::peripherals::$type {
            type RxInterrupt = $crate::interrupt::typelevel::$rx_irq;
            type TxInterrupt = $crate::interrupt::typelevel::$tx_irq;
        }
    };
}
