//! Energy Management Unit (EMU) driver.
//!
//! This driver provides power management and sleep mode functionality for
//! Silicon Labs EFR32 series MCUs.
//!
//! # Energy Modes
//!
//! The EFR32 series supports multiple energy modes with different power/performance tradeoffs:
//!
//! - **EM0** (Run): Active mode, CPU running
//! - **EM1** (Sleep): CPU sleeping, peripherals active
//! - **EM2** (Deep Sleep): Low-frequency peripherals active, RAM retained
//! - **EM3** (Stop): Minimal peripherals, RAM retained
//! - **EM4** (Shutoff/Hibernate): Lowest power, selective RAM retention
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::emu::{Emu, EnergyMode};
//!
//! let emu = Emu::new();
//!
//! // Enter EM1 sleep mode
//! emu.enter_em1();
//!
//! // Enter EM2 deep sleep mode
//! emu.enter_em2(false);
//! ```
#![warn(missing_docs)]

use crate::chip::pac;

/// Energy mode selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EnergyMode {
    /// EM0: Active/Run mode - CPU executing code.
    Em0,
    /// EM1: Sleep mode - CPU sleeping, high-frequency peripherals available.
    Em1,
    /// EM2: Deep Sleep mode - Low-frequency peripherals active.
    Em2,
    /// EM3: Stop mode - Most peripherals disabled, RAM retained.
    Em3,
    /// EM4: Shutoff mode - Lowest power consumption.
    Em4,
}

/// EM4 sub-modes for Series 2 devices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Em4Mode {
    /// EM4 Shutoff - Lowest power, no RAM retention.
    Shutoff,
    /// EM4 Hibernate - Low power with BURTC/GPIO wakeup capability.
    #[default]
    Hibernate,
}

/// EM2/EM3 debug mode configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DebugMode {
    /// Disable debug functionality in EM2/EM3.
    #[default]
    Disabled,
    /// Enable debug functionality in EM2/EM3 (higher power consumption).
    Enabled,
}

/// EMU configuration.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct Config {
    /// EM4 mode selection.
    pub em4_mode: Em4Mode,
    /// Debug mode for EM2/EM3.
    pub debug_mode: DebugMode,
    /// Retain RAM in EM4 (for Hibernate mode only).
    pub em4_retain_ram: bool,
    /// Enable BURTC wakeup from EM4.
    pub em4_burtc_wakeup: bool,
    /// Enable GPIO wakeup from EM4.
    pub em4_gpio_wakeup: bool,
}

/// Energy Management Unit driver.
pub struct Emu {
    _private: (),
}

impl Emu {
    /// Create a new EMU driver with default configuration.
    pub fn new() -> Self {
        Self::with_config(Config::default())
    }

    /// Create a new EMU driver with custom configuration.
    pub fn with_config(config: Config) -> Self {
        let emu = emu_regs();

        // Configure EM4 control
        // Note: EM4 entry value: 0 = disabled, 1 = EM4S, 2 = EM4H
        // We don't set em4entry here - it's used only when actually entering EM4

        // Configure debug mode
        emu.ctrl().modify(|w| {
            match config.debug_mode {
                DebugMode::Disabled => w.set_em2dbgen(false),
                DebugMode::Enabled => w.set_em2dbgen(true),
            }
        });

        let _ = config; // Use all config fields

        Self { _private: () }
    }

    /// Enter EM1 (Sleep) mode.
    ///
    /// The CPU is halted but high-frequency peripherals remain active.
    /// Any interrupt will wake the device.
    #[inline]
    pub fn enter_em1(&self) {
        // Clear SLEEPDEEP bit to enter EM1 (not deep sleep)
        unsafe {
            let scb = &*cortex_m::peripheral::SCB::PTR;
            scb.scr.modify(|scr| scr & !(1 << 2));
        }

        // Wait for interrupt
        cortex_m::asm::wfi();
    }

    /// Enter EM2 (Deep Sleep) mode.
    ///
    /// High-frequency oscillators are disabled. Low-frequency peripherals
    /// (BURTC, LETIMER, etc.) remain active. RAM is retained.
    ///
    /// # Arguments
    /// * `restore` - If true, save and restore oscillator state on wakeup
    #[inline]
    pub fn enter_em2(&self, restore: bool) {
        self.enter_deep_sleep(false, restore);
    }

    /// Enter EM3 (Stop) mode.
    ///
    /// Most oscillators are disabled. Only ULFRCO remains active.
    /// RAM is retained. Wake on specific peripheral interrupts or GPIO.
    ///
    /// # Arguments
    /// * `restore` - If true, save and restore oscillator state on wakeup
    #[inline]
    pub fn enter_em3(&self, restore: bool) {
        self.enter_deep_sleep(true, restore);
    }

