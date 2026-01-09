//! Inter-Integrated Circuit (I2C) driver.
//!
//! This driver provides async I2C master functionality for Silicon Labs EFR32 series MCUs.
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::i2c::{I2c, Config};
//!
//! let config = Config::default();
//! let mut i2c = I2c::new(p.I2C0, p.PA_05, p.PA_06, Irqs, config);
//!
//! // Write to device at address 0x50
//! i2c.blocking_write(0x50, &[0x00, 0x01]).unwrap();
//!
//! // Read from device
//! let mut buf = [0u8; 4];
//! i2c.blocking_read(0x50, &mut buf).unwrap();
//! ```
#![warn(missing_docs)]

use core::future::poll_fn;
use core::marker::PhantomData;
use core::task::Poll;

use embassy_hal_internal::{Peri, PeripheralType};
use embassy_sync::waitqueue::AtomicWaker;

use crate::chip::pac;
use crate::gpio::{AnyPin, Pin as GpioPin, SealedPin as GpioSealedPin};
use crate::interrupt;
use crate::interrupt::typelevel::Interrupt;

// GPIO peripheral access
use pac::GPIO;

/// I2C clock speed modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Speed {
    /// Standard mode (100 kHz)
    #[default]
    Standard,
    /// Fast mode (400 kHz)
    Fast,
    /// Fast mode plus (1 MHz)
    FastPlus,
}

impl Speed {
    /// Get the frequency in Hz for this speed mode.
    pub fn frequency(&self) -> u32 {
        match self {
            Speed::Standard => 100_000,
            Speed::Fast => 400_000,
            Speed::FastPlus => 1_000_000,
        }
    }

    /// Get the clock low/high ratio for this speed mode.
    fn clhr(&self) -> pac::i2c0::vals::Clhr {
        match self {
            Speed::Standard => pac::i2c0::vals::Clhr::STANDARD,
            Speed::Fast => pac::i2c0::vals::Clhr::ASYMMETRIC,
            Speed::FastPlus => pac::i2c0::vals::Clhr::FAST,
        }
    }

    /// Get the Nlow + Nhigh sum for clock division calculation.
    fn n_sum(&self) -> u32 {
        match self {
            Speed::Standard => 8,  // 4 + 4
            Speed::Fast => 9,      // 6 + 3
            Speed::FastPlus => 17, // 11 + 6
        }
    }
}

/// I2C configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// I2C bus speed.
    pub speed: Speed,
    /// Reference clock frequency in Hz. Set to 0 to use the default (20 MHz).
    pub ref_freq: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            speed: Speed::Standard,
            ref_freq: 0, // Use default
        }
    }
}

/// I2C error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// Bus error - misplaced START/STOP condition.
    Bus,
    /// Arbitration lost - another master took control.
    ArbitrationLoss,
    /// NACK received - device did not acknowledge.
    Nack,
    /// Timeout waiting for operation to complete.
    Timeout,
}

/// Internal state shared between driver instances.
pub struct State {
    waker: AtomicWaker,
}

