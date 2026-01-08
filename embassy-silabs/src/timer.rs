//! Timer/Counter peripheral driver with PWM and input capture support.
//!
//! This driver provides timer, PWM, and input capture functionality for Silicon Labs EFR32 series MCUs.
//!
//! # Example - PWM Output
//!
//! ```no_run,ignore
//! use embassy_silabs::timer::{SimplePwm, Config};
//!
//! let config = Config::default();
//! let mut pwm = SimplePwm::new_ch0(p.TIMER0, p.PA_00, config);
//!
//! // Set frequency to 1 kHz
//! pwm.set_frequency(1_000);
//!
//! // Set duty cycle to 50%
//! pwm.set_duty(0, pwm.max_duty() / 2);
//!
//! // Enable the PWM output
//! pwm.enable(0);
//! ```
//!
//! # Example - Input Capture
//!
//! ```no_run,ignore
//! use embassy_silabs::timer::{InputCapture, CaptureConfig, Edge};
//!
//! // Configure for rising edge capture
//! let config = CaptureConfig {
//!     edge: Edge::Rising,
//!     filter: false,
//!     ..Default::default()
//! };
//!
//! let mut capture = InputCapture::new(p.TIMER0, p.PA_00, Irqs, config);
//!
//! // Wait for a capture event
//! let timestamp = capture.wait().await;
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

// ============================================================================
// Common Types
// ============================================================================

/// Timer prescaler values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u16)]
pub enum Prescaler {
    /// Divide by 1
    #[default]
    Div1 = 0,
    /// Divide by 2
    Div2 = 1,
    /// Divide by 4
    Div4 = 3,
    /// Divide by 8
    Div8 = 7,
    /// Divide by 16
    Div16 = 15,
    /// Divide by 32
    Div32 = 31,
    /// Divide by 64
    Div64 = 63,
    /// Divide by 128
    Div128 = 127,
    /// Divide by 256
    Div256 = 255,
    /// Divide by 512
    Div512 = 511,
    /// Divide by 1024
    Div1024 = 1023,
}

impl Prescaler {
    /// Get the division factor for this prescaler.
    pub fn divisor(&self) -> u32 {
        (*self as u32) + 1
    }

    /// Convert to PAC Presc type.
    fn to_pac(self) -> pac::timer0::vals::Presc {
        pac::timer0::vals::Presc::from_bits(self as u16)
    }
}

/// Timer counting mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CountingMode {
    /// Count up from 0 to TOP.
    #[default]
    Up,
    /// Count down from TOP to 0.
    Down,
    /// Count up then down (center-aligned PWM).
    UpDown,
}

/// Edge detection mode for input capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Edge {
    /// Capture on rising edge.
    #[default]
    Rising,
    /// Capture on falling edge.
    Falling,
    /// Capture on both edges.
    Both,
    /// No edge detection.
    None,
}

/// Capture channel selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Channel {
    /// Capture/Compare channel 0.
    #[default]
    Ch0 = 0,
    /// Capture/Compare channel 1.
    Ch1 = 1,
    /// Capture/Compare channel 2.
    Ch2 = 2,
}

// ============================================================================
// Timer Configuration
// ============================================================================

/// Timer configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// Timer prescaler.
    pub prescaler: Prescaler,
    /// Counting mode.
    pub mode: CountingMode,
    /// Reference clock frequency in Hz. Set to 0 to use default (20 MHz).
    pub ref_freq: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prescaler: Prescaler::Div1,
            mode: CountingMode::Up,
            ref_freq: 0,
        }
    }
}

/// Input capture configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct CaptureConfig {
    /// Timer prescaler.
    pub prescaler: Prescaler,
    /// Edge to capture on.
    pub edge: Edge,
    /// Enable digital filter on input.
    pub filter: bool,
    /// Reference clock frequency in Hz. Set to 0 to use default (20 MHz).
    pub ref_freq: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            prescaler: Prescaler::Div1,
            edge: Edge::Rising,
            filter: false,
            ref_freq: 0,
        }
    }
}

// ============================================================================
// State for async operations
// ============================================================================

/// Internal state shared between driver instances.
pub struct State {
    /// Waker for channel 0.
    pub waker_ch0: AtomicWaker,
    /// Waker for channel 1.
    pub waker_ch1: AtomicWaker,
    /// Waker for channel 2.
    pub waker_ch2: AtomicWaker,
    /// Waker for overflow.
    pub waker_overflow: AtomicWaker,
}

