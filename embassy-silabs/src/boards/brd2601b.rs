//! Board support for BRD2601B - Thunderboard Sense 2 (EFR32MG24)
//!
//! This board includes:
//! - 3 LEDs (active low, accent RGB LED)
//! - 2 push buttons
//! - Multiple sensors on I2C1:
//!   - Si7021 temperature/humidity sensor
//!   - Si7210 Hall effect sensor
//!   - BMP384 pressure sensor
//!   - VEML6035 ambient light sensor
//!   - ICM-20689 6-axis IMU
//! - PDM microphone
//! - MX25R8035F SPI flash
//!
//! ## Pin Assignments
//!
//! | Function        | Pin  | Notes                    |
//! |-----------------|------|--------------------------|
//! | LED0 (Red)      | PD02 | Active low               |
//! | LED1 (Green)    | PA04 | Active low               |
//! | BTN0            | PB02 | Active low with pull-up  |
//! | BTN1            | PB03 | Active low with pull-up  |
//! | I2C1 SCL        | PC04 | Sensor I2C               |
//! | I2C1 SDA        | PC05 | Sensor I2C               |
//! | Sensor Enable   | PC09 | Powers most sensors      |
//! | Mic Enable      | PC08 | Powers microphone        |

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

/// Board configuration for BRD2601B (Thunderboard Sense 2)
///
/// This struct provides type-safe access to board-specific peripherals.
/// Use `Board::new()` to extract board peripherals from the HAL `Peripherals` struct.
pub struct Board {
    // LEDs (active low)
    /// LED 0 - Red LED on PD02
    pub led0: Peri<'static, PD_02>,
    /// LED 1 - Green LED on PA04
    pub led1: Peri<'static, PA_04>,

    // Buttons
    /// Push button 0 on PB02
    pub btn0: Peri<'static, PB_02>,
    /// Push button 1 on PB03
    pub btn1: Peri<'static, PB_03>,

    // Sensor I2C (I2C1)
    /// I2C1 SCL for sensors
    pub sensor_scl: Peri<'static, PC_04>,
    /// I2C1 SDA for sensors
    pub sensor_sda: Peri<'static, PC_05>,

    // Enable pins
    /// Sensor power enable (RHT, Hall, Pressure, Light, IMU)
    pub sensor_enable: Peri<'static, PC_09>,
    /// Microphone power enable
    pub mic_enable: Peri<'static, PC_08>,
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
    pub pa05: Peri<'static, PA_05>,
    pub pa06: Peri<'static, PA_06>,
    pub pa07: Peri<'static, PA_07>,
    pub pa08: Peri<'static, PA_08>,
    pub pa09: Peri<'static, PA_09>,

    // Remaining GPIO Port B
    pub pb00: Peri<'static, PB_00>,
    pub pb01: Peri<'static, PB_01>,
    pub pb04: Peri<'static, PB_04>,
    pub pb05: Peri<'static, PB_05>,

    // Remaining GPIO Port C
    pub pc00: Peri<'static, PC_00>,
    pub pc01: Peri<'static, PC_01>,
    pub pc02: Peri<'static, PC_02>,
    pub pc03: Peri<'static, PC_03>,
    pub pc06: Peri<'static, PC_06>,
    pub pc07: Peri<'static, PC_07>,

    // Remaining GPIO Port D
    pub pd00: Peri<'static, PD_00>,
    pub pd01: Peri<'static, PD_01>,
    pub pd03: Peri<'static, PD_03>,
    pub pd04: Peri<'static, PD_04>,
    pub pd05: Peri<'static, PD_05>,
}

impl Board {
    /// LED polarity for this board (all LEDs are active low)
    pub const LED_POLARITY: LedPolarity = LedPolarity::ActiveLow;

    /// Create a new board configuration from the HAL peripherals.
    ///
    /// This consumes the board-specific pins and returns the remaining
    /// peripherals for application use.
    pub fn new(p: Peripherals) -> (Self, RemainingPeripherals) {
        let board = Self {
            // LEDs
            led0: p.PD_02,
            led1: p.PA_04,

            // Buttons
            btn0: p.PB_02,
            btn1: p.PB_03,

            // Sensor I2C
            sensor_scl: p.PC_04,
            sensor_sda: p.PC_05,

            // Enable pins
            sensor_enable: p.PC_09,
            mic_enable: p.PC_08,
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
            pa05: p.PA_05,
            pa06: p.PA_06,
            pa07: p.PA_07,
            pa08: p.PA_08,
            pa09: p.PA_09,
            pb00: p.PB_00,
            pb01: p.PB_01,
            pb04: p.PB_04,
            pb05: p.PB_05,
            pc00: p.PC_00,
            pc01: p.PC_01,
            pc02: p.PC_02,
            pc03: p.PC_03,
            pc06: p.PC_06,
            pc07: p.PC_07,
            pd00: p.PD_00,
            pd01: p.PD_01,
            pd03: p.PD_03,
            pd04: p.PD_04,
            pd05: p.PD_05,
        };

        (board, remaining)
    }
}
