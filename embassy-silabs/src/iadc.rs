//! Incremental Analog to Digital Converter (IADC) driver.
//!
//! This driver provides async and blocking ADC functionality for Silicon Labs EFR32 series MCUs.
//!
//! # Example - One-shot conversion
//!
//! ```no_run,ignore
//! use embassy_silabs::iadc::{Iadc, Config, Input};
//!
//! let config = Config::default();
//! let mut adc = Iadc::new(p.IADC0, Irqs, config);
//!
//! // Read from pin PA0 (single-ended, referenced to GND)
//! let sample = adc.read(Input::from_pin(&p.PA_00)).await;
//! ```
#![warn(missing_docs)]

use core::future::poll_fn;
use core::marker::PhantomData;
use core::task::Poll;

use embassy_hal_internal::{Peri, PeripheralType};
use embassy_sync::waitqueue::AtomicWaker;

use crate::chip::pac;
use crate::gpio::Pin as GpioPin;
use crate::interrupt;
use crate::interrupt::typelevel::Interrupt;

/// IADC voltage reference selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Reference {
    /// Internal 1.2V bandgap reference (buffered) to ground.
    #[default]
    Internal1V2,
    /// External reference (unbuffered) VREFP to VREFN. (1.25V default calibration)
    External1V25,
    /// VDDX (unbuffered) to ground.
    Vddx,
    /// 0.8 * VDDX (buffered) to ground.
    Vddx0P8Buf,
}

impl Reference {
    fn to_pac_val(self) -> pac::iadc0::vals::Cfg0Refsel {
        match self {
            Reference::Internal1V2 => pac::iadc0::vals::Cfg0Refsel::VBGR,
            Reference::External1V25 => pac::iadc0::vals::Cfg0Refsel::VREF,
            Reference::Vddx => pac::iadc0::vals::Cfg0Refsel::VDDX,
            Reference::Vddx0P8Buf => pac::iadc0::vals::Cfg0Refsel::VDDX0P8BUF,
        }
    }

    /// Get the reference voltage in millivolts.
    pub fn voltage_mv(&self) -> u32 {
        match self {
            Reference::Internal1V2 => 1210,
            Reference::External1V25 => 1250,
            Reference::Vddx => 3300, // Typical AVDD
            Reference::Vddx0P8Buf => 2640, // 0.8 * 3300
        }
    }
}

/// IADC oversampling ratio for high-speed mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Oversampling {
    /// 2x oversampling
    #[default]
    Osr2,
    /// 4x oversampling
    Osr4,
    /// 8x oversampling
    Osr8,
    /// 16x oversampling
    Osr16,
    /// 32x oversampling
    Osr32,
    /// 64x oversampling
    Osr64,
}

impl Oversampling {
    fn to_pac_val(self) -> pac::iadc0::vals::Cfg0Osrhs {
        match self {
            Oversampling::Osr2 => pac::iadc0::vals::Cfg0Osrhs::HISPD2,
            Oversampling::Osr4 => pac::iadc0::vals::Cfg0Osrhs::HISPD4,
            Oversampling::Osr8 => pac::iadc0::vals::Cfg0Osrhs::HISPD8,
            Oversampling::Osr16 => pac::iadc0::vals::Cfg0Osrhs::HISPD16,
            Oversampling::Osr32 => pac::iadc0::vals::Cfg0Osrhs::HISPD32,
            Oversampling::Osr64 => pac::iadc0::vals::Cfg0Osrhs::HISPD64,
        }
    }
}

/// IADC analog gain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Gain {
    /// 0.5x analog gain
    Gain0P5,
    /// 1x analog gain
    #[default]
    Gain1,
    /// 2x analog gain
    Gain2,
    /// 3x analog gain
    Gain3,
    /// 4x analog gain
    Gain4,
}

impl Gain {
    fn to_pac_val(self) -> pac::iadc0::vals::Cfg0Analoggain {
        match self {
            Gain::Gain0P5 => pac::iadc0::vals::Cfg0Analoggain::ANAGAIN0P5,
            Gain::Gain1 => pac::iadc0::vals::Cfg0Analoggain::ANAGAIN1,
            Gain::Gain2 => pac::iadc0::vals::Cfg0Analoggain::ANAGAIN2,
            Gain::Gain3 => pac::iadc0::vals::Cfg0Analoggain::ANAGAIN3,
            Gain::Gain4 => pac::iadc0::vals::Cfg0Analoggain::ANAGAIN4,
        }
    }
}

/// IADC result alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Alignment {
    /// 12-bit right aligned
    #[default]
    Right12,
    /// 12-bit left aligned
    Left12,
}

impl Alignment {
    fn to_pac_val(self) -> pac::iadc0::vals::SinglefifocfgAlignment {
        match self {
            Alignment::Right12 => pac::iadc0::vals::SinglefifocfgAlignment::RIGHT12,
            Alignment::Left12 => pac::iadc0::vals::SinglefifocfgAlignment::LEFT12,
        }
    }
}