impl State {
    /// Create a new state instance.
    pub const fn new() -> Self {
        Self {
            waker_ch0: AtomicWaker::new(),
            waker_ch1: AtomicWaker::new(),
            waker_ch2: AtomicWaker::new(),
            waker_overflow: AtomicWaker::new(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// Interrupt handler for Timer.
pub struct InterruptHandler<T: Instance> {
    _phantom: PhantomData<T>,
}

impl<T: Instance> interrupt::typelevel::Handler<T::Interrupt> for InterruptHandler<T> {
    unsafe fn on_interrupt() {
        let r = T::regs();
        let s = T::state();

        let if_flags = r.if_().read();

        // Check which interrupt triggered and wake appropriate waker
        if if_flags.cc0() {
            // Clear the CC0 interrupt flag
            r.if_().write(|w| w.set_cc0(true));
            s.waker_ch0.wake();
        }
        if if_flags.cc1() {
            // Clear the CC1 interrupt flag
            r.if_().write(|w| w.set_cc1(true));
            s.waker_ch1.wake();
        }
        if if_flags.cc2() {
            // Clear the CC2 interrupt flag
            r.if_().write(|w| w.set_cc2(true));
            s.waker_ch2.wake();
        }
        if if_flags.of() {
            // Clear the overflow interrupt flag
            r.if_().write(|w| w.set_of(true));
            s.waker_overflow.wake();
        }
    }
}

// ============================================================================
// SimplePwm Driver
// ============================================================================

/// Simple PWM driver for a single timer with up to 3 channels.
pub struct SimplePwm<'d, T: Instance> {
    _phantom: PhantomData<&'d T>,
    ref_freq: u32,
    prescaler: Prescaler,
}

impl<'d, T: Instance> SimplePwm<'d, T> {
    /// Create a new PWM driver with a single output channel (CC0).
    pub fn new_ch0(
        _timer: Peri<'d, T>,
        pin: Peri<'d, impl GpioPin>,
        config: Config,
    ) -> Self {
        Self::new_inner(Some(pin.into()), None, None, config)
    }

    /// Create a new PWM driver with two output channels (CC0 and CC1).
    pub fn new_ch0_ch1(
        _timer: Peri<'d, T>,
        ch0_pin: Peri<'d, impl GpioPin>,
        ch1_pin: Peri<'d, impl GpioPin>,
        config: Config,
    ) -> Self {
        Self::new_inner(Some(ch0_pin.into()), Some(ch1_pin.into()), None, config)
    }

    /// Create a new PWM driver with three output channels (CC0, CC1, and CC2).
    pub fn new_ch0_ch1_ch2(
        _timer: Peri<'d, T>,
        ch0_pin: Peri<'d, impl GpioPin>,
        ch1_pin: Peri<'d, impl GpioPin>,
        ch2_pin: Peri<'d, impl GpioPin>,
        config: Config,
    ) -> Self {
        Self::new_inner(
            Some(ch0_pin.into()),
            Some(ch1_pin.into()),
            Some(ch2_pin.into()),
            config,
        )
    }

    fn new_inner(
        ch0_pin: Option<Peri<'d, AnyPin>>,
        ch1_pin: Option<Peri<'d, AnyPin>>,
        ch2_pin: Option<Peri<'d, AnyPin>>,
        config: Config,
    ) -> Self {
        let r = T::regs();

        // Enable timer clock
        enable_timer_clock::<T>();

        // Disable timer before configuration
        r.en().write(|w| w.set_en(false));

        // Configure timer mode
        r.cfg().write(|w| {
            w.set_presc(config.prescaler.to_pac());
            match config.mode {
                CountingMode::Up => w.set_mode(pac::timer0::vals::CfgMode::UP),
                CountingMode::Down => w.set_mode(pac::timer0::vals::CfgMode::DOWN),
                CountingMode::UpDown => w.set_mode(pac::timer0::vals::CfgMode::UPDOWN),
            }
        });

        // Configure CC0 for PWM mode
        if ch0_pin.is_some() {
            r.cc0_cfg().write(|w| {
                w.set_mode(pac::timer0::vals::Cc0CfgMode::PWM);
            });
            r.cc0_ctrl().write(|w| {
                w.set_outinv(false);
            });
        }

        // Configure CC1 for PWM mode
        if ch1_pin.is_some() {
            r.cc1_cfg().write(|w| {
                w.set_mode(pac::timer0::vals::Cc1CfgMode::PWM);
            });
            r.cc1_ctrl().write(|w| {
                w.set_outinv(false);
            });
        }

        // Configure CC2 for PWM mode
        if ch2_pin.is_some() {
            r.cc2_cfg().write(|w| {
                w.set_mode(pac::timer0::vals::Cc2CfgMode::PWM);
            });
            r.cc2_ctrl().write(|w| {
                w.set_outinv(false);
            });
        }

        // Configure GPIO pins
        configure_pwm_pins::<T>(&ch0_pin, &ch1_pin, &ch2_pin);

        // Set default TOP value (16-bit max)
        r.top().write(|w| w.set_top(0xFFFF));

        // Enable timer
        r.en().write(|w| w.set_en(true));

        // Start the timer
        r.cmd().write(|w| w.set_start(true));

        let ref_freq = if config.ref_freq == 0 {
            20_000_000 // Default 20 MHz
        } else {
            config.ref_freq
        };

        Self {
            _phantom: PhantomData,
            ref_freq,
            prescaler: config.prescaler,
        }
    }

    /// Get the maximum duty cycle value (TOP value).
    pub fn max_duty(&self) -> u16 {
        T::regs().top().read().top() as u16
    }

    /// Set the PWM frequency in Hz.
    ///
    /// This adjusts the TOP value to achieve the desired frequency.
    pub fn set_frequency(&mut self, freq_hz: u32) {
        if freq_hz == 0 {
            return;
        }

        let r = T::regs();
        let timer_freq = self.ref_freq / self.prescaler.divisor();
        let top = (timer_freq / freq_hz).saturating_sub(1).min(0xFFFF) as u16;

        r.top().write(|w| w.set_top(top as u32));
    }

    /// Get the current PWM frequency in Hz.
    pub fn get_frequency(&self) -> u32 {
        let r = T::regs();
        let top = r.top().read().top();
        if top == 0 {
            return 0;
        }

        let timer_freq = self.ref_freq / self.prescaler.divisor();
        timer_freq / (top + 1)
    }

    /// Set the duty cycle for a channel.
    ///
    /// The duty value should be between 0 and `max_duty()`.
    pub fn set_duty(&mut self, channel: Channel, duty: u16) {
        let r = T::regs();
        match channel {
            Channel::Ch0 => r.cc0_ocb().write(|w| w.set_ocb(duty as u32)),
            Channel::Ch1 => r.cc1_ocb().write(|w| w.set_ocb(duty as u32)),
            Channel::Ch2 => r.cc2_ocb().write(|w| w.set_ocb(duty as u32)),
        }
    }

    /// Get the current duty cycle for a channel.
    pub fn get_duty(&self, channel: Channel) -> u16 {
        let r = T::regs();
        match channel {
            Channel::Ch0 => r.cc0_oc().read().oc() as u16,
            Channel::Ch1 => r.cc1_oc().read().oc() as u16,
            Channel::Ch2 => r.cc2_oc().read().oc() as u16,
        }
    }

    /// Enable PWM output on a channel.
    pub fn enable(&mut self, channel: Channel) {
        let r = T::regs();
        match channel {
            Channel::Ch0 => r.cc0_cfg().modify(|w| w.set_mode(pac::timer0::vals::Cc0CfgMode::PWM)),
            Channel::Ch1 => r.cc1_cfg().modify(|w| w.set_mode(pac::timer0::vals::Cc1CfgMode::PWM)),
            Channel::Ch2 => r.cc2_cfg().modify(|w| w.set_mode(pac::timer0::vals::Cc2CfgMode::PWM)),
        }
    }

    /// Disable PWM output on a channel.
    pub fn disable(&mut self, channel: Channel) {
        let r = T::regs();
        match channel {
            Channel::Ch0 => r.cc0_cfg().modify(|w| w.set_mode(pac::timer0::vals::Cc0CfgMode::OFF)),
            Channel::Ch1 => r.cc1_cfg().modify(|w| w.set_mode(pac::timer0::vals::Cc1CfgMode::OFF)),
            Channel::Ch2 => r.cc2_cfg().modify(|w| w.set_mode(pac::timer0::vals::Cc2CfgMode::OFF)),
        }
    }

    /// Set duty cycle as a percentage (0-100).
    pub fn set_duty_percent(&mut self, channel: Channel, percent: u8) {
        let max = self.max_duty() as u32;
        let duty = (max * percent.min(100) as u32) / 100;
        self.set_duty(channel, duty as u16);
    }
}

impl<'d, T: Instance> Drop for SimplePwm<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();

        // Stop and disable timer
        r.cmd().write(|w| w.set_stop(true));
        r.en().write(|w| w.set_en(false));

        // Deconfigure pins
        deconfigure_pins::<T>();
    }
}

// ============================================================================
// InputCapture Driver
// ============================================================================

/// Input capture driver for a single channel.
///
/// This driver configures a timer channel in input capture mode and provides
/// async methods to wait for capture events.
pub struct InputCapture<'d, T: Instance> {
    _phantom: PhantomData<&'d T>,
    channel: Channel,
    ref_freq: u32,
    prescaler: Prescaler,
}