impl State {
    /// Create a new state instance.
    pub const fn new() -> Self {
        Self {
            waker: AtomicWaker::new(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Interrupt handler for I2C.
pub struct InterruptHandler<T: Instance> {
    _phantom: PhantomData<T>,
}

impl<T: Instance> interrupt::typelevel::Handler<T::Interrupt> for InterruptHandler<T> {
    unsafe fn on_interrupt() {
        let r = T::regs();
        let s = T::state();

        // Disable all interrupts to prevent repeated firing
        r.ien().write(|w| w.0 = 0);

        // Wake the async task
        s.waker.wake();
    }
}

// ============================================================================
// I2C Driver
// ============================================================================

/// I2C driver.
pub struct I2c<'d, T: Instance> {
    _p: Peri<'d, T>,
}

impl<'d, T: Instance> I2c<'d, T> {
    /// Create a new I2C driver.
    pub fn new(
        i2c: Peri<'d, T>,
        scl: Peri<'d, impl GpioPin>,
        sda: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::Interrupt, InterruptHandler<T>> + 'd,
        config: Config,
    ) -> Self {
        Self::new_inner(i2c, scl.into(), sda.into(), config)
    }

    fn new_inner(
        i2c: Peri<'d, T>,
        scl: Peri<'d, AnyPin>,
        sda: Peri<'d, AnyPin>,
        config: Config,
    ) -> Self {
        let r = T::regs();

        // Enable I2C clock
        pac::CMU.clken0().modify(|w| {
            match T::index() {
                0 => w.set_i2c0(true),
                1 => w.set_i2c1(true),
                _ => {}
            }
        });

        // Reset the I2C peripheral
        i2c_reset(r);

        // Configure bus frequency
        let ref_freq = if config.ref_freq == 0 {
            20_000_000 // Default to 20 MHz
        } else {
            config.ref_freq
        };
        set_bus_freq(r, ref_freq, config.speed);

        // Configure GPIO pins for I2C (open-drain with pull-up)
        configure_pins::<T>(&scl, &sda);

        // Enable the I2C peripheral
        r.en().write(|w| w.set_en(pac::i2c0::vals::En::ENABLE));

        // Enable interrupt in NVIC
        T::Interrupt::unpend();
        unsafe { T::Interrupt::enable() };

        Self { _p: i2c }
    }

    /// Write data to the I2C bus (blocking).
    pub fn blocking_write(&mut self, address: u8, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();

        // Abort any pending operation if bus is busy
        if r.state().read().busy() {
            r.cmd().write(|w| w.set_abort(true));
        }

        // Clear pending interrupts and TX buffer
        r.cmd().write(|w| {
            w.set_clearpc(true);
            w.set_cleartx(true);
        });
        flush_rx(r);
        clear_interrupts(r);

        // Send START + address (write mode - LSB = 0)
        r.txdata().write(|w| w.set_txdata(address << 1));
        r.cmd().write(|w| w.set_start(true));

        // Wait for ACK on address
        self.blocking_wait_ack()?;

        // Send data bytes
        for &byte in data {
            r.txdata().write(|w| w.set_txdata(byte));
            self.blocking_wait_ack()?;
        }

        // Send STOP
        r.cmd().write(|w| w.set_stop(true));
        self.blocking_wait_stop()?;

        Ok(())
    }

    /// Read data from the I2C bus (blocking).
    pub fn blocking_read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();

        if buffer.is_empty() {
            return Ok(());
        }

        // Abort any pending operation if bus is busy
        if r.state().read().busy() {
            r.cmd().write(|w| w.set_abort(true));
        }

        // Clear pending interrupts and buffers
        r.cmd().write(|w| {
            w.set_clearpc(true);
            w.set_cleartx(true);
        });
        flush_rx(r);
        clear_interrupts(r);

        // Send START + address (read mode - LSB = 1)
        r.txdata().write(|w| w.set_txdata((address << 1) | 1));
        r.cmd().write(|w| w.set_start(true));

        // Wait for ACK on address
        self.blocking_wait_ack()?;

        // Pre-arm NACK for last byte
        let len = buffer.len();
        if len == 1 {
            r.cmd().write(|w| w.set_nack(true));
        }

        // Read data bytes
        for (i, byte) in buffer.iter_mut().enumerate() {
            // Wait for data
            self.blocking_wait_rxdata()?;
            *byte = r.rxdata().read().rxdata();

            // Send ACK/NACK for next byte
            if i < len - 1 {
                r.cmd().write(|w| w.set_ack(true));
                // Pre-arm NACK for the last byte
                if i == len - 2 {
                    r.cmd().write(|w| w.set_nack(true));
                }
            }
        }

        // Send STOP
        r.cmd().write(|w| w.set_stop(true));
        self.blocking_wait_stop()?;

        Ok(())
    }

    /// Write then read data from the I2C bus (blocking).
    pub fn blocking_write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Error> {
        let r = T::regs();

        // Abort any pending operation if bus is busy
        if r.state().read().busy() {
            r.cmd().write(|w| w.set_abort(true));
        }

        // Clear pending interrupts and TX buffer
        r.cmd().write(|w| {
            w.set_clearpc(true);
            w.set_cleartx(true);
        });
        flush_rx(r);
        clear_interrupts(r);

        // Send START + address (write mode)
        r.txdata().write(|w| w.set_txdata(address << 1));
        r.cmd().write(|w| w.set_start(true));

        // Wait for ACK on address
        self.blocking_wait_ack()?;

        // Send write data
        for &byte in write {
            r.txdata().write(|w| w.set_txdata(byte));
            self.blocking_wait_ack()?;
        }

        // Send repeated START + address (read mode)
        r.cmd().write(|w| w.set_start(true));
        r.txdata().write(|w| w.set_txdata((address << 1) | 1));

        // Wait for ACK on address
        self.blocking_wait_ack()?;

        // Pre-arm NACK for last byte
        let read_len = read.len();
        if read_len == 1 {
            r.cmd().write(|w| w.set_nack(true));
        }

        // Read data bytes
        for (i, byte) in read.iter_mut().enumerate() {
            self.blocking_wait_rxdata()?;
            *byte = r.rxdata().read().rxdata();

            if i < read_len - 1 {
                r.cmd().write(|w| w.set_ack(true));
                if i == read_len - 2 {
                    r.cmd().write(|w| w.set_nack(true));
                }
            }
        }

        // Send STOP
        r.cmd().write(|w| w.set_stop(true));
        self.blocking_wait_stop()?;

        Ok(())
    }

    /// Async write data to the I2C bus.
    pub async fn write(&mut self, address: u8, data: &[u8]) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        // Abort any pending operation if bus is busy
        if r.state().read().busy() {
            r.cmd().write(|w| w.set_abort(true));
        }

        // Clear pending interrupts and TX buffer
        r.cmd().write(|w| {
            w.set_clearpc(true);
            w.set_cleartx(true);
        });
        flush_rx(r);
        clear_interrupts(r);

        // Send START + address (write mode)
        r.txdata().write(|w| w.set_txdata(address << 1));
        r.cmd().write(|w| w.set_start(true));

        // Wait for ACK on address
        self.async_wait_ack(r, s).await?;

        // Send data bytes
        for &byte in data {
            r.txdata().write(|w| w.set_txdata(byte));
            self.async_wait_ack(r, s).await?;
        }

        // Send STOP
        r.cmd().write(|w| w.set_stop(true));
        self.async_wait_stop(r, s).await?;

        Ok(())
    }

    /// Async read data from the I2C bus.
    pub async fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        if buffer.is_empty() {
            return Ok(());
        }

        // Abort any pending operation if bus is busy
        if r.state().read().busy() {
            r.cmd().write(|w| w.set_abort(true));
        }

        // Clear pending interrupts and buffers
        r.cmd().write(|w| {
            w.set_clearpc(true);
            w.set_cleartx(true);
        });
        flush_rx(r);
        clear_interrupts(r);

        // Send START + address (read mode)
        r.txdata().write(|w| w.set_txdata((address << 1) | 1));
        r.cmd().write(|w| w.set_start(true));

        // Wait for ACK on address
        self.async_wait_ack(r, s).await?;

        // Pre-arm NACK for last byte
        let len = buffer.len();
        if len == 1 {
            r.cmd().write(|w| w.set_nack(true));
        }

        // Read data bytes
        for (i, byte) in buffer.iter_mut().enumerate() {
            self.async_wait_rxdata(r, s).await?;
            *byte = r.rxdata().read().rxdata();

            if i < len - 1 {
                r.cmd().write(|w| w.set_ack(true));
                if i == len - 2 {
                    r.cmd().write(|w| w.set_nack(true));
                }
            }
        }

        // Send STOP
        r.cmd().write(|w| w.set_stop(true));
        self.async_wait_stop(r, s).await?;

        Ok(())
    }

