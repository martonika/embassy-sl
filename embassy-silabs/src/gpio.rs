//! General purpose input/output (GPIO) driver.
#![macro_use]

use core::convert::Infallible;
use core::hint::unreachable_unchecked;

use embassy_hal_internal::{Peri, PeripheralType, impl_peripheral};

use crate::chip::pac;
use pac::GPIO_NS as GPIO;
use pac::common::*;
use pac::gpio::ns as gpio;
use pac::gpio::regs as gpio_regs;
use pac::gpio::vals;
use pac::gpio_p as port;

#[cfg(feature = "defmt")]
use defmt::error;

/// GPIO input driver.
pub struct Input<'d> {
    pub(crate) pin: Flex<'d>,
}

impl<'d> Input<'d> {
    /// Create GPIO input driver for a [Pin] with the provided [Pull] configuration.
    #[inline]
    pub fn new(pin: Peri<'d, impl Pin>, mode: vals::PortMode) -> Self {
        let mut pin = Flex::new(pin);
        pin.set_as_input(mode);

        Self { pin }
    }

    /// Get whether the pin input level is high.
    #[inline]
    pub fn is_high(&self) -> bool {
        self.pin.is_high()
    }

    /// Get whether the pin input level is low.
    #[inline]
    pub fn is_low(&self) -> bool {
        self.pin.is_low()
    }

    /// Get the pin input level.
    #[inline]
    pub fn get_level(&self) -> Level {
        self.pin.get_level()
    }
}

/// GPIO output driver.
pub struct Output<'d> {
    pub(crate) pin: Flex<'d>,
}

impl<'d> Output<'d> {
    /// Create GPIO output driver for a [Pin] with the provided [Level] and [OutputDriver] configuration.
    #[inline]
    pub fn new(pin: Peri<'d, impl Pin>, initial_output: Level) -> Self {
        let mut pin = Flex::new(pin);
        pin.set_as_output();
        match initial_output {
            Level::High => pin.set_high(),
            Level::Low => pin.set_low(),
        }
        Self { pin }
    }

    /// Set the output as high.
    #[inline]
    pub fn set_high(&mut self) {
        self.pin.set_high()
    }

    /// Set the output as low.
    #[inline]
    pub fn set_low(&mut self) {
        self.pin.set_low()
    }

    /// Toggle the output level.
    #[inline]
    pub fn toggle(&mut self) {
        self.pin.toggle()
    }

    /// Set the output level.
    #[inline]
    pub fn set_level(&mut self, level: Level) {
        self.pin.set_level(level)
    }

    /// Get whether the output level is set to high.
    #[inline]
    pub fn is_set_high(&self) -> bool {
        self.pin.is_set_high()
    }

    /// Get whether the output level is set to low.
    #[inline]
    pub fn is_set_low(&self) -> bool {
        self.pin.is_set_low()
    }

    /// Get the current output level.
    #[inline]
    pub fn get_output_level(&self) -> Level {
        self.pin.get_output_level()
    }
}

/// Digital input or output level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Level {
    /// Logical low.
    Low,
    /// Logical high.
    High,
}

impl From<bool> for Level {
    fn from(val: bool) -> Self {
        match val {
            true => Self::High,
            false => Self::Low,
        }
    }
}

impl From<Level> for bool {
    fn from(level: Level) -> bool {
        match level {
            Level::Low => false,
            Level::High => true,
        }
    }
}

/// GPIO flexible pin.
///
/// This pin can either be a disconnected, input, or output pin, or both. The level register bit will remain
/// set while not in output mode, so the pin's level will be 'remembered' when it is not in output
/// mode.
pub struct Flex<'d> {
    pub(crate) pin: Peri<'d, AnyPin>,
}

impl<'d> Flex<'d> {
    /// Wrap the pin in a `Flex`.
    ///
    /// The pin remains disconnected. The initial output level is unspecified, but can be changed
    /// before the pin is put into output mode.
    #[inline]
    pub fn new(pin: Peri<'d, impl Pin>) -> Self {
        // Pin will be in disconnected state.
        Self { pin: pin.into() }
    }

    /// Put the pin into input mode.
    #[inline]
    pub fn set_as_input(&mut self, mode: vals::PortMode) {
        match mode {
            vals::PortMode::INPUT | vals::PortMode::INPUTPULL | vals::PortMode::INPUTPULLFILTER => {
                self.pin.mode_w(mode)
            }
            #[cfg(feature = "defmt")]
            _ => error!("Pin setting is not for a valid input mode!"),
            #[cfg(not(feature = "defmt"))]
            _ => panic!("Pin setting is not for a valid input mode!"),
        }
    }

