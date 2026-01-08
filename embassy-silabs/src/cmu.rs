//! Clock Management Unit (CMU) driver.
//!
//! This module provides clock tree configuration for Silicon Labs EFR32 series MCUs.
//! It allows enabling/disabling oscillators, selecting clock sources, and configuring
//! peripheral clocks.
//!
//! # Overview
//!
//! The EFR32 clock tree has several layers:
//! - **Oscillators**: HFXO (external crystal), HFRCO (internal RC), LFXO, LFRCO, ULFRCO
//! - **Clock branches**: SYSCLK, HCLK, PCLK, various peripheral clocks
//! - **Peripheral clocks**: Individual enables for each peripheral
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::cmu::{Cmu, HfrcoFreq, IadcClkSel, WdogClkSel};
//!
//! // Configure HFRCO to 80 MHz
//! Cmu::set_hfrco_frequency(HfrcoFreq::Freq80MHz);
//!
//! // Set IADC clock source
//! Cmu::set_iadc_clock_source(IadcClkSel::Hfrcoem23);
//!
//! // Set WDOG0 clock source to ULFRCO (1kHz)
//! Cmu::set_wdog0_clock_source(WdogClkSel::Ulfrco);
//! ```
#![warn(missing_docs)]

use crate::chip::pac;

/// HFRCO (High Frequency RC Oscillator) frequency bands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HfrcoFreq {
    /// 1 MHz
    Freq1MHz,
    /// 2 MHz
    Freq2MHz,
    /// 4 MHz
    Freq4MHz,
    /// 7 MHz
    Freq7MHz,
    /// 13 MHz
    Freq13MHz,
    /// 16 MHz
    Freq16MHz,
    /// 19 MHz
    Freq19MHz,
    /// 26 MHz
    Freq26MHz,
    /// 32 MHz
    Freq32MHz,
    /// 38 MHz
    #[default]
    Freq38MHz,
    /// 48 MHz
    Freq48MHz,
    /// 56 MHz
    Freq56MHz,
    /// 64 MHz
    Freq64MHz,
    /// 80 MHz
    Freq80MHz,
}

impl HfrcoFreq {
    /// Get the frequency in Hz.
    pub fn frequency_hz(&self) -> u32 {
        match self {
            HfrcoFreq::Freq1MHz => 1_000_000,
            HfrcoFreq::Freq2MHz => 2_000_000,
            HfrcoFreq::Freq4MHz => 4_000_000,
            HfrcoFreq::Freq7MHz => 7_000_000,
            HfrcoFreq::Freq13MHz => 13_000_000,
            HfrcoFreq::Freq16MHz => 16_000_000,
            HfrcoFreq::Freq19MHz => 19_000_000,
            HfrcoFreq::Freq26MHz => 26_000_000,
            HfrcoFreq::Freq32MHz => 32_000_000,
            HfrcoFreq::Freq38MHz => 38_000_000,
            HfrcoFreq::Freq48MHz => 48_000_000,
            HfrcoFreq::Freq56MHz => 56_000_000,
            HfrcoFreq::Freq64MHz => 64_000_000,
            HfrcoFreq::Freq80MHz => 80_000_000,
        }
    }
}

/// SYSCLK source selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SysclkSel {
    /// FSRCO (Fast startup RC oscillator)
    Fsrco,
    /// HFRCO (High frequency RC oscillator)
    #[default]
    Hfrco,
    /// HFXO (High frequency crystal oscillator)
    Hfxo,
    /// CLKIN0 (External clock input)
    Clkin0,
}

/// IADC clock source selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum IadcClkSel {
    /// EM01GRPACLK (default)
    #[default]
    Em01GrpA,
    /// FSRCO
    Fsrco,
    /// HFRCOEM23
    Hfrcoem23,
}

impl IadcClkSel {
    fn to_pac_val(self) -> pac::cmu::vals::IadcclkctrlClksel {
        match self {
            IadcClkSel::Em01GrpA => pac::cmu::vals::IadcclkctrlClksel::EM01GRPACLK,
            IadcClkSel::Fsrco => pac::cmu::vals::IadcclkctrlClksel::FSRCO,
            IadcClkSel::Hfrcoem23 => pac::cmu::vals::IadcclkctrlClksel::HFRCOEM23,
        }
    }
}

/// WDOG clock source selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WdogClkSel {
    /// LFRCO (Low frequency RC oscillator, ~32.768 kHz)
    #[default]
    Lfrco,
    /// LFXO (Low frequency crystal oscillator, 32.768 kHz)
    Lfxo,
    /// ULFRCO (Ultra low frequency RC oscillator, ~1 kHz)
    Ulfrco,
    /// HCLK / 1024
    HclkDiv1024,
}