    /// Async write then read data from the I2C bus.
    pub async fn write_read(
        &mut self,
        address: u8,
        write: &[u8],
        read: &mut [u8],
    ) -> Result<(), Error> {
        let r = T::regs();
        let s = T::state();

        // Disable all interrupts first
        r.ien().write(|w| w.0 = 0);

        // If bus is busy or in a bad state, perform a full reset
        let state = r.state().read();
        if state.busy() || state.nacked() {
            // Abort and send STOP to release the bus
            r.cmd().write(|w| {
                w.set_abort(true);
                w.set_stop(true);
            });
            
            // Wait for bus to become idle (with timeout)
            let mut timeout = 10000u32;
            while r.state().read().busy() && timeout > 0 {
                timeout -= 1;
            }
        }

        // Clear pending interrupts and TX/RX buffers
        r.cmd().write(|w| {
            w.set_clearpc(true);
            w.set_cleartx(true);
        });
        flush_rx(r);
        clear_interrupts(r);

        // Send START + address (write mode)
        r.txdata().write(|w| w.set_txdata(address << 1));
        r.cmd().write(|w| w.set_start(true));

        // Wait for ACK on address
        self.async_wait_ack(r, s).await?;

        // Send write data
        for &byte in write {
            r.txdata().write(|w| w.set_txdata(byte));
            self.async_wait_ack(r, s).await?;
        }

        // Send repeated START + address (read mode)
        r.cmd().write(|w| w.set_start(true));
        r.txdata().write(|w| w.set_txdata((address << 1) | 1));

        // Wait for ACK on address
        self.async_wait_ack(r, s).await?;

        // Pre-arm NACK for last byte
        let read_len = read.len();
        if read_len == 1 {
            r.cmd().write(|w| w.set_nack(true));
        }

        // Read data bytes
        for (i, byte) in read.iter_mut().enumerate() {
            self.async_wait_rxdata(r, s).await?;
            *byte = r.rxdata().read().rxdata();

            if i < read_len - 1 {
                r.cmd().write(|w| w.set_ack(true));
                if i == read_len - 2 {
                    r.cmd().write(|w| w.set_nack(true));
                }
            }
        }

        // Send STOP
        r.cmd().write(|w| w.set_stop(true));
        self.async_wait_stop(r, s).await?;

        Ok(())
    }