impl<'d, T: Instance> InputCapture<'d, T> {
    /// Create a new input capture driver on channel 0.
    pub fn new_ch0(
        _timer: Peri<'d, T>,
        pin: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::Interrupt, InterruptHandler<T>> + 'd,
        config: CaptureConfig,
    ) -> Self {
        Self::new_inner(pin.into(), Channel::Ch0, config)
    }

    /// Create a new input capture driver on channel 1.
    pub fn new_ch1(
        _timer: Peri<'d, T>,
        pin: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::Interrupt, InterruptHandler<T>> + 'd,
        config: CaptureConfig,
    ) -> Self {
        Self::new_inner(pin.into(), Channel::Ch1, config)
    }

    /// Create a new input capture driver on channel 2.
    pub fn new_ch2(
        _timer: Peri<'d, T>,
        pin: Peri<'d, impl GpioPin>,
        _irq: impl interrupt::typelevel::Binding<T::Interrupt, InterruptHandler<T>> + 'd,
        config: CaptureConfig,
    ) -> Self {
        Self::new_inner(pin.into(), Channel::Ch2, config)
    }

    fn new_inner(pin: Peri<'d, AnyPin>, channel: Channel, config: CaptureConfig) -> Self {
        let r = T::regs();

        // Enable timer clock
        enable_timer_clock::<T>();

        // Disable timer before configuration
        r.en().write(|w| w.set_en(false));

        // Configure timer mode (count up for capture)
        r.cfg().write(|w| {
            w.set_presc(config.prescaler.to_pac());
            w.set_mode(pac::timer0::vals::CfgMode::UP);
        });

        // Configure the capture channel
        match channel {
            Channel::Ch0 => {
                let icedge = match config.edge {
                    Edge::Rising => pac::timer0::vals::Cc0CtrlIcedge::RISING,
                    Edge::Falling => pac::timer0::vals::Cc0CtrlIcedge::FALLING,
                    Edge::Both => pac::timer0::vals::Cc0CtrlIcedge::BOTH,
                    Edge::None => pac::timer0::vals::Cc0CtrlIcedge::NONE,
                };
                let filt = if config.filter {
                    pac::timer0::vals::Cc0CfgFilt::ENABLE
                } else {
                    pac::timer0::vals::Cc0CfgFilt::DISABLE
                };
                r.cc0_cfg().write(|w| {
                    w.set_mode(pac::timer0::vals::Cc0CfgMode::INPUTCAPTURE);
                    w.set_insel(pac::timer0::vals::Cc0CfgInsel::PIN);
                    w.set_filt(filt);
                });
                r.cc0_ctrl().write(|w| {
                    w.set_icedge(icedge);
                });
                // Enable CC0 interrupt
                r.ien().modify(|w| w.set_cc0(true));
            }
            Channel::Ch1 => {
                let icedge = match config.edge {
                    Edge::Rising => pac::timer0::vals::Cc1CtrlIcedge::RISING,
                    Edge::Falling => pac::timer0::vals::Cc1CtrlIcedge::FALLING,
                    Edge::Both => pac::timer0::vals::Cc1CtrlIcedge::BOTH,
                    Edge::None => pac::timer0::vals::Cc1CtrlIcedge::NONE,
                };
                let filt = if config.filter {
                    pac::timer0::vals::Cc1CfgFilt::ENABLE
                } else {
                    pac::timer0::vals::Cc1CfgFilt::DISABLE
                };
                r.cc1_cfg().write(|w| {
                    w.set_mode(pac::timer0::vals::Cc1CfgMode::INPUTCAPTURE);
                    w.set_insel(pac::timer0::vals::Cc1CfgInsel::PIN);
                    w.set_filt(filt);
                });
                r.cc1_ctrl().write(|w| {
                    w.set_icedge(icedge);
                });
                // Enable CC1 interrupt
                r.ien().modify(|w| w.set_cc1(true));
            }
            Channel::Ch2 => {
                let icedge = match config.edge {
                    Edge::Rising => pac::timer0::vals::Cc2CtrlIcedge::RISING,
                    Edge::Falling => pac::timer0::vals::Cc2CtrlIcedge::FALLING,
                    Edge::Both => pac::timer0::vals::Cc2CtrlIcedge::BOTH,
                    Edge::None => pac::timer0::vals::Cc2CtrlIcedge::NONE,
                };
                let filt = if config.filter {
                    pac::timer0::vals::Cc2CfgFilt::ENABLE
                } else {
                    pac::timer0::vals::Cc2CfgFilt::DISABLE
                };
                r.cc2_cfg().write(|w| {
                    w.set_mode(pac::timer0::vals::Cc2CfgMode::INPUTCAPTURE);
                    w.set_insel(pac::timer0::vals::Cc2CfgInsel::PIN);
                    w.set_filt(filt);
                });
                r.cc2_ctrl().write(|w| {
                    w.set_icedge(icedge);
                });
                // Enable CC2 interrupt
                r.ien().modify(|w| w.set_cc2(true));
            }
        }

        // Configure GPIO pin for input
        configure_capture_pin::<T>(&pin, channel);

        // Set TOP to maximum for free-running counter
        r.top().write(|w| w.set_top(0xFFFFFFFF));

        // Enable timer
        r.en().write(|w| w.set_en(true));

        // Start the timer
        r.cmd().write(|w| w.set_start(true));

        // Enable interrupt in NVIC
        T::Interrupt::unpend();
        unsafe { T::Interrupt::enable() };

        let ref_freq = if config.ref_freq == 0 {
            20_000_000 // Default 20 MHz
        } else {
            config.ref_freq
        };

        Self {
            _phantom: PhantomData,
            channel,
            ref_freq,
            prescaler: config.prescaler,
        }
    }