/// IADC configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// Voltage reference selection.
    pub reference: Reference,
    /// Oversampling ratio.
    pub oversampling: Oversampling,
    /// Analog gain.
    pub gain: Gain,
    /// Result alignment.
    pub alignment: Alignment,
    /// Source clock frequency in Hz. Set to 0 to use default (20 MHz).
    pub src_clk_freq: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            reference: Reference::Internal1V2,
            oversampling: Oversampling::Osr2,
            gain: Gain::Gain1,
            alignment: Alignment::Right12,
            src_clk_freq: 0, // Use default
        }
    }
}

/// IADC positive input selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PosInput {
    /// Ground
    Gnd,
    /// AVDD / 4
    Avdd,
    /// DVDD / 4
    Dvdd,
    /// VSS
    Vss,
    /// Decouple capacitor
    Decouple,
    /// GPIO pin (port, pin)
    Pin(u8, u8),
}

impl PosInput {
    /// Create a positive input from a GPIO pin.
    pub fn from_pin<P: GpioPin>(pin: &Peri<'_, P>) -> Self {
        let port = pin.pin_port() / 16;
        let pin_num = pin.pin_port() % 16;
        PosInput::Pin(port, pin_num)
    }

    fn to_raw(self) -> u16 {
        match self {
            PosInput::Gnd => 0x00 << 4,
            PosInput::Avdd => 0x01 << 4,
            PosInput::Dvdd => (0x01 << 4) | 4,
            PosInput::Vss => (0x01 << 4) | 2,
            PosInput::Decouple => (0x01 << 4) | 7,
            PosInput::Pin(port, pin) => ((0x04 + port) as u16) << 4 | (pin as u16),
        }
    }
}

/// IADC negative input selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NegInput {
    /// Ground (default for single-ended)
    #[default]
    Gnd,
    /// GPIO pin (port, pin) for differential
    Pin(u8, u8),
}

impl NegInput {
    /// Create a negative input from a GPIO pin (for differential measurement).
    pub fn from_pin<P: GpioPin>(pin: &Peri<'_, P>) -> Self {
        let port = pin.pin_port() / 16;
        let pin_num = pin.pin_port() % 16;
        NegInput::Pin(port, pin_num)
    }

    fn to_raw(self) -> u16 {
        match self {
            NegInput::Gnd => 0x01, // GND with odd mux
            NegInput::Pin(port, pin) => ((0x04 + port) as u16) << 4 | (pin as u16),
        }
    }
}

/// IADC input configuration (positive and negative inputs).
#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// Positive input
    pub pos: PosInput,
    /// Negative input (Ground for single-ended)
    pub neg: NegInput,
}

impl Input {
    /// Create a single-ended input from a GPIO pin (referenced to ground).
    pub fn single_ended<P: GpioPin>(pin: &Peri<'_, P>) -> Self {
        Self {
            pos: PosInput::from_pin(pin),
            neg: NegInput::Gnd,
        }
    }

    /// Create a differential input from two GPIO pins.
    pub fn differential<P: GpioPin, N: GpioPin>(pos: &Peri<'_, P>, neg: &Peri<'_, N>) -> Self {
        Self {
            pos: PosInput::from_pin(pos),
            neg: NegInput::from_pin(neg),
        }
    }

    /// Create an input for measuring AVDD/4.
    pub fn avdd() -> Self {
        Self {
            pos: PosInput::Avdd,
            neg: NegInput::Gnd,
        }
    }

    /// Create an input for measuring DVDD/4.
    pub fn dvdd() -> Self {
        Self {
            pos: PosInput::Dvdd,
            neg: NegInput::Gnd,
        }
    }
}

/// IADC error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// Conversion timeout
    Timeout,
    /// FIFO overflow or underflow
    FifoError,
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

/// Interrupt handler for IADC.
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
// IADC Driver
// ============================================================================

/// IADC driver for analog to digital conversion.
pub struct Iadc<'d, T: Instance> {
    _p: Peri<'d, T>,
    config: Config,
}

impl<'d, T: Instance> Iadc<'d, T> {
    /// Create a new IADC driver.
    pub fn new(
        iadc: Peri<'d, T>,
        _irq: impl interrupt::typelevel::Binding<T::Interrupt, InterruptHandler<T>> + 'd,
        config: Config,
    ) -> Self {
        let r = T::regs();

        // Enable IADC clock
        pac::CMU.clken0().modify(|w| w.set_iadc0(true));

        // Disable IADC before configuration
        r.en().write(|w| w.set_en(pac::iadc0::vals::En::DISABLE));

        // Calculate prescalers
        let src_clk_freq = if config.src_clk_freq == 0 {
            20_000_000 // Default to 20 MHz
        } else {
            config.src_clk_freq
        };

        // Calculate timebase (1us period)
        let timebase = ((src_clk_freq / 1_000_000) - 1).min(0x7F) as u8;

        // Configure CTRL register
        r.ctrl().write(|w| {
            w.set_timebase(timebase);
            w.set_warmupmode(pac::iadc0::vals::Warmupmode::NORMAL);
        });

        // Configure CFG0 (configuration 0)
        r.cfg0().write(|w| {
            w.set_adcmode(pac::iadc0::vals::Cfg0Adcmode::NORMAL);
            w.set_osrhs(config.oversampling.to_pac_val());
            w.set_analoggain(config.gain.to_pac_val());
            w.set_refsel(config.reference.to_pac_val());
            w.set_twoscompl(pac::iadc0::vals::Cfg0Twoscompl::AUTO);
        });

        // Configure single FIFO
        r.singlefifocfg().write(|w| {
            w.set_alignment(config.alignment.to_pac_val());
            w.set_showid(false);
        });

        // Configure trigger for immediate start
        r.trigger().write(|w| {
            w.set_singletrigsel(pac::iadc0::vals::Singletrigsel::IMMEDIATE);
            w.set_singletrigaction(pac::iadc0::vals::Singletrigaction::ONCE);
        });

        // Enable IADC
        r.en().write(|w| w.set_en(pac::iadc0::vals::En::ENABLE));

        // Enable interrupt in NVIC
        T::Interrupt::unpend();
        unsafe { T::Interrupt::enable() };

        Self { _p: iadc, config }
    }