    // ========================================================================
    // Blocking helper methods
    // ========================================================================

    fn blocking_wait_ack(&self) -> Result<(), Error> {
        let r = T::regs();
        loop {
            let if_flags = r.if_().read();
            if if_flags.buserr() {
                clear_interrupts(r);
                return Err(Error::Bus);
            }
            if if_flags.arblost() {
                clear_interrupts(r);
                return Err(Error::ArbitrationLoss);
            }
            if if_flags.nack() {
                clear_interrupts(r);
                r.cmd().write(|w| w.set_stop(true));
                return Err(Error::Nack);
            }
            if if_flags.ack() {
                r.if_().write(|w| w.set_ack(true)); // Clear ACK flag
                return Ok(());
            }
        }
    }

    fn blocking_wait_rxdata(&self) -> Result<(), Error> {
        let r = T::regs();
        loop {
            let if_flags = r.if_().read();
            if if_flags.buserr() {
                clear_interrupts(r);
                return Err(Error::Bus);
            }
            if if_flags.arblost() {
                clear_interrupts(r);
                return Err(Error::ArbitrationLoss);
            }
            if r.status().read().rxdatav() {
                return Ok(());
            }
        }
    }

    fn blocking_wait_stop(&self) -> Result<(), Error> {
        let r = T::regs();
        loop {
            let if_flags = r.if_().read();
            if if_flags.mstop() {
                r.if_().write(|w| w.set_mstop(true)); // Clear MSTOP flag
                return Ok(());
            }
            if if_flags.buserr() {
                clear_interrupts(r);
                return Err(Error::Bus);
            }
        }
    }

    // ========================================================================
    // Async helper methods
    // ========================================================================