    /// Put the pin into output mode.
    ///
    /// Technically this is an input+output (unless DINDIS), but let's use it for the basic push-pull mode
    #[inline]
    pub fn set_as_output(&mut self) {
        self.pin.mode_w(vals::PortMode::PUSHPULL)
    }

    /// Put the pin into input + output mode.
    ///
    /// Settings allow for both Open Source (wired-OR) and Open Drain (wired-AND) modes.
    #[inline]
    pub fn set_as_input_output(&mut self, mode: vals::PortMode) {
        match mode {
            #[cfg(feature = "defmt")]
            vals::PortMode::DISABLED
            | vals::PortMode::INPUT
            | vals::PortMode::INPUTPULL
            | vals::PortMode::INPUTPULLFILTER
            | vals::PortMode::PUSHPULL => error!("Pin setting is not for a valid input mode!"),
            #[cfg(not(feature = "defmt"))]
            vals::PortMode::DISABLED
            | vals::PortMode::INPUT
            | vals::PortMode::INPUTPULL
            | vals::PortMode::INPUTPULLFILTER
            | vals::PortMode::PUSHPULL => panic!("Pin setting is not for a valid input mode!"),
            _ => self.pin.mode_w(mode),
        }
    }

    /// Put the pin into disconnected mode.
    #[inline]
    pub fn set_as_disabled(&mut self) {
        self.pin.mode_w(vals::PortMode::DISABLED);
    }

    /// Get whether the pin input level is high.
    #[inline]
    pub fn is_high(&self) -> bool {
        self.pin.din()
    }

    /// Get whether the pin input level is low.
    #[inline]
    pub fn is_low(&self) -> bool {
        !self.is_high()
    }

    /// Get the pin input level.
    #[inline]
    pub fn get_level(&self) -> Level {
        self.is_high().into()
    }

    /// Set the output as high.
    #[inline]
    pub fn set_high(&mut self) {
        self.pin.set_high()
    }

    /// Set the output as low.
    #[inline]
    pub fn set_low(&mut self) {
        self.pin.set_low()
    }

    /// Toggle the output level.
    #[inline]
    pub fn toggle(&mut self) {
        if self.is_set_low() {
            self.set_high()
        } else {
            self.set_low()
        }
    }

    /// Set the output level.
    #[inline]
    pub fn set_level(&mut self, level: Level) {
        match level {
            Level::Low => self.pin.set_low(),
            Level::High => self.pin.set_high(),
        }
    }

    /// Get whether the output level is set to high.
    #[inline]
    pub fn is_set_high(&self) -> bool {
        self.pin.dout_r()
    }

    /// Get whether the output level is set to low.
    #[inline]
    pub fn is_set_low(&self) -> bool {
        !self.is_set_high()
    }

    /// Get the current output level.
    #[inline]
    pub fn get_output_level(&self) -> Level {
        self.is_set_high().into()
    }
}

impl<'d> Drop for Flex<'d> {
    fn drop(&mut self) {
        self.set_as_disabled();
    }
}

// =====================

pub(crate) trait SealedPin {
    fn pin_port(&self) -> u8;

    #[inline]
    fn _pin(&self) -> u8 {
        self.pin_port() % 16
    }