    /// Wait for a capture event and return the captured timestamp.
    ///
    /// This method will block until an edge is detected on the input pin.
    /// The returned value is the timer counter value at the moment of capture.
    pub async fn wait(&mut self) -> u32 {
        let r = T::regs();
        let s = T::state();

        // Get the appropriate waker and enable interrupt
        #[allow(clippy::type_complexity)]
        let (waker, enable_irq, read_icf): (
            &AtomicWaker,
            fn(&pac::timer0::Timer0),
            fn(&pac::timer0::Timer0) -> u32,
        ) = match self.channel {
            Channel::Ch0 => (
                &s.waker_ch0,
                |r| r.ien().modify(|w| w.set_cc0(true)),
                |r| r.cc0_icf().read().icf(),
            ),
            Channel::Ch1 => (
                &s.waker_ch1,
                |r| r.ien().modify(|w| w.set_cc1(true)),
                |r| r.cc1_icf().read().icf(),
            ),
            Channel::Ch2 => (
                &s.waker_ch2,
                |r| r.ien().modify(|w| w.set_cc2(true)),
                |r| r.cc2_icf().read().icf(),
            ),
        };

        poll_fn(|cx| {
            waker.register(cx.waker());

            // Check if there's data in the capture FIFO
            let status = r.status().read();
            let fifo_not_empty = match self.channel {
                Channel::Ch0 => !status.icfempty0(),
                Channel::Ch1 => !status.icfempty1(),
                Channel::Ch2 => !status.icfempty2(),
            };

            if fifo_not_empty {
                // Read captured value from FIFO
                let captured = read_icf(&r);
                Poll::Ready(captured)
            } else {
                // Enable interrupt and wait
                enable_irq(&r);
                Poll::Pending
            }
        })
        .await
    }