    async fn async_wait_ack(
        &self,
        r: pac::i2c0::I2c0,
        s: &'static State,
    ) -> Result<(), Error> {
        poll_fn(|cx| {
            s.waker.register(cx.waker());

            let if_flags = r.if_().read();
            let state = r.state().read();
            
            if if_flags.buserr() {
                clear_interrupts(r);
                return Poll::Ready(Err(Error::Bus));
            }
            if if_flags.arblost() {
                clear_interrupts(r);
                return Poll::Ready(Err(Error::ArbitrationLoss));
            }
            if if_flags.nack() || state.nacked() {
                // Send STOP and wait for bus to be released
                r.cmd().write(|w| w.set_stop(true));
                let mut timeout = 10000u32;
                while r.state().read().busy() && timeout > 0 {
                    timeout -= 1;
                }
                clear_interrupts(r);
                return Poll::Ready(Err(Error::Nack));
            }
            if if_flags.ack() {
                r.if_().write(|w| w.set_ack(true));
                return Poll::Ready(Ok(()));
            }

            // Enable interrupts for ACK/NACK/errors
            r.ien().write(|w| {
                w.set_ack(true);
                w.set_nack(true);
                w.set_buserr(true);
                w.set_arblost(true);
            });

            Poll::Pending
        })
        .await
    }

    async fn async_wait_rxdata(
        &self,
        r: pac::i2c0::I2c0,
        s: &'static State,
    ) -> Result<(), Error> {
        poll_fn(|cx| {
            s.waker.register(cx.waker());

            let if_flags = r.if_().read();
            let state = r.state().read();
            
            if if_flags.buserr() {
                clear_interrupts(r);
                return Poll::Ready(Err(Error::Bus));
            }
            if if_flags.arblost() {
                clear_interrupts(r);
                return Poll::Ready(Err(Error::ArbitrationLoss));
            }
            // Check for NACK condition - transaction has failed
            if state.nacked() {
                // Send STOP and wait for bus to be released
                r.cmd().write(|w| w.set_stop(true));
                let mut timeout = 10000u32;
                while r.state().read().busy() && timeout > 0 {
                    timeout -= 1;
                }
                clear_interrupts(r);
                return Poll::Ready(Err(Error::Nack));
            }
            // Check if bus is no longer busy and master - transaction aborted
            if !state.busy() && !state.master() {
                clear_interrupts(r);
                return Poll::Ready(Err(Error::Bus));
            }
            
            let status = r.status().read();
            if status.rxdatav() {
                return Poll::Ready(Ok(()));
            }

            // Enable interrupts for RXDATAV/errors
            r.ien().write(|w| {
                w.set_rxdatav(true);
                w.set_buserr(true);
                w.set_arblost(true);
            });

            Poll::Pending
        })
        .await
    }

    async fn async_wait_stop(
        &self,
        r: pac::i2c0::I2c0,
        s: &'static State,
    ) -> Result<(), Error> {
        poll_fn(|cx| {
            s.waker.register(cx.waker());

            let if_flags = r.if_().read();
            if if_flags.mstop() {
                r.if_().write(|w| w.set_mstop(true));
                return Poll::Ready(Ok(()));
            }
            if if_flags.buserr() {
                clear_interrupts(r);
                return Poll::Ready(Err(Error::Bus));
            }

            // Enable interrupt for MSTOP
            r.ien().write(|w| {
                w.set_mstop(true);
                w.set_buserr(true);
            });

            Poll::Pending
        })
        .await
    }
}

impl<'d, T: Instance> Drop for I2c<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();

        // Disable I2C
        r.en().write(|w| w.set_en(pac::i2c0::vals::En::DISABLE));

        // Deconfigure pins
        deconfigure_pins::<T>();
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Reset the I2C peripheral to default state.
fn i2c_reset(r: pac::i2c0::I2c0) {
    // Disable first
    r.en().write(|w| w.set_en(pac::i2c0::vals::En::DISABLE));

    // Cancel ongoing operations and clear TX buffer
    r.cmd().write(|w| {
        w.set_clearpc(true);
        w.set_cleartx(true);
        w.set_abort(true);
    });

    // Reset registers
    r.ctrl().write(|_| {});
    r.clkdiv().write(|_| {});
    r.saddr().write(|_| {});
    r.saddrmask().write(|_| {});
    r.ien().write(|_| {});

    // Flush RX and clear interrupts
    flush_rx(r);
    clear_interrupts(r);
}