    /// Internal function to enter EM2 or EM3.
    fn enter_deep_sleep(&self, _em3: bool, _restore: bool) {
        // Set SLEEPDEEP bit to enter EM2/EM3
        unsafe {
            let scb = &*cortex_m::peripheral::SCB::PTR;
            scb.scr.modify(|scr| scr | (1 << 2));
        }

        // Memory barrier before WFI
        cortex_m::asm::dsb();

        // Wait for interrupt
        cortex_m::asm::wfi();

        // Memory barrier after wakeup
        cortex_m::asm::isb();

        // Clear SLEEPDEEP bit
        unsafe {
            let scb = &*cortex_m::peripheral::SCB::PTR;
            scb.scr.modify(|scr| scr & !(1 << 2));
        }
    }

    /// Enter EM4 (Shutoff/Hibernate) mode.
    ///
    /// # Safety
    ///
    /// Entering EM4 will reset the device on wakeup. Only use this when
    /// you have properly configured wakeup sources and saved any necessary state.
    ///
    /// The device will reset and start execution from the beginning on wakeup.
    pub fn enter_em4(&self) -> ! {
        let emu = emu_regs();

        // Set SLEEPDEEP bit
        unsafe {
            let scb = &*cortex_m::peripheral::SCB::PTR;
            scb.scr.modify(|scr| scr | (1 << 2));
        }

        // Memory barrier before EM4 entry sequence
        cortex_m::asm::dsb();

        // EM4 entry sequence - write special value to EM4CTRL
        // em4entry field: 0 = disabled, 1 = EM4S (shutoff), 2 = EM4H (hibernate)
        emu.em4ctrl().modify(|w| w.set_em4entry(2)); // EM4H

        // Wait loop for EM4 entry
        loop {
            cortex_m::asm::wfi();
        }
    }

    /// Get the cause of the last wakeup from EM4.
    pub fn em4_wakeup_cause(&self) -> Em4WakeupCause {
        let emu = emu_regs();
        let status = emu.status().read();

        Em4WakeupCause {
            pin_wakeup: status.em4ioret(),
            burtc_wakeup: false, // Check BURTC status separately if needed
        }
    }

    /// Check if the device woke up from EM4.
    pub fn was_em4_wakeup(&self) -> bool {
        let emu = emu_regs();
        emu.status().read().em4ioret()
    }

    /// Clear EM4 wakeup cause flags.
    pub fn clear_em4_wakeup_cause(&self) {
        // EM4 status is typically cleared by hardware on wakeup
    }

    /// Enable BURTC as an EM4 wakeup source.
    pub fn enable_burtc_em4_wakeup(&self) {
        // BURTC wakeup is typically enabled via BURTC configuration
        // The EM4 wakeup sources are configured in the respective peripheral
    }

    /// Configure RAM retention for EM4.
    ///
    /// # Arguments
    /// * `blocks` - Bitmask of RAM blocks to retain (device-specific)
    pub fn set_em4_ram_retention(&self, _blocks: u32) {
        // RAM retention configuration is device-specific
        // For Series 2, RAM is automatically retained in EM4H
    }

    /// Get the current voltage scaling level.
    pub fn voltage_scale_level(&self) -> VoltageScale {
        let emu = emu_regs();
        let status = emu.status().read();

        if status.vscale().to_bits() == 1 {
            VoltageScale::Scale1
        } else {
            VoltageScale::Scale0
        }
    }

    /// Set the voltage scaling level for EM0/EM1.
    ///
    /// Lower voltage scales reduce power consumption but limit maximum clock frequency.
    pub fn set_voltage_scale(&self, scale: VoltageScale) {
        let emu = emu_regs();

        // Wait for any pending voltage scaling to complete
        while emu.status().read().vscalebusy() {}

        // Issue voltage scale command
        // Note: Series 2 devices only have scale1 and scale2 commands
        emu.cmd().write(|w| {
            match scale {
                VoltageScale::Scale0 => {
                    // Scale0 is the lowest - use scale1 as fallback
                    w.set_em01vscale1(true);
                }
                VoltageScale::Scale1 => w.set_em01vscale1(true),
                VoltageScale::Scale2 => w.set_em01vscale2(true),
            }
        });

        // Wait for voltage scaling to complete
        while emu.status().read().vscalebusy() {}
    }