impl WdogClkSel {
    fn to_wdog0_pac_val(self) -> pac::cmu::vals::Wdog0clkctrlClksel {
        match self {
            WdogClkSel::Lfrco => pac::cmu::vals::Wdog0clkctrlClksel::LFRCO,
            WdogClkSel::Lfxo => pac::cmu::vals::Wdog0clkctrlClksel::LFXO,
            WdogClkSel::Ulfrco => pac::cmu::vals::Wdog0clkctrlClksel::ULFRCO,
            WdogClkSel::HclkDiv1024 => pac::cmu::vals::Wdog0clkctrlClksel::HCLKDIV1024,
        }
    }

    fn to_wdog1_pac_val(self) -> pac::cmu::vals::Wdog1clkctrlClksel {
        match self {
            WdogClkSel::Lfrco => pac::cmu::vals::Wdog1clkctrlClksel::LFRCO,
            WdogClkSel::Lfxo => pac::cmu::vals::Wdog1clkctrlClksel::LFXO,
            WdogClkSel::Ulfrco => pac::cmu::vals::Wdog1clkctrlClksel::ULFRCO,
            WdogClkSel::HclkDiv1024 => pac::cmu::vals::Wdog1clkctrlClksel::HCLKDIV1024,
        }
    }

    /// Get the approximate clock frequency in Hz.
    pub fn frequency_hz(&self) -> u32 {
        match self {
            WdogClkSel::Lfrco => 32_768,
            WdogClkSel::Lfxo => 32_768,
            WdogClkSel::Ulfrco => 1_000,
            WdogClkSel::HclkDiv1024 => 39_063, // Assuming 40 MHz HCLK
        }
    }
}

/// EUSART clock source selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EusartClkSel {
    /// EM01GRPCCLK (default)
    #[default]
    Em01GrpC,
    /// HFRCOEM23
    Hfrcoem23,
    /// LFRCO
    Lfrco,
    /// LFXO
    Lfxo,
}

/// Clock Management Unit driver.
///
/// This struct provides static methods to configure the clock tree.
/// No instance is needed - all methods operate directly on hardware registers.
pub struct Cmu;

impl Cmu {
    // ========================================================================
    // Oscillator control
    // ========================================================================

    /// Enable the LFRCO (Low Frequency RC Oscillator, ~32.768 kHz).
    pub fn enable_lfrco() {
        pac::CMU.clken0().modify(|w| w.set_lfrco(true));
    }

    /// Disable the LFRCO.
    pub fn disable_lfrco() {
        pac::CMU.clken0().modify(|w| w.set_lfrco(false));
    }

    /// Check if LFRCO is enabled.
    pub fn is_lfrco_enabled() -> bool {
        pac::CMU.clken0().read().lfrco()
    }

    /// Enable the LFXO (Low Frequency Crystal Oscillator, 32.768 kHz).
    pub fn enable_lfxo() {
        pac::CMU.clken0().modify(|w| w.set_lfxo(true));
    }

    /// Disable the LFXO.
    pub fn disable_lfxo() {
        pac::CMU.clken0().modify(|w| w.set_lfxo(false));
    }

    /// Check if LFXO is enabled.
    pub fn is_lfxo_enabled() -> bool {
        pac::CMU.clken0().read().lfxo()
    }

    /// Enable the ULFRCO (Ultra Low Frequency RC Oscillator, ~1 kHz).
    pub fn enable_ulfrco() {
        pac::CMU.clken0().modify(|w| w.set_ulfrco(true));
    }

    /// Disable the ULFRCO.
    pub fn disable_ulfrco() {
        pac::CMU.clken0().modify(|w| w.set_ulfrco(false));
    }

    /// Check if ULFRCO is enabled.
    pub fn is_ulfrco_enabled() -> bool {
        pac::CMU.clken0().read().ulfrco()
    }

    /// Enable the HFRCO (High Frequency RC Oscillator).
    pub fn enable_hfrco() {
        pac::CMU.clken0().modify(|w| w.set_hfrco0(true));
    }

    /// Disable the HFRCO.
    pub fn disable_hfrco() {
        pac::CMU.clken0().modify(|w| w.set_hfrco0(false));
    }

    /// Check if HFRCO is enabled.
    pub fn is_hfrco_enabled() -> bool {
        pac::CMU.clken0().read().hfrco0()
    }

    /// Enable the HFXO (High Frequency Crystal Oscillator).
    pub fn enable_hfxo() {
        pac::CMU.clken0().modify(|w| w.set_hfxo0(true));
    }

    /// Disable the HFXO.
    pub fn disable_hfxo() {
        pac::CMU.clken0().modify(|w| w.set_hfxo0(false));
    }

    /// Check if HFXO is enabled.
    pub fn is_hfxo_enabled() -> bool {
        pac::CMU.clken0().read().hfxo0()
    }