/// Flush the RX buffer.
fn flush_rx(r: pac::i2c0::I2c0) {
    while r.status().read().rxdatav() {
        let _ = r.rxdata().read();
    }
    // Clear RXDATAV flag using IF_CLR
    clear_interrupt_flags(r, 1 << 5); // RXDATAV is bit 5
}

/// Clear specified interrupt flags using the IF_CLR register.
/// On EFR32 Series 2, the CLR register is at base + 0x2000 + offset.
fn clear_interrupt_flags(r: pac::i2c0::I2c0, flags: u32) {
    // IF register is at offset 60 (0x3C) from the I2C base
    // IF_CLR is at base + 0x2000 + 0x3C
    const IF_OFFSET: usize = 60;
    const CLR_OFFSET: usize = 0x2000;
    
    let base = r.as_ptr() as usize;
    let if_clr_addr = (base + CLR_OFFSET + IF_OFFSET) as *mut u32;
    
    // Safety: We're writing to a valid peripheral register
    unsafe {
        core::ptr::write_volatile(if_clr_addr, flags);
    }
}

/// Clear all interrupt flags.
fn clear_interrupts(r: pac::i2c0::I2c0) {
    clear_interrupt_flags(r, 0xFFFF_FFFF);
}

/// Set the I2C bus frequency.
fn set_bus_freq(r: pac::i2c0::I2c0, ref_freq: u32, speed: Speed) {
    let freq_scl = speed.frequency();
    let n_sum = speed.n_sum();
    let clhr = speed.clhr();

    // Set clock low/high ratio
    r.ctrl().modify(|w| w.set_clhr(clhr));

    // Calculate clock divider
    // DIV = (freqRef / (freqScl * N)) - 1, where N = Nlow + Nhigh
    // With I2C_CR_MAX = 8 correction factor
    const I2C_CR_MAX: u32 = 8;

    let denominator = n_sum * freq_scl;
    let div = if denominator > 0 {
        let numerator = ref_freq.saturating_sub(I2C_CR_MAX * freq_scl);
        numerator.div_ceil(denominator).saturating_sub(1)
    } else {
        0
    };

    r.clkdiv().write(|w| w.set_div(div as u16));
}

/// Configure GPIO pins for I2C.
fn configure_pins<T: Instance>(scl: &Peri<'_, AnyPin>, sda: &Peri<'_, AnyPin>) {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    let scl_port = scl.pin_port() / 16;
    let scl_pin = scl.pin_port() % 16;
    let sda_port = sda.pin_port() / 16;
    let sda_pin = sda.pin_port() % 16;

    // Configure pins as wired-AND with pull-up (open-drain)
    scl.mode_w(pac::gpio::vals::PortMode::WIREDANDPULLUP);
    sda.mode_w(pac::gpio::vals::PortMode::WIREDANDPULLUP);

    // Set pins high initially
    scl.set_high();
    sda.set_high();

    // Configure GPIO routing
    match T::index() {
        0 => {
            gpio.i2c0_sclroute().write(|w| {
                w.set_port(scl_port);
                w.set_pin(scl_pin);
            });
            gpio.i2c0_sdaroute().write(|w| {
                w.set_port(sda_port);
                w.set_pin(sda_pin);
            });
            gpio.i2c0_routeen().write(|w| {
                w.set_sclpen(true);
                w.set_sdapen(true);
            });
        }
        1 => {
            gpio.i2c1_sclroute().write(|w| {
                w.set_port(scl_port);
                w.set_pin(scl_pin);
            });
            gpio.i2c1_sdaroute().write(|w| {
                w.set_port(sda_port);
                w.set_pin(sda_pin);
            });
            gpio.i2c1_routeen().write(|w| {
                w.set_sclpen(true);
                w.set_sdapen(true);
            });
        }
        _ => {}
    }
}

