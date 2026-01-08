//! Watchdog (WDOG) peripheral driver.
//!
//! The watchdog timer is used to detect system malfunctions and automatically reset
//! the microcontroller if the software fails to regularly "feed" the watchdog.
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::wdog::{Wdog, Config, Timeout};
//!
//! let config = Config::default()
//!     .with_timeout(Timeout::Seconds2);
//!
//! let mut wdog = Wdog::new(p.WDOG0, config);
//!
//! // Feed the watchdog periodically to prevent reset
//! loop {
//!     // Do some work...
//!     wdog.feed();
//!     Timer::after_millis(500).await;
//! }
//! ```
#![warn(missing_docs)]

use core::marker::PhantomData;

use embassy_hal_internal::{Peri, PeripheralType};

use crate::chip::pac;

/// Watchdog timeout period.
///
/// The timeout is based on the watchdog clock (typically ULFRCO at 1kHz or LFRCO at 32.768kHz).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum Timeout {
    /// 9 clock cycles (~9ms @ 1kHz, ~0.27ms @ 32.768kHz)
    Cycles9 = 0,
    /// 17 clock cycles (~17ms @ 1kHz, ~0.52ms @ 32.768kHz)
    Cycles17 = 1,
    /// 33 clock cycles (~33ms @ 1kHz, ~1ms @ 32.768kHz)
    Cycles33 = 2,
    /// 65 clock cycles (~65ms @ 1kHz, ~2ms @ 32.768kHz)
    Cycles65 = 3,
    /// 129 clock cycles (~129ms @ 1kHz, ~4ms @ 32.768kHz)
    Cycles129 = 4,
    /// 257 clock cycles (~257ms @ 1kHz, ~8ms @ 32.768kHz)
    Cycles257 = 5,
    /// 513 clock cycles (~513ms @ 1kHz, ~16ms @ 32.768kHz)
    Cycles513 = 6,
    /// 1025 clock cycles (~1s @ 1kHz, ~31ms @ 32.768kHz)
    Cycles1k = 7,
    /// 2049 clock cycles (~2s @ 1kHz, ~63ms @ 32.768kHz)
    Cycles2k = 8,
    /// 4097 clock cycles (~4s @ 1kHz, ~125ms @ 32.768kHz)
    Cycles4k = 9,
    /// 8193 clock cycles (~8s @ 1kHz, ~250ms @ 32.768kHz)
    Cycles8k = 10,
    /// 16385 clock cycles (~16s @ 1kHz, ~500ms @ 32.768kHz)
    Cycles16k = 11,
    /// 32769 clock cycles (~33s @ 1kHz, ~1s @ 32.768kHz)
    #[default]
    Cycles32k = 12,
    /// 65537 clock cycles (~66s @ 1kHz, ~2s @ 32.768kHz)
    Cycles64k = 13,
    /// 131073 clock cycles (~131s @ 1kHz, ~4s @ 32.768kHz)
    Cycles128k = 14,
    /// 262145 clock cycles (~262s @ 1kHz, ~8s @ 32.768kHz)
    Cycles256k = 15,
}

impl Timeout {
    /// Get the number of clock cycles for this timeout.
    pub fn cycles(&self) -> u32 {
        match self {
            Timeout::Cycles9 => 9,
            Timeout::Cycles17 => 17,
            Timeout::Cycles33 => 33,
            Timeout::Cycles65 => 65,
            Timeout::Cycles129 => 129,
            Timeout::Cycles257 => 257,
            Timeout::Cycles513 => 513,
            Timeout::Cycles1k => 1025,
            Timeout::Cycles2k => 2049,
            Timeout::Cycles4k => 4097,
            Timeout::Cycles8k => 8193,
            Timeout::Cycles16k => 16385,
            Timeout::Cycles32k => 32769,
            Timeout::Cycles64k => 65537,
            Timeout::Cycles128k => 131073,
            Timeout::Cycles256k => 262145,
        }
    }

    /// Get the approximate timeout in milliseconds assuming ULFRCO (1kHz).
    pub fn timeout_ms_ulfrco(&self) -> u32 {
        self.cycles()
    }

    /// Get the approximate timeout in milliseconds assuming LFRCO (32.768kHz).
    pub fn timeout_ms_lfrco(&self) -> u32 {
        (self.cycles() * 1000) / 32768
    }
}