    /// Enable the HFRCOEM23 (for EM2/3 operation).
    pub fn enable_hfrcoem23() {
        pac::CMU.clken0().modify(|w| w.set_hfrcoem23(true));
    }

    /// Disable the HFRCOEM23.
    pub fn disable_hfrcoem23() {
        pac::CMU.clken0().modify(|w| w.set_hfrcoem23(false));
    }

    /// Check if HFRCOEM23 is enabled.
    pub fn is_hfrcoem23_enabled() -> bool {
        pac::CMU.clken0().read().hfrcoem23()
    }

    // ========================================================================
    // Peripheral clock control
    // ========================================================================

    /// Enable the GPIO clock.
    pub fn enable_gpio() {
        pac::CMU.clken0().modify(|w| w.set_gpio(true));
    }

    /// Enable the IADC0 clock.
    pub fn enable_iadc0() {
        pac::CMU.clken0().modify(|w| w.set_iadc0(true));
    }

    /// Disable the IADC0 clock.
    pub fn disable_iadc0() {
        pac::CMU.clken0().modify(|w| w.set_iadc0(false));
    }

    /// Enable the WDOG0 clock.
    pub fn enable_wdog0() {
        pac::CMU.clken0().modify(|w| w.set_wdog0(true));
    }

    /// Disable the WDOG0 clock.
    pub fn disable_wdog0() {
        pac::CMU.clken0().modify(|w| w.set_wdog0(false));
    }

    /// Enable the WDOG1 clock.
    pub fn enable_wdog1() {
        pac::CMU.clken1().modify(|w| w.set_wdog1(true));
    }

    /// Disable the WDOG1 clock.
    pub fn disable_wdog1() {
        pac::CMU.clken1().modify(|w| w.set_wdog1(false));
    }

    /// Enable a timer clock by index.
    pub fn enable_timer(index: u8) {
        pac::CMU.clken0().modify(|w| {
            match index {
                0 => w.set_timer0(true),
                1 => w.set_timer1(true),
                2 => w.set_timer2(true),
                3 => w.set_timer3(true),
                4 => w.set_timer4(true),
                _ => {}
            }
        });
    }

    /// Disable a timer clock by index.
    pub fn disable_timer(index: u8) {
        pac::CMU.clken0().modify(|w| {
            match index {
                0 => w.set_timer0(false),
                1 => w.set_timer1(false),
                2 => w.set_timer2(false),
                3 => w.set_timer3(false),
                4 => w.set_timer4(false),
                _ => {}
            }
        });
    }

    /// Enable the I2C0 clock.
    pub fn enable_i2c0() {
        pac::CMU.clken0().modify(|w| w.set_i2c0(true));
    }

    /// Enable the I2C1 clock.
    pub fn enable_i2c1() {
        pac::CMU.clken0().modify(|w| w.set_i2c1(true));
    }

    /// Enable the USART0 clock.
    pub fn enable_usart0() {
        pac::CMU.clken0().modify(|w| w.set_usart0(true));
    }

    /// Enable the EUSART0 clock.
    pub fn enable_eusart0() {
        pac::CMU.clken1().modify(|w| w.set_eusart0(true));
    }

    /// Enable the EUSART1 clock.
    pub fn enable_eusart1() {
        pac::CMU.clken1().modify(|w| w.set_eusart1(true));
    }

    /// Enable the LDMA clock.
    pub fn enable_ldma() {
        pac::CMU.clken0().modify(|w| w.set_ldma(true));
    }

    /// Disable the LDMA clock.
    pub fn disable_ldma() {
        pac::CMU.clken0().modify(|w| w.set_ldma(false));
    }

    /// Enable the GPCRC clock.
    pub fn enable_gpcrc() {
        pac::CMU.clken0().modify(|w| w.set_gpcrc(true));
    }

    /// Disable the GPCRC clock.
    pub fn disable_gpcrc() {
        pac::CMU.clken0().modify(|w| w.set_gpcrc(false));
    }

    /// Enable the BURTC clock.
    pub fn enable_burtc() {
        pac::CMU.clken0().modify(|w| w.set_burtc(true));
    }

    /// Disable the BURTC clock.
    pub fn disable_burtc() {
        pac::CMU.clken0().modify(|w| w.set_burtc(false));
    }

    /// Enable the SYSRTC clock.
    pub fn enable_sysrtc() {
        pac::CMU.clken0().modify(|w| w.set_sysrtc0(true));
    }

    /// Disable the SYSRTC clock.
    pub fn disable_sysrtc() {
        pac::CMU.clken0().modify(|w| w.set_sysrtc0(false));
    }

    /// Enable the LETIMER0 clock.
    pub fn enable_letimer0() {
        pac::CMU.clken0().modify(|w| w.set_letimer0(true));
    }