/// Deconfigure GPIO pins when I2C is dropped.
fn deconfigure_pins<T: Instance>() {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    match T::index() {
        0 => {
            gpio.i2c0_routeen().write(|w| {
                w.set_sclpen(false);
                w.set_sdapen(false);
            });
        }
        1 => {
            gpio.i2c1_routeen().write(|w| {
                w.set_sclpen(false);
                w.set_sdapen(false);
            });
        }
        _ => {}
    }
}

// ============================================================================
// Instance trait and implementations
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::i2c0::I2c0;
    fn state() -> &'static State;
    fn index() -> u8;
}

/// I2C peripheral instance trait.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {
    /// Interrupt for this peripheral.
    type Interrupt: interrupt::typelevel::Interrupt;
}

// ============================================================================
// embedded-hal trait implementations
// ============================================================================

impl embedded_hal::i2c::Error for Error {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        match self {
            Error::Bus => embedded_hal::i2c::ErrorKind::Bus,
            Error::ArbitrationLoss => embedded_hal::i2c::ErrorKind::ArbitrationLoss,
            Error::Nack => embedded_hal::i2c::ErrorKind::NoAcknowledge(
                embedded_hal::i2c::NoAcknowledgeSource::Unknown,
            ),
            Error::Timeout => embedded_hal::i2c::ErrorKind::Other,
        }
    }
}

impl<'d, T: Instance> embedded_hal::i2c::ErrorType for I2c<'d, T> {
    type Error = Error;
}

impl<'d, T: Instance> embedded_hal::i2c::I2c for I2c<'d, T> {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        // Handle common cases efficiently
        match operations {
            [] => Ok(()),
            [embedded_hal::i2c::Operation::Write(data)] => self.blocking_write(address, data),
            [embedded_hal::i2c::Operation::Read(buffer)] => self.blocking_read(address, buffer),
            [embedded_hal::i2c::Operation::Write(write), embedded_hal::i2c::Operation::Read(read)] => {
                self.blocking_write_read(address, write, read)
            }
            _ => {
                // For complex multi-operation transactions, fall back to individual ops
                for op in operations {
                    match op {
                        embedded_hal::i2c::Operation::Read(buffer) => {
                            self.blocking_read(address, buffer)?;
                        }
                        embedded_hal::i2c::Operation::Write(data) => {
                            self.blocking_write(address, data)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

impl<'d, T: Instance> embedded_hal_async::i2c::I2c for I2c<'d, T> {
    async fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation<'_>],
    ) -> Result<(), Self::Error> {
        // Handle common cases efficiently
        match operations {
            [] => Ok(()),
            [embedded_hal::i2c::Operation::Write(data)] => self.write(address, data).await,
            [embedded_hal::i2c::Operation::Read(buffer)] => self.read(address, buffer).await,
            [embedded_hal::i2c::Operation::Write(write), embedded_hal::i2c::Operation::Read(read)] => {
                self.write_read(address, write, read).await
            }
            _ => {
                // For complex multi-operation transactions, fall back to individual ops
                // Note: This is not ideal as it sends STOP between operations
                for op in operations {
                    match op {
                        embedded_hal::i2c::Operation::Read(buffer) => {
                            self.read(address, buffer).await?;
                        }
                        embedded_hal::i2c::Operation::Write(data) => {
                            self.write(address, data).await?;
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for I2C peripherals.
#[macro_export]
macro_rules! impl_i2c {
    ($type:ident, $pac_type:ident, $irq:ident, $index:expr) => {
        impl $crate::i2c::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::i2c0::I2c0 {
                unsafe { $crate::pac::i2c0::I2c0::from_ptr($crate::pac::$pac_type.as_ptr()) }
            }
            fn state() -> &'static $crate::i2c::State {
                static STATE: $crate::i2c::State = $crate::i2c::State::new();
                &STATE
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::i2c::Instance for $crate::peripherals::$type {
            type Interrupt = $crate::interrupt::typelevel::$irq;
        }
    };
}