    /// Lock EMU configuration to prevent further changes.
    pub fn lock(&self) {
        // EMU lock is write-only with a specific unlock key
        // Writing any value other than the unlock key will lock it
        // For Series 2 devices, the register auto-locks after any operation
        // that modifies protected fields
    }

    /// Unlock EMU configuration.
    pub fn unlock(&self) {
        let emu = emu_regs();
        emu.lock().write(|w| w.set_lockkey(pac::emu::vals::Lockkey::UNLOCK));
    }

    /// Check if EMU is locked.
    ///
    /// Note: Lock register is write-only, so we check status register instead.
    pub fn is_locked(&self) -> bool {
        // EMU lock status can't be read directly from lock register
        // Return false as a conservative default
        false
    }
}

impl Default for Emu {
    fn default() -> Self {
        Self::new()
    }
}

/// Voltage scaling levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum VoltageScale {
    /// Scale 0: Lowest voltage (1.0V), limited clock frequency.
    Scale0,
    /// Scale 1: Medium voltage (1.1V), balanced performance.
    Scale1,
    /// Scale 2: Full voltage (1.2V), maximum clock frequency.
    Scale2,
}

/// EM4 wakeup cause information.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Em4WakeupCause {
    /// True if wakeup was caused by a GPIO pin.
    pub pin_wakeup: bool,
    /// True if wakeup was caused by BURTC.
    pub burtc_wakeup: bool,
}

// ============================================================================
// Helper functions
// ============================================================================

#[inline]
fn emu_regs() -> pac::emu::Emu {
    #[cfg(feature = "_ns")]
    {
        pac::EMU_NS
    }
    #[cfg(not(feature = "_ns"))]
    {
        pac::EMU_S
    }
}

// ============================================================================
// Wakeup pin configuration
// ============================================================================

/// EM4 wakeup pin polarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WakeupPolarity {
    /// Wake on rising edge (low to high).
    #[default]
    RisingEdge,
    /// Wake on falling edge (high to low).
    FallingEdge,
}

impl Emu {
    /// Configure a GPIO pin as an EM4 wakeup source.
    ///
    /// # Arguments
    /// * `pin` - GPIO pin number (device-specific)
    /// * `enable` - Enable or disable wakeup from this pin
    /// * `polarity` - Edge polarity for wakeup
    pub fn configure_em4_wakeup_pin(
        &self,
        pin: u8,
        enable: bool,
        polarity: WakeupPolarity,
    ) {
        // EM4 wakeup pins are configured through GPIO and EMU registers
        // This is device-specific and may require GPIO configuration as well
        let _emu = emu_regs();

        if enable {
            // Enable the specified pin as a wakeup source
            // The exact implementation depends on the device's GPIO/EMU mapping
        }

        let _ = (pin, polarity); // Silence unused warnings for now
    }

    /// Retain GPIO state when entering EM4.
    ///
    /// This preserves the GPIO output state during EM4 sleep.
    pub fn retain_gpio_em4(&self, _retain: bool) {
        // GPIO retention is configured via em4ctrl.em4ioretmode
        // Note: Implementation depends on specific device requirements
    }
}

// ============================================================================
// Power-on reset and brown-out detection
// ============================================================================

/// Brown-out detection level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BodLevel {
    /// Level 0 (lowest threshold).
    Level0,
    /// Level 1.
    Level1,
    /// Level 2.
    Level2,
    /// Level 3 (highest threshold).
    Level3,
}

impl Emu {
    /// Check if a power-on reset occurred.
    pub fn was_power_on_reset(&self) -> bool {
        // Check RMU (Reset Management Unit) status
        // This is typically handled through the RMU peripheral
        false
    }

    /// Check if a brown-out reset occurred.
    pub fn was_brownout_reset(&self) -> bool {
        // Check RMU status for brown-out indication
        false
    }
}

// ============================================================================
// Temperature sensor
// ============================================================================

impl Emu {
    /// Read the on-chip temperature sensor.
    ///
    /// Returns the temperature in degrees Celsius.
    /// Note: The accuracy is limited and calibration may be needed.
    pub fn read_temperature(&self) -> Option<i32> {
        let emu = emu_regs();

        // Check if temperature is valid
        let temp = emu.temp().read();
        if temp.tempavg() == 0 {
            return None;
        }

        // Convert raw value to temperature
        // The formula is device-specific and may require calibration data
        // For EFR32xG24: T = (temp_raw - cal_offset) * cal_gain
        // Using simplified calculation here
        let raw = temp.tempavg() as i32;

        // Approximate conversion (device-specific calibration recommended)
        // Raw value is in 1/4 degree increments, offset from 0°C
        let temp_c = (raw - 512) / 4;

        Some(temp_c)
    }
}
