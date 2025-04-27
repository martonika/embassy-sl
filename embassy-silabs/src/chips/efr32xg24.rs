pub mod pac {
    pub use silabs_pac::*;

    // nonsecure
    #[cfg(feature = "_ns")]
    #[doc(no_inline)]
    pub use silabs_pac::{
        CMU_NS as CMU, GPIO_NS as GPIO, LFRCO_NS as LFRCO, SYSRTC0_NS as SYSRTC,
        TIMER0_NS as TIMER0, TIMER1_NS as TIMER1, TIMER2_NS as TIMER2, TIMER3_NS as TIMER3,
        TIMER4_NS as TIMER4,
    };
}

embassy_hal_internal::peripherals! {
    // RTC
    SYSRTC,
    // Clock sources
    LFRCO,
    // Timers
    TIMER0,
    TIMER1,
    TIMER2,
    TIMER3,
    TIMER4,

    // GPIO Port A
    PA_00,
    PA_01,
    PA_02,
    PA_03,
    PA_04,
    PA_05,
    PA_06,
    PA_07,
    PA_08,
    PA_09,

    // GPIO Port B
    PB_00,
    PB_01,
    PB_02,
    PB_03,
    PB_04,
    PB_05,

    // GPIO Port C
    PC_00,
    PC_01,
    PC_02,
    PC_03,
    PC_04,
    PC_05,
    PC_06,
    PC_07,
    PC_08,
    PC_09,

    // GPIO Port D
    PD_00,
    PD_01,
    PD_02,
    PD_03,
    PD_04,
    PD_05,
}

// In our implementation, pin_port is port_num*16 + pin_num,
// where Port A = 0 Port B = 1, Port C = 2, Port D = 3
impl_pin!(PA_00, 0, 0);
impl_pin!(PA_01, 0, 1);
impl_pin!(PA_02, 0, 2);
impl_pin!(PA_03, 0, 3);
impl_pin!(PA_04, 0, 4);
impl_pin!(PA_05, 0, 5);
impl_pin!(PA_06, 0, 6);
impl_pin!(PA_07, 0, 7);
impl_pin!(PA_08, 0, 8);
impl_pin!(PA_09, 0, 9);

impl_pin!(PB_00, 1, 0);
impl_pin!(PB_01, 1, 1);
impl_pin!(PB_02, 1, 2);
impl_pin!(PB_03, 1, 3);
impl_pin!(PB_04, 1, 4);
impl_pin!(PB_05, 1, 5);

impl_pin!(PC_00, 2, 0);
impl_pin!(PC_01, 2, 1);
impl_pin!(PC_02, 2, 2);
impl_pin!(PC_03, 2, 3);
impl_pin!(PC_04, 2, 4);
impl_pin!(PC_05, 2, 5);
impl_pin!(PC_06, 2, 6);
impl_pin!(PC_07, 2, 7);
impl_pin!(PC_08, 2, 8);
impl_pin!(PC_09, 2, 9);

impl_pin!(PD_00, 3, 0);
impl_pin!(PD_01, 3, 1);
impl_pin!(PD_02, 3, 2);
impl_pin!(PD_03, 3, 3);
impl_pin!(PD_04, 3, 4);
impl_pin!(PD_05, 3, 5);

// Embassy's own 'mod interrupt' macro
embassy_hal_internal::interrupt_mod!(
    //RTC
    SYSRTC_APP, // Clock sources
    LFRCO, // Timers
    TIMER0, TIMER1, TIMER2, TIMER3, TIMER4,
);