    /// Disable the LETIMER0 clock.
    pub fn disable_letimer0() {
        pac::CMU.clken0().modify(|w| w.set_letimer0(false));
    }

    /// Enable the DCDC clock.
    pub fn enable_dcdc() {
        pac::CMU.clken0().modify(|w| w.set_dcdc(true));
    }

    /// Disable the DCDC clock.
    pub fn disable_dcdc() {
        pac::CMU.clken0().modify(|w| w.set_dcdc(false));
    }

    // ========================================================================
    // Peripheral clock source selection
    // ========================================================================

    /// Set the IADC clock source.
    pub fn set_iadc_clock_source(sel: IadcClkSel) {
        pac::CMU.iadcclkctrl().write(|w| w.set_clksel(sel.to_pac_val()));
    }

    /// Set the WDOG0 clock source.
    pub fn set_wdog0_clock_source(sel: WdogClkSel) {
        pac::CMU.wdog0clkctrl().write(|w| w.set_clksel(sel.to_wdog0_pac_val()));
    }

    /// Set the WDOG1 clock source.
    pub fn set_wdog1_clock_source(sel: WdogClkSel) {
        pac::CMU.wdog1clkctrl().write(|w| w.set_clksel(sel.to_wdog1_pac_val()));
    }

    // ========================================================================
    // Clock status
    // ========================================================================

    /// Get the current SYSCLK frequency (approximate).
    ///
    /// This returns the default frequency. For accurate frequency after
    /// configuration changes, track the frequency yourself.
    pub fn sysclk_frequency() -> u32 {
        // Default is HFRCO at 38 MHz
        // TODO: Read actual configuration
        38_000_000
    }

    /// Get the HCLK frequency (approximate).
    pub fn hclk_frequency() -> u32 {
        // HCLK is typically SYSCLK / 1
        Self::sysclk_frequency()
    }

    /// Get the PCLK frequency (approximate).
    pub fn pclk_frequency() -> u32 {
        // PCLK is typically HCLK / 1
        Self::hclk_frequency()
    }
}

/// Clocks configuration structure.
///
/// Use this to configure multiple clocks at once during initialization.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct ClocksConfig {
    /// Enable LFRCO
    pub lfrco: bool,
    /// Enable LFXO
    pub lfxo: bool,
    /// Enable ULFRCO
    pub ulfrco: bool,
    /// Enable HFRCOEM23
    pub hfrcoem23: bool,
    /// IADC clock source
    pub iadc_clk: Option<IadcClkSel>,
    /// WDOG0 clock source
    pub wdog0_clk: Option<WdogClkSel>,
    /// WDOG1 clock source
    pub wdog1_clk: Option<WdogClkSel>,
}

impl ClocksConfig {
    /// Create a new clocks configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable LFRCO.
    pub fn with_lfrco(mut self, enable: bool) -> Self {
        self.lfrco = enable;
        self
    }

    /// Enable LFXO.
    pub fn with_lfxo(mut self, enable: bool) -> Self {
        self.lfxo = enable;
        self
    }

    /// Enable ULFRCO.
    pub fn with_ulfrco(mut self, enable: bool) -> Self {
        self.ulfrco = enable;
        self
    }

    /// Enable HFRCOEM23.
    pub fn with_hfrcoem23(mut self, enable: bool) -> Self {
        self.hfrcoem23 = enable;
        self
    }

    /// Set IADC clock source.
    pub fn with_iadc_clock(mut self, sel: IadcClkSel) -> Self {
        self.iadc_clk = Some(sel);
        self
    }

    /// Set WDOG0 clock source.
    pub fn with_wdog0_clock(mut self, sel: WdogClkSel) -> Self {
        self.wdog0_clk = Some(sel);
        self
    }

    /// Set WDOG1 clock source.
    pub fn with_wdog1_clock(mut self, sel: WdogClkSel) -> Self {
        self.wdog1_clk = Some(sel);
        self
    }

    /// Apply this configuration to the hardware.
    pub fn apply(&self) {
        // Configure oscillators
        if self.lfrco {
            Cmu::enable_lfrco();
        }
        if self.lfxo {
            Cmu::enable_lfxo();
        }
        if self.ulfrco {
            Cmu::enable_ulfrco();
        }
        if self.hfrcoem23 {
            Cmu::enable_hfrcoem23();
        }

        // Configure peripheral clocks
        if let Some(sel) = self.iadc_clk {
            Cmu::set_iadc_clock_source(sel);
        }
        if let Some(sel) = self.wdog0_clk {
            Cmu::set_wdog0_clock_source(sel);
        }
        if let Some(sel) = self.wdog1_clk {
            Cmu::set_wdog1_clock_source(sel);
        }
    }
}