    /// Try to read a capture value without blocking.
    ///
    /// Returns `Some(value)` if a capture has occurred, `None` otherwise.
    pub fn try_read(&mut self) -> Option<u32> {
        let r = T::regs();

        // Check if FIFO has data
        let status = r.status().read();
        let fifo_not_empty = match self.channel {
            Channel::Ch0 => !status.icfempty0(),
            Channel::Ch1 => !status.icfempty1(),
            Channel::Ch2 => !status.icfempty2(),
        };

        if fifo_not_empty {
            let value = match self.channel {
                Channel::Ch0 => r.cc0_icf().read().icf(),
                Channel::Ch1 => r.cc1_icf().read().icf(),
                Channel::Ch2 => r.cc2_icf().read().icf(),
            };
            Some(value)
        } else {
            None
        }
    }

    /// Get the current counter value.
    pub fn counter(&self) -> u32 {
        T::regs().cnt().read().cnt()
    }

    /// Clear the counter.
    pub fn clear(&mut self) {
        T::regs().cnt().write(|w| w.set_cnt(0));
    }

    /// Get the timer frequency in Hz.
    pub fn frequency(&self) -> u32 {
        self.ref_freq / self.prescaler.divisor()
    }

    /// Convert a captured timestamp to microseconds.
    pub fn capture_to_us(&self, capture: u32) -> u32 {
        let freq = self.frequency();
        if freq == 0 {
            return 0;
        }
        // capture * 1_000_000 / freq, but avoid overflow
        ((capture as u64 * 1_000_000) / freq as u64) as u32
    }

