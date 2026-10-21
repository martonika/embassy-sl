//! Board support for **BRD4186C** — xG24 Dev Kit with **+10 dBm** radio (EFR32MG24**B210**).
//!
//! Enable `board-brd4186c` in `embassy-silabs` (and app crates) to select the B210
//! RAIL sequencer image (`RAIL_SEQ_IMAGE_PA_10_DBM`).
//!
//! ## Pin Assignments (BRD4002A mainboard)
//!
//! | Function        | Pin  | Notes                    |
//! |-----------------|------|--------------------------|
//! | LED0            | PB02 | Active high              |
//! | LED1            | PB04 | Active high              |
//! | BTN0            | PB01 |                          |
//! | BTN1            | PB03 |                          |
//! | VCOM enable     | PB00 | WSTK board controller    |
//! | Display MOSI    | PC01 | EUSART1 TX               |
//! | Display CLK     | PC03 | EUSART1 CLK              |
//! | Display EXTCOMIN| PC06 |                          |
//! | Display CS      | PC08 |                          |
//! | Display enable  | PC09 |                          |
//! | Sensor I2C SCL  | PC05 | I2C1                     |
//! | Sensor I2C SDA  | PC07 | I2C1                     |
//! | Sensor enable   | PD03 |                          |

use crate::peripherals::*;
use crate::prs::AsyncChannel;
use crate::{Peri, Peripherals};

/// Silicon Labs board ordering number.
pub const BOARD_NAME: &str = "BRD4186C";

/// Target MCU (+10 dBm PA).
pub const MG24_PART: &str = "EFR32MG24B210F1536IM48";

/// Maximum conducted output power for the on-board radio module.
pub const RADIO_PA_DBM: u8 = 10;

/// Pin polarity for LEDs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedPolarity {
    /// LED is on when pin is high
    ActiveHigh,
    /// LED is on when pin is low
    ActiveLow,
}

/// BRD4186C board peripherals
pub struct Board {
    /// LED 0 - Green LED on PB02
    pub led0: Peri<'static, PB_02>,
    /// LED 1 - Green LED on PB04
    pub led1: Peri<'static, PB_04>,

    /// Push button 0 on PB01
    pub btn0: Peri<'static, PB_01>,
    /// Push button 1 on PB03
    pub btn1: Peri<'static, PB_03>,

    /// I2C1 SCL for sensors
    pub sensor_scl: Peri<'static, PC_05>,
    /// I2C1 SDA for sensors
    pub sensor_sda: Peri<'static, PC_07>,

    /// Display SPI MOSI (EUSART1 TX)
    pub display_mosi: Peri<'static, PC_01>,
    /// Display SPI CLK (EUSART1 CLK)
    pub display_clk: Peri<'static, PC_03>,
    /// Display SPI CS (GPIO)
    pub display_cs: Peri<'static, PC_08>,
    /// External COM inversion signal
    pub display_extcomin: Peri<'static, PC_06>,

    /// Display power enable
    pub display_enable: Peri<'static, PC_09>,
    /// WSTK board-controller / VCOM enable (PB00)
    pub vcom_enable: Peri<'static, PB_00>,
    /// RHT sensor enable
    pub sensor_enable: Peri<'static, PD_03>,
}

/// Remaining peripherals not used by the board configuration.
pub struct RemainingPeripherals {
    pub timer0: Peri<'static, TIMER0>,
    pub timer1: Peri<'static, TIMER1>,
    pub timer2: Peri<'static, TIMER2>,
    pub timer3: Peri<'static, TIMER3>,
    pub timer4: Peri<'static, TIMER4>,

    pub usart0: Peri<'static, USART0>,
    pub eusart0: Peri<'static, EUSART0>,
    pub eusart1: Peri<'static, EUSART1>,
    pub i2c0: Peri<'static, I2C0>,
    pub i2c1: Peri<'static, I2C1>,

    pub iadc0: Peri<'static, IADC0>,

    pub wdog0: Peri<'static, WDOG0>,
    pub wdog1: Peri<'static, WDOG1>,

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

    pub pb05: Peri<'static, PB_05>,

    pub pc00: Peri<'static, PC_00>,
    pub pc02: Peri<'static, PC_02>,
    pub pc04: Peri<'static, PC_04>,

    pub pd00: Peri<'static, PD_00>,
    pub pd01: Peri<'static, PD_01>,
    pub pd02: Peri<'static, PD_02>,
    pub pd04: Peri<'static, PD_04>,
    pub pd05: Peri<'static, PD_05>,
}

impl Board {
    pub const LED_POLARITY: LedPolarity = LedPolarity::ActiveHigh;

    pub fn new(p: Peripherals) -> (Self, RemainingPeripherals) {
        let board = Self {
            led0: p.PB_02,
            led1: p.PB_04,
            btn0: p.PB_01,
            btn1: p.PB_03,
            sensor_scl: p.PC_05,
            sensor_sda: p.PC_07,
            display_mosi: p.PC_01,
            display_clk: p.PC_03,
            display_cs: p.PC_08,
            display_extcomin: p.PC_06,
            display_enable: p.PC_09,
            vcom_enable: p.PB_00,
            sensor_enable: p.PD_03,
        };

        let remaining = RemainingPeripherals {
            timer0: p.TIMER0,
            timer1: p.TIMER1,
            timer2: p.TIMER2,
            timer3: p.TIMER3,
            timer4: p.TIMER4,
            usart0: p.USART0,
            eusart0: p.EUSART0,
            eusart1: p.EUSART1,
            i2c0: p.I2C0,
            i2c1: p.I2C1,
            iadc0: p.IADC0,
            wdog0: p.WDOG0,
            wdog1: p.WDOG1,
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

    /// Route RACL PAEN (TX) and LNAEN (RX) PRS signals to LED0 and LED1.
    pub fn route_rf_activity_leds(self) {
        crate::prs::route_racl_paen(AsyncChannel::new(0), self.led0);
        crate::prs::route_racl_lnaen(AsyncChannel::new(1), self.led1);
    }
}