/// Watchdog warning interrupt threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum Warning {
    /// Warning disabled
    #[default]
    Disabled = 0,
    /// Warning at 25% of timeout
    Percent25 = 1,
    /// Warning at 50% of timeout
    Percent50 = 2,
    /// Warning at 75% of timeout
    Percent75 = 3,
}

/// Watchdog configuration.
#[derive(Clone)]
#[non_exhaustive]
pub struct Config {
    /// Timeout period.
    pub timeout: Timeout,
    /// Continue running during debug halt.
    pub debug_run: bool,
    /// Continue running in EM1 energy mode.
    pub em1_run: bool,
    /// Continue running in EM2 energy mode.
    pub em2_run: bool,
    /// Continue running in EM3 energy mode.
    pub em3_run: bool,
    /// Block entry to EM4 energy mode.
    pub em4_block: bool,
    /// Warning interrupt threshold.
    pub warning: Warning,
    /// Lock configuration after initialization.
    pub lock: bool,
    /// Disable watchdog reset (only generate interrupts).
    pub reset_disable: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Timeout::Cycles32k,
            debug_run: false,
            em1_run: false,
            em2_run: false,
            em3_run: false,
            em4_block: false,
            warning: Warning::Disabled,
            lock: false,
            reset_disable: false,
        }
    }
}

impl Config {
    /// Set the timeout period.
    pub fn with_timeout(mut self, timeout: Timeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable running during debug.
    pub fn with_debug_run(mut self, enable: bool) -> Self {
        self.debug_run = enable;
        self
    }

    /// Enable running in EM1 mode.
    pub fn with_em1_run(mut self, enable: bool) -> Self {
        self.em1_run = enable;
        self
    }

    /// Enable running in EM2 mode.
    pub fn with_em2_run(mut self, enable: bool) -> Self {
        self.em2_run = enable;
        self
    }

    /// Enable running in EM3 mode.
    pub fn with_em3_run(mut self, enable: bool) -> Self {
        self.em3_run = enable;
        self
    }

    /// Block EM4 mode entry.
    pub fn with_em4_block(mut self, block: bool) -> Self {
        self.em4_block = block;
        self
    }

    /// Set warning interrupt threshold.
    pub fn with_warning(mut self, warning: Warning) -> Self {
        self.warning = warning;
        self
    }

    /// Lock configuration after initialization.
    pub fn with_lock(mut self, lock: bool) -> Self {
        self.lock = lock;
        self
    }

    /// Disable reset generation (interrupt only).
    pub fn with_reset_disable(mut self, disable: bool) -> Self {
        self.reset_disable = disable;
        self
    }
}

// ============================================================================
// WDOG Driver
// ============================================================================

/// Watchdog driver.
pub struct Wdog<'d, T: Instance> {
    _phantom: PhantomData<&'d T>,
}

impl<'d, T: Instance> Wdog<'d, T> {
    /// Create and start a new watchdog driver.
    pub fn new(_wdog: Peri<'d, T>, config: Config) -> Self {
        let r = T::regs();

        // Enable WDOG clock
        T::enable_clock();

        // Disable WDOG before configuration
        r.en().write(|w| w.set_en(false));

        // Wait for synchronization
        while r.syncbusy().read().0 != 0 {}

        // Configure WDOG
        r.cfg().write(|w| {
            w.set_debugrun(if config.debug_run {
                pac::wdog0::vals::Debugrun::ENABLE
            } else {
                pac::wdog0::vals::Debugrun::DISABLE
            });
            w.set_em1run(if config.em1_run {
                pac::wdog0::vals::Em1run::ENABLE
            } else {
                pac::wdog0::vals::Em1run::DISABLE
            });
            w.set_em2run(if config.em2_run {
                pac::wdog0::vals::Em2run::ENABLE
            } else {
                pac::wdog0::vals::Em2run::DISABLE
            });
            w.set_em3run(if config.em3_run {
                pac::wdog0::vals::Em3run::ENABLE
            } else {
                pac::wdog0::vals::Em3run::DISABLE
            });
            w.set_em4block(if config.em4_block {
                pac::wdog0::vals::Em4block::ENABLE
            } else {
                pac::wdog0::vals::Em4block::DISABLE
            });
            w.set_wdogrstdis(if config.reset_disable {
                pac::wdog0::vals::Wdogrstdis::DIS
            } else {
                pac::wdog0::vals::Wdogrstdis::EN
            });
            w.set_persel(pac::wdog0::vals::Persel::from_bits(config.timeout as u8));
            w.set_warnsel(pac::wdog0::vals::Warnsel::from_bits(config.warning as u8));
        });

        // Wait for synchronization
        while r.syncbusy().read().0 != 0 {}

        // Enable WDOG
        r.en().write(|w| w.set_en(true));

        // Wait for synchronization
        while r.syncbusy().read().0 != 0 {}

        // Lock if requested
        if config.lock {
            r.lock().write(|w| w.set_lockkey(pac::wdog0::vals::Lockkey::LOCK));
        }

        Self {
            _phantom: PhantomData,
        }
    }