    /// Measure pulse width between two edges.
    ///
    /// This captures two consecutive edges and returns the time difference.
    pub async fn measure_pulse_width(&mut self) -> u32 {
        let first = self.wait().await;
        let second = self.wait().await;
        second.wrapping_sub(first)
    }

    /// Measure frequency of an input signal.
    ///
    /// This captures two rising edges and calculates the frequency.
    /// Note: The edge configuration should be set to Rising for accurate results.
    pub async fn measure_frequency(&mut self) -> u32 {
        let period = self.measure_pulse_width().await;
        if period == 0 {
            return 0;
        }
        self.frequency() / period
    }
}

impl<'d, T: Instance> Drop for InputCapture<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();

        // Disable channel interrupts
        match self.channel {
            Channel::Ch0 => r.ien().modify(|w| w.set_cc0(false)),
            Channel::Ch1 => r.ien().modify(|w| w.set_cc1(false)),
            Channel::Ch2 => r.ien().modify(|w| w.set_cc2(false)),
        }

        // Stop and disable timer
        r.cmd().write(|w| w.set_stop(true));
        r.en().write(|w| w.set_en(false));

        // Deconfigure pins
        deconfigure_pins::<T>();
    }
}

// ============================================================================
// Timer Counter Driver
// ============================================================================

/// General-purpose timer/counter driver.
///
/// This driver provides basic timer functionality including:
/// - Free-running counter
/// - One-shot mode
/// - Counter mode (external clock)
pub struct Timer<'d, T: Instance> {
    _phantom: PhantomData<&'d T>,
    ref_freq: u32,
    prescaler: Prescaler,
}

impl<'d, T: Instance> Timer<'d, T> {
    /// Create a new timer in free-running mode.
    pub fn new(_timer: Peri<'d, T>, config: Config) -> Self {
        let r = T::regs();

        // Enable timer clock
        enable_timer_clock::<T>();

        // Disable timer before configuration
        r.en().write(|w| w.set_en(false));

        // Configure timer mode
        r.cfg().write(|w| {
            w.set_presc(config.prescaler.to_pac());
            match config.mode {
                CountingMode::Up => w.set_mode(pac::timer0::vals::CfgMode::UP),
                CountingMode::Down => w.set_mode(pac::timer0::vals::CfgMode::DOWN),
                CountingMode::UpDown => w.set_mode(pac::timer0::vals::CfgMode::UPDOWN),
            }
        });

        // Set TOP to maximum for free-running counter
        r.top().write(|w| w.set_top(0xFFFFFFFF));

        // Enable timer
        r.en().write(|w| w.set_en(true));

        let ref_freq = if config.ref_freq == 0 {
            20_000_000 // Default 20 MHz
        } else {
            config.ref_freq
        };

        Self {
            _phantom: PhantomData,
            ref_freq,
            prescaler: config.prescaler,
        }
    }

    /// Start the timer.
    pub fn start(&mut self) {
        T::regs().cmd().write(|w| w.set_start(true));
    }

    /// Stop the timer.
    pub fn stop(&mut self) {
        T::regs().cmd().write(|w| w.set_stop(true));
    }