    /// Perform a blocking single conversion.
    pub fn blocking_read(&mut self, input: Input) -> u16 {
        let r = T::regs();

        // Configure the input
        self.configure_single_input(input);

        // Start single conversion
        r.cmd().write(|w| w.set_singlestart(true));

        // Wait for conversion complete
        while !r.status().read().singlefifodv() {}

        // Read result
        r.singlefifodata().read().data() as u16
    }

    /// Perform an async single conversion.
    pub async fn read(&mut self, input: Input) -> u16 {
        let r = T::regs();
        let s = T::state();

        // Configure the input
        self.configure_single_input(input);

        // Start single conversion
        r.cmd().write(|w| w.set_singlestart(true));

        // Wait for conversion complete
        poll_fn(|cx| {
            s.waker.register(cx.waker());

            if r.status().read().singlefifodv() {
                Poll::Ready(())
            } else {
                // Enable single done interrupt
                r.ien().write(|w| w.set_singledone(true));
                Poll::Pending
            }
        })
        .await;

        // Clear interrupt flag
        r.if_().write(|w| w.set_singledone(true));

        // Read result
        r.singlefifodata().read().data() as u16
    }

    /// Read multiple samples into a buffer (blocking).
    pub fn blocking_read_many(&mut self, input: Input, buf: &mut [u16]) {
        for sample in buf.iter_mut() {
            *sample = self.blocking_read(input);
        }
    }

    /// Read multiple samples into a buffer (async).
    pub async fn read_many(&mut self, input: Input, buf: &mut [u16]) {
        for sample in buf.iter_mut() {
            *sample = self.read(input).await;
        }
    }

    /// Convert a raw ADC sample to millivolts.
    pub fn sample_to_mv(&self, sample: u16) -> u32 {
        let vref_mv = self.config.reference.voltage_mv();
        // For 12-bit right-aligned result
        (sample as u32 * vref_mv) / 4095
    }

    /// Get the resolution in bits.
    pub fn resolution(&self) -> u8 {
        12
    }

    fn configure_single_input(&self, input: Input) {
        let r = T::regs();

        let pos_raw = input.pos.to_raw();
        let neg_raw = input.neg.to_raw();

        r.single().write(|w| {
            // Set positive input (port in upper bits, pin in lower bits)
            w.set_portpos(pac::iadc0::vals::SinglePortpos::from_bits((pos_raw >> 4) as u8));
            w.set_pinpos((pos_raw & 0x0F) as u8);
            // Set negative input
            w.set_portneg(pac::iadc0::vals::SinglePortneg::from_bits((neg_raw >> 4) as u8));
            w.set_pinneg((neg_raw & 0x0F) as u8);
            // Use config 0
            w.set_cfg(pac::iadc0::vals::SingleCfg::CONFIG0);
        });
    }
}

impl<'d, T: Instance> Drop for Iadc<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();

        // Disable IADC
        r.en().write(|w| w.set_en(pac::iadc0::vals::En::DISABLE));

        // Disable interrupt
        T::Interrupt::disable();
    }
}

// ============================================================================
// Instance trait and implementations
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::iadc0::Iadc0;
    fn state() -> &'static State;
}

/// IADC peripheral instance trait.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {
    /// Interrupt for this peripheral.
    type Interrupt: interrupt::typelevel::Interrupt;
}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for IADC peripherals.
#[macro_export]
macro_rules! impl_iadc {
    ($type:ident, $pac_type:ident, $irq:ident) => {
        impl $crate::iadc::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::iadc0::Iadc0 {
                unsafe { $crate::pac::iadc0::Iadc0::from_ptr($crate::pac::$pac_type.as_ptr()) }
            }
            fn state() -> &'static $crate::iadc::State {
                static STATE: $crate::iadc::State = $crate::iadc::State::new();
                &STATE
            }
        }
        impl $crate::iadc::Instance for $crate::peripherals::$type {
            type Interrupt = $crate::interrupt::typelevel::$irq;
        }
    };
}