    /// Feed (pet/kick) the watchdog to prevent reset.
    ///
    /// This resets the watchdog counter. Call this periodically before the timeout expires.
    #[inline]
    pub fn feed(&mut self) {
        let r = T::regs();
        r.cmd().write(|w| w.set_clear(pac::wdog0::vals::Clear::CLEARED));
    }

    /// Check if the watchdog is running.
    pub fn is_running(&self) -> bool {
        T::regs().en().read().en()
    }

    /// Check if the watchdog is locked.
    pub fn is_locked(&self) -> bool {
        T::regs().status().read().lock() == pac::wdog0::vals::Lock::LOCKED
    }

    /// Enable the warning interrupt.
    pub fn enable_warning_interrupt(&mut self) {
        let r = T::regs();
        r.ien().modify(|w| w.set_warn(true));
    }

    /// Disable the warning interrupt.
    pub fn disable_warning_interrupt(&mut self) {
        let r = T::regs();
        r.ien().modify(|w| w.set_warn(false));
    }

    /// Check and clear the warning interrupt flag.
    pub fn check_warning(&mut self) -> bool {
        let r = T::regs();
        let warn = r.if_().read().warn();
        if warn {
            r.if_().write(|w| w.set_warn(true));
        }
        warn
    }

    /// Unlock the watchdog for reconfiguration (requires reset to take effect on Series 2).
    pub fn unlock(&mut self) {
        let r = T::regs();
        // On Series 2, the unlock key is a specific value
        r.lock().write(|w| w.set_lockkey(pac::wdog0::vals::Lockkey::UNLOCK));
    }
}

impl<'d, T: Instance> Drop for Wdog<'d, T> {
    fn drop(&mut self) {
        // Note: On many devices, the watchdog cannot be disabled once started
        // without a reset. This is a safety feature.
        // We attempt to disable, but it may not take effect if locked.
        let r = T::regs();

        // Try to disable (will fail if locked)
        if !self.is_locked() {
            r.en().write(|w| w.set_en(false));
        }
    }
}

// ============================================================================
// Instance trait and implementations
// ============================================================================

pub(crate) trait SealedInstance {
    fn regs() -> pac::wdog0::Wdog0;
    fn enable_clock();
    #[allow(dead_code)]
    fn index() -> u8;
}

/// WDOG peripheral instance trait.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType + 'static + Send {}

// ============================================================================
// Macro for implementing Instance trait
// ============================================================================

/// Macro to implement the Instance trait for WDOG peripherals.
#[macro_export]
macro_rules! impl_wdog {
    ($type:ident, $pac_type:ident, $index:expr) => {
        impl $crate::wdog::SealedInstance for $crate::peripherals::$type {
            fn regs() -> $crate::pac::wdog0::Wdog0 {
                unsafe { $crate::pac::wdog0::Wdog0::from_ptr($crate::pac::$pac_type.as_ptr()) }
            }
            fn enable_clock() {
                match $index {
                    0 => $crate::pac::CMU.clken0().modify(|w| w.set_wdog0(true)),
                    1 => $crate::pac::CMU.clken1().modify(|w| w.set_wdog1(true)),
                    _ => {}
                }
            }
            fn index() -> u8 {
                $index
            }
        }
        impl $crate::wdog::Instance for $crate::peripherals::$type {}
    };
}