    #[inline]
    fn port(&self) -> port::GpioPort {
        match self.pin_port() / 16 {
            0 => gpio::port_a(GPIO),
            1 => gpio::port_b(GPIO),
            2 => gpio::port_c(GPIO),
            3 => gpio::port_d(GPIO),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    fn ctrl(&self) -> Reg<gpio_regs::Ctrl, RW> {
        self.port().ctrl()
    }

    #[inline]
    fn din(&self) -> bool {
        self.port().din().read().din() & (1 << self._pin()) != 0
    }

    #[inline]
    fn dout_r(&self) -> bool {
        self.port().dout().read().dout() & (1 << self._pin()) != 0
    }

    #[inline]
    fn mode_reg(&self) -> Reg<gpio_regs::Mode, RW> {
        match self._pin() {
            0..=7 => self.port().model(),
            8 | 9 => self.port().modeh(),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    fn mode_r(&self) -> vals::PortMode {
        match self._pin() {
            0 => self.mode_reg().read().mode0(),
            1 => self.mode_reg().read().mode1(),
            2 => self.mode_reg().read().mode2(),
            3 => self.mode_reg().read().mode3(),
            4 => self.mode_reg().read().mode4(),
            5 => self.mode_reg().read().mode5(),
            6 => self.mode_reg().read().mode6(),
            7 => self.mode_reg().read().mode7(),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    fn mode_w(&self, val: vals::PortMode) {
        match self._pin() {
            0 => self.mode_reg().modify(|w| w.set_mode0(val)),
            1 => self.mode_reg().modify(|w| w.set_mode1(val)),
            2 => self.mode_reg().modify(|w| w.set_mode2(val)),
            3 => self.mode_reg().modify(|w| w.set_mode3(val)),
            4 => self.mode_reg().modify(|w| w.set_mode4(val)),
            5 => self.mode_reg().modify(|w| w.set_mode5(val)),
            6 => self.mode_reg().modify(|w| w.set_mode6(val)),
            7 => self.mode_reg().modify(|w| w.set_mode7(val)),
            _ => unsafe { unreachable_unchecked() },
        }
    }

    /// Set the output as high.
    #[inline]
    fn set_high(&self) {
        let mut val = self.port().dout().read().dout();
        val |= 1 << self._pin();
        self.port().dout().modify(|w| w.set_dout(val));
    }

    /// Set the output as low.
    #[inline]
    fn set_low(&self) {
        let mut val = self.port().dout().read().dout();
        val &= !(1 << self._pin());
        self.port().dout().modify(|w| w.set_dout(val));
    }
}

/// Interface for a Pin that can be configured by an [Input] or [Output] driver, or converted to an [AnyPin].
#[allow(private_bounds)]
pub trait Pin: PeripheralType + Into<AnyPin> + SealedPin + Sized + 'static {
    /// Number of the pin within the port (0..15)
    #[inline]
    fn pin(&self) -> u8 {
        self._pin()
    }

    /// Port of the pin
    #[inline]
    fn port(&self) -> port::GpioPort {
        match self.pin_port() / 16 {
            0 => gpio::port_a(GPIO),
            1 => gpio::port_b(GPIO),
            2 => gpio::port_c(GPIO),
            3 => gpio::port_d(GPIO),
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

/// Type-erased GPIO pin
pub struct AnyPin {
    pub(crate) pin_port: u8,
}

impl AnyPin {
    /// Create an [AnyPin] for a specific pin.
    ///
    /// # Safety
    /// - `pin_port` should not in use by another driver.
    #[inline]
    pub unsafe fn steal(pin_port: u8) -> Peri<'static, Self> {
        unsafe { Peri::new_unchecked(Self { pin_port }) }
    }
}

impl_peripheral!(AnyPin);
impl Pin for AnyPin {}
impl SealedPin for AnyPin {
    #[inline]
    fn pin_port(&self) -> u8 {
        self.pin_port
    }
}

// =====================

macro_rules! impl_pin {
    ($type:ident, $port_num:expr, $pin_num:expr) => {
        impl crate::gpio::Pin for peripherals::$type {}
        impl crate::gpio::SealedPin for peripherals::$type {
            #[inline]
            fn pin_port(&self) -> u8 {
                $port_num * 16 + $pin_num
            }
        }

        impl From<peripherals::$type> for crate::gpio::AnyPin {
            fn from(_val: peripherals::$type) -> Self {
                Self {
                    pin_port: $port_num * 16 + $pin_num,
                }
            }
        }
    };
}

// ====================

impl<'d> embedded_hal::digital::ErrorType for Input<'d> {
    type Error = Infallible;
}

impl<'d> embedded_hal::digital::InputPin for Input<'d> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_low())
    }
}

impl<'d> embedded_hal::digital::ErrorType for Output<'d> {
    type Error = Infallible;
}

impl<'d> embedded_hal::digital::OutputPin for Output<'d> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
}

impl<'d> embedded_hal::digital::StatefulOutputPin for Output<'d> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_high())
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_low())
    }
}

impl<'d> embedded_hal::digital::ErrorType for Flex<'d> {
    type Error = Infallible;
}

/// Implement [`InputPin`] for [`Flex`];
///
/// If the pin is not in input mode the result is unspecified.
impl<'d> embedded_hal::digital::InputPin for Flex<'d> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_low())
    }
}

impl<'d> embedded_hal::digital::OutputPin for Flex<'d> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.set_high();
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.set_low();
        Ok(())
    }
}

impl<'d> embedded_hal::digital::StatefulOutputPin for Flex<'d> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_high())
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_low())
    }
}