    /// Get the current counter value.
    pub fn counter(&self) -> u32 {
        T::regs().cnt().read().cnt()
    }

    /// Set the counter value.
    pub fn set_counter(&mut self, value: u32) {
        T::regs().cnt().write(|w| w.set_cnt(value));
    }

    /// Clear the counter to zero.
    pub fn clear(&mut self) {
        self.set_counter(0);
    }

    /// Get the TOP value.
    pub fn top(&self) -> u32 {
        T::regs().top().read().top()
    }

    /// Set the TOP value.
    pub fn set_top(&mut self, top: u32) {
        T::regs().top().write(|w| w.set_top(top));
    }

    /// Get the timer frequency in Hz.
    pub fn frequency(&self) -> u32 {
        self.ref_freq / self.prescaler.divisor()
    }

    /// Check if the timer is running.
    pub fn is_running(&self) -> bool {
        T::regs().status().read().running()
    }

    /// Enable one-shot mode.
    ///
    /// In one-shot mode, the timer will stop automatically when it reaches TOP.
    pub fn set_one_shot(&mut self, enabled: bool) {
        T::regs().cfg().modify(|w| w.set_osmen(enabled));
    }
}

impl<'d, T: Instance> Drop for Timer<'d, T> {
    fn drop(&mut self) {
        let r = T::regs();

        // Stop and disable timer
        r.cmd().write(|w| w.set_stop(true));
        r.en().write(|w| w.set_en(false));
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Enable the timer clock in CMU.
fn enable_timer_clock<T: Instance>() {
    pac::CMU.clken0().modify(|w| {
        match T::index() {
            0 => w.set_timer0(true),
            1 => w.set_timer1(true),
            2 => w.set_timer2(true),
            3 => w.set_timer3(true),
            4 => w.set_timer4(true),
            _ => {}
        }
    });
}

/// Configure GPIO pins for PWM output.
fn configure_pwm_pins<T: Instance>(
    ch0: &Option<Peri<'_, AnyPin>>,
    ch1: &Option<Peri<'_, AnyPin>>,
    ch2: &Option<Peri<'_, AnyPin>>,
) {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    // Configure pins as push-pull outputs
    if let Some(pin) = ch0 {
        pin.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
    }
    if let Some(pin) = ch1 {
        pin.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
    }
    if let Some(pin) = ch2 {
        pin.mode_w(pac::gpio::vals::PortMode::PUSHPULL);
    }

    // Configure routing based on timer instance
    match T::index() {
        0 => {
            if let Some(pin) = ch0 {
                gpio.timer0_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer0_routeen().modify(|w| w.set_cc0pen(true));
            }
            if let Some(pin) = ch1 {
                gpio.timer0_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer0_routeen().modify(|w| w.set_cc1pen(true));
            }
            if let Some(pin) = ch2 {
                gpio.timer0_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer0_routeen().modify(|w| w.set_cc2pen(true));
            }
        }
        1 => {
            if let Some(pin) = ch0 {
                gpio.timer1_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer1_routeen().modify(|w| w.set_cc0pen(true));
            }
            if let Some(pin) = ch1 {
                gpio.timer1_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer1_routeen().modify(|w| w.set_cc1pen(true));
            }
            if let Some(pin) = ch2 {
                gpio.timer1_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer1_routeen().modify(|w| w.set_cc2pen(true));
            }
        }
        2 => {
            if let Some(pin) = ch0 {
                gpio.timer2_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer2_routeen().modify(|w| w.set_cc0pen(true));
            }
            if let Some(pin) = ch1 {
                gpio.timer2_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer2_routeen().modify(|w| w.set_cc1pen(true));
            }
            if let Some(pin) = ch2 {
                gpio.timer2_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer2_routeen().modify(|w| w.set_cc2pen(true));
            }
        }
        3 => {
            if let Some(pin) = ch0 {
                gpio.timer3_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer3_routeen().modify(|w| w.set_cc0pen(true));
            }
            if let Some(pin) = ch1 {
                gpio.timer3_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer3_routeen().modify(|w| w.set_cc1pen(true));
            }
            if let Some(pin) = ch2 {
                gpio.timer3_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer3_routeen().modify(|w| w.set_cc2pen(true));
            }
        }
        4 => {
            if let Some(pin) = ch0 {
                gpio.timer4_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer4_routeen().modify(|w| w.set_cc0pen(true));
            }
            if let Some(pin) = ch1 {
                gpio.timer4_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer4_routeen().modify(|w| w.set_cc1pen(true));
            }
            if let Some(pin) = ch2 {
                gpio.timer4_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
                gpio.timer4_routeen().modify(|w| w.set_cc2pen(true));
            }
        }
        _ => {}
    }
}

/// Configure a GPIO pin for input capture.
fn configure_capture_pin<T: Instance>(pin: &Peri<'_, AnyPin>, channel: Channel) {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    // Configure pin as input
    pin.mode_w(pac::gpio::vals::PortMode::INPUT);

    // Configure routing based on timer instance and channel
    match T::index() {
        0 => match channel {
            Channel::Ch0 => {
                gpio.timer0_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch1 => {
                gpio.timer0_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch2 => {
                gpio.timer0_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
        },
        1 => match channel {
            Channel::Ch0 => {
                gpio.timer1_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch1 => {
                gpio.timer1_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch2 => {
                gpio.timer1_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
        },
        2 => match channel {
            Channel::Ch0 => {
                gpio.timer2_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch1 => {
                gpio.timer2_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch2 => {
                gpio.timer2_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
        },
        3 => match channel {
            Channel::Ch0 => {
                gpio.timer3_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch1 => {
                gpio.timer3_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch2 => {
                gpio.timer3_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
        },
        4 => match channel {
            Channel::Ch0 => {
                gpio.timer4_cc0route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch1 => {
                gpio.timer4_cc1route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
            Channel::Ch2 => {
                gpio.timer4_cc2route().write(|w| {
                    w.set_port(pin.pin_port() / 16);
                    w.set_pin(pin.pin_port() % 16);
                });
            }
        },
        _ => {}
    }
}

/// Deconfigure GPIO pins when driver is dropped.
fn deconfigure_pins<T: Instance>() {
    let gpio = unsafe { pac::gpio::Gpio::from_ptr(GPIO.as_ptr()) };

    match T::index() {
        0 => {
            gpio.timer0_routeen().write(|w| {
                w.set_cc0pen(false);
                w.set_cc1pen(false);
                w.set_cc2pen(false);
            });
        }
        1 => {
            gpio.timer1_routeen().write(|w| {
                w.set_cc0pen(false);
                w.set_cc1pen(false);
                w.set_cc2pen(false);
            });
        }
        2 => {
            gpio.timer2_routeen().write(|w| {
                w.set_cc0pen(false);
                w.set_cc1pen(false);
                w.set_cc2pen(false);
            });
        }
        3 => {
            gpio.timer3_routeen().write(|w| {
                w.set_cc0pen(false);
                w.set_cc1pen(false);
                w.set_cc2pen(false);
            });
        }
        4 => {
            gpio.timer4_routeen().write(|w| {
                w.set_cc0pen(false);
                w.set_cc1pen(false);
                w.set_cc2pen(false);
            });
        }
        _ => {}
    }
}

// ============================================================================
// Instance trait
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::timer0::Timer0;
    fn state() -> &'static State;
    fn index() -> u8;
}

/// Timer peripheral instance trait.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {
    /// Interrupt for this peripheral.
    type Interrupt: interrupt::typelevel::Interrupt;
}

// ============================================================================
// embedded-hal PWM trait implementations
// ============================================================================

impl<'d, T: Instance> embedded_hal::pwm::ErrorType for SimplePwm<'d, T> {
    type Error = core::convert::Infallible;
}

impl<'d, T: Instance> embedded_hal::pwm::SetDutyCycle for SimplePwm<'d, T> {
    fn max_duty_cycle(&self) -> u16 {
        self.max_duty()
    }

    fn set_duty_cycle(&mut self, duty: u16) -> Result<(), Self::Error> {
        self.set_duty(Channel::Ch0, duty);
        Ok(())
    }
}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for Timer peripherals.
#[macro_export]
macro_rules! impl_timer {
    ($type:ident, $pac_type:ident, $index:expr) => {
        impl $crate::timer::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::timer0::Timer0 {
                unsafe { $crate::pac::timer0::Timer0::from_ptr($crate::pac::$pac_type.as_ptr()) }
            }
            fn state() -> &'static $crate::timer::State {
                static STATE: $crate::timer::State = $crate::timer::State::new();
                &STATE
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::timer::Instance for $crate::peripherals::$type {
            type Interrupt = $crate::interrupt::typelevel::$type;
        }
    };
}
