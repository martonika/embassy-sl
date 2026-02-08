//! Board support for BRD4186C - xG24 Dev Kit (EFR32MG24B210F1536IM48)
//!
//! This board includes:
//! - 2 LEDs (active high)
//! - 2 push buttons
//! - Si7021 temperature/humidity sensor on I2C1
//! - LS013B7DH03 Sharp Memory LCD on USART0 SPI
//! - MX25R8035F SPI flash
//! - VCOM (Virtual COM port) via debug USB
//!
//! ## Pin Assignments
//!
//! | Function       | Pin  | Notes                    |
//! |----------------|------|--------------------------|
//! | LED0           | PB02 | Active high              |
//! | LED1           | PB04 | Active high              |
//! | BTN0           | PB01 | Active low with pull-up  |
//! | BTN1           | PB03 | Active low with pull-up  |
//! | I2C1 SCL       | PC05 | Sensor I2C               |
//! | I2C1 SDA       | PC07 | Sensor I2C               |
//! | Display MOSI   | PC01 | USART0 TX                |
//! | Display CLK    | PC03 | USART0 CLK               |
//! | Display CS     | PC08 | GPIO                     |
//! | Display Enable | PC09 | GPIO, enables display    |
//! | VCOM Enable    | PB00 | GPIO, enables VCOM       |
//! | Sensor Enable  | PD03 | GPIO, enables RHT sensor |

use crate::peripherals::*;
use crate::{Peri, Peripherals};

/// Pin polarity for LEDs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedPolarity {
    /// LED is on when pin is high
    ActiveHigh,
    /// LED is on when pin is low
    ActiveLow,
}

/// Board configuration for BRD4186C (xG24 Dev Kit)
///
/// This struct provides type-safe access to board-specific peripherals.
/// Use `Board::new()` to extract board peripherals from the HAL `Peripherals` struct.
pub struct Board {
    // LEDs
    /// LED 0 - Green LED on PB02
    pub led0: Peri<'static, PB_02>,
    /// LED 1 - Green LED on PB04
    pub led1: Peri<'static, PB_04>,

    // Buttons
    /// Push button 0 on PB01
    pub btn0: Peri<'static, PB_01>,
    /// Push button 1 on PB03
    pub btn1: Peri<'static, PB_03>,

    // Sensor I2C (I2C1)
    /// I2C1 SCL for sensors
    pub sensor_scl: Peri<'static, PC_05>,
    /// I2C1 SDA for sensors
    pub sensor_sda: Peri<'static, PC_07>,

    // Display SPI (USART0)
    /// Display SPI MOSI (USART0 TX)
    pub display_mosi: Peri<'static, PC_01>,
    /// Display SPI CLK (USART0 CLK)
    pub display_clk: Peri<'static, PC_03>,
    /// Display SPI CS (GPIO)
    pub display_cs: Peri<'static, PC_08>,
    /// External COM inversion signal
    pub display_extcomin: Peri<'static, PC_06>,

    // Enable pins
    /// Display power enable
    pub display_enable: Peri<'static, PC_09>,
    /// VCOM (Virtual COM port) enable
    pub vcom_enable: Peri<'static, PB_00>,
    /// RHT (Relative Humidity & Temperature) sensor enable
    pub sensor_enable: Peri<'static, PD_03>,
}

/// Remaining peripherals not used by the board configuration.
///
/// These can be used for custom application needs.
pub struct RemainingPeripherals {
    // Timers
    pub timer0: Peri<'static, TIMER0>,
    pub timer1: Peri<'static, TIMER1>,
    pub timer2: Peri<'static, TIMER2>,
    pub timer3: Peri<'static, TIMER3>,
    pub timer4: Peri<'static, TIMER4>,

    // Communication peripherals
    pub usart0: Peri<'static, USART0>,
    pub eusart0: Peri<'static, EUSART0>,
    pub eusart1: Peri<'static, EUSART1>,
    pub i2c0: Peri<'static, I2C0>,
    pub i2c1: Peri<'static, I2C1>,

    // ADC
    pub iadc0: Peri<'static, IADC0>,

    // Watchdogs
    pub wdog0: Peri<'static, WDOG0>,
    pub wdog1: Peri<'static, WDOG1>,

    // Remaining GPIO Port A
    pub pa00: Peri<'static, PA_00>,
    pub pa01: Peri<'static, PA_01>,
    pub pa02: Peri<'static, PA_02>,
    pub pa03: Peri<'static, PA_03>,
    pub pa04: Peri<'static, PA_04>,
    pub pa05: Peri<'static, PA_05>,
    pub pa06: Peri<'static, PA_06>,
    pub pa07: Peri<'static, PA_07>,
    pub pa08: Peri<'static, PA_08>,
    pub pa09: Peri<'static, PA_09>,

    // Remaining GPIO Port B
    pub pb05: Peri<'static, PB_05>,

    // Remaining GPIO Port C
    pub pc00: Peri<'static, PC_00>,
    pub pc02: Peri<'static, PC_02>,
    pub pc04: Peri<'static, PC_04>,

    // Remaining GPIO Port D
    pub pd00: Peri<'static, PD_00>,
    pub pd01: Peri<'static, PD_01>,
    pub pd02: Peri<'static, PD_02>,
    pub pd04: Peri<'static, PD_04>,
    pub pd05: Peri<'static, PD_05>,
}

impl Board {
    /// LED polarity for this board (both LEDs are active high)
    pub const LED_POLARITY: LedPolarity = LedPolarity::ActiveHigh;

    /// Create a new board configuration from the HAL peripherals.
    ///
    /// This consumes the board-specific pins and returns the remaining
    /// peripherals for application use.
    pub fn new(p: Peripherals) -> (Self, RemainingPeripherals) {
        let board = Self {
            // LEDs
            led0: p.PB_02,
            led1: p.PB_04,

            // Buttons
            btn0: p.PB_01,
            btn1: p.PB_03,

            // Sensor I2C
            sensor_scl: p.PC_05,
            sensor_sda: p.PC_07,

            // Display SPI
            display_mosi: p.PC_01,
            display_clk: p.PC_03,
            display_cs: p.PC_08,
            display_extcomin: p.PC_06,

            // Enable pins
            display_enable: p.PC_09,
            vcom_enable: p.PB_00,
            sensor_enable: p.PD_03,
        };

        let remaining = RemainingPeripherals {
            // Timers
            timer0: p.TIMER0,
            timer1: p.TIMER1,
            timer2: p.TIMER2,
            timer3: p.TIMER3,
            timer4: p.TIMER4,

            // Communication peripherals
            usart0: p.USART0,
            eusart0: p.EUSART0,
            eusart1: p.EUSART1,
            i2c0: p.I2C0,
            i2c1: p.I2C1,

            // ADC
            iadc0: p.IADC0,

            // Watchdogs
            wdog0: p.WDOG0,
            wdog1: p.WDOG1,

            // Remaining GPIO
            pa00: p.PA_00,
            pa01: p.PA_01,
            pa02: p.PA_02,
            pa03: p.PA_03,
            pa04: p.PA_04,
            pa05: p.PA_05,
            pa06: p.PA_06,
            pa07: p.PA_07,
            pa08: p.PA_08,
            pa09: p.PA_09,
            pb05: p.PB_05,
            pc00: p.PC_00,
            pc02: p.PC_02,
            pc04: p.PC_04,
            pd00: p.PD_00,
            pd01: p.PD_01,
            pd02: p.PD_02,
            pd04: p.PD_04,
            pd05: p.PD_05,
        };

        (board, remaining)
    }
}
