//! Peripheral Reflex System (PRS) helpers for EFR32xG24.
//!
//! Routes asynchronous PRS producers to GPIO pins. On xG24 the radio front-end
//! activity signals are named RACL_PAEN (TX) and RACL_LNAEN (RX).

use crate::gpio::{AnyPin, Pin, SealedPin};
use crate::pac::{self, gpio::vals::PortMode, prs::vals::AsyncCh0CtrlFnsel};
use crate::Peri;

/// Async PRS channel index (0-15 on EFR32xG24).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsyncChannel(u8);

impl AsyncChannel {
    /// Create a channel index. Panics in debug builds if `ch >= 16`.
    pub const fn new(ch: u8) -> Self {
        assert!(ch < 16);
        Self(ch)
    }

    const fn index(self) -> u8 {
        self.0
    }
}

/// RACL PRS source select (`PRS_ASYNC_CH_CTRL_SOURCESEL_RACL`).
const RACL_SOURCESEL: u8 = 0x30;
/// RACL PAEN signal select (`PRS_ASYNC_CH_CTRL_SIGSEL_RACLPAEN`, TX activity).
const RACL_PAEN_SIGSEL: u8 = 0x02;
/// RACL LNAEN signal select (`PRS_ASYNC_CH_CTRL_SIGSEL_RACLLNAEN`, RX activity).
const RACL_LNAEN_SIGSEL: u8 = 0x01;

fn prs() -> pac::prs::Prs {
    #[cfg(feature = "_ns")]
    {
        pac::PRS_NS
    }
    #[cfg(not(feature = "_ns"))]
    {
        pac::PRS_S
    }
}

fn enable_clocks() {
    pac::CMU.clken0().modify(|w| {
        w.set_gpio(true);
        w.set_prs(true);
    });
}

fn configure_gpio_for_prs(pin: &Peri<'_, AnyPin>) {
    pin.mode_w(PortMode::PUSHPULL);
    pin.set_low();
}

fn route_racl_signal<P: Pin>(ch: AsyncChannel, pin: Peri<'_, P>, sigsel: u8) {
    enable_clocks();
    let pin: Peri<'_, AnyPin> = pin.into();
    let port = pin.pin_port() / 16;
    let pin_num = pin._pin();
    configure_gpio_for_prs(&pin);
    connect_async(ch, RACL_SOURCESEL, sigsel);
    route_channel_to_pin(ch, port, pin_num);
}

/// Route RACL PAEN (TX activity) to a GPIO pin via PRS.
pub fn route_racl_paen<P: Pin>(ch: AsyncChannel, pin: Peri<'_, P>) {
    route_racl_signal(ch, pin, RACL_PAEN_SIGSEL);
}

/// Route RACL LNAEN (RX activity) to a GPIO pin via PRS.
pub fn route_racl_lnaen<P: Pin>(ch: AsyncChannel, pin: Peri<'_, P>) {
    route_racl_signal(ch, pin, RACL_LNAEN_SIGSEL);
}

fn connect_async(ch: AsyncChannel, sourcesel: u8, sigsel: u8) {
    let p = prs();
    let fnsel = AsyncCh0CtrlFnsel::A;
    match ch.index() {
        0 => p.async_ch0_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh0CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(fnsel);
        }),
        1 => p.async_ch1_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh1CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh1CtrlFnsel::A);
        }),
        2 => p.async_ch2_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh2CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh2CtrlFnsel::A);
        }),
        3 => p.async_ch3_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh3CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh3CtrlFnsel::A);
        }),
        4 => p.async_ch4_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh4CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh4CtrlFnsel::A);
        }),
        5 => p.async_ch5_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh5CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh5CtrlFnsel::A);
        }),
        6 => p.async_ch6_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh6CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh6CtrlFnsel::A);
        }),
        7 => p.async_ch7_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh7CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh7CtrlFnsel::A);
        }),
        8 => p.async_ch8_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh8CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh8CtrlFnsel::A);
        }),
        9 => p.async_ch9_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh9CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh9CtrlFnsel::A);
        }),
        10 => p.async_ch10_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh10CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh10CtrlFnsel::A);
        }),
        11 => p.async_ch11_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh11CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh11CtrlFnsel::A);
        }),
        12 => p.async_ch12_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh12CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh12CtrlFnsel::A);
        }),
        13 => p.async_ch13_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh13CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh13CtrlFnsel::A);
        }),
        14 => p.async_ch14_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh14CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh14CtrlFnsel::A);
        }),
        15 => p.async_ch15_ctrl().modify(|w| {
            w.set_sourcesel(sourcesel);
            w.set_sigsel(pac::prs::vals::AsyncCh15CtrlSigsel::from_bits(sigsel));
            w.set_fnsel(pac::prs::vals::AsyncCh15CtrlFnsel::A);
        }),
        _ => unreachable!(),
    }
}

fn route_channel_to_pin(ch: AsyncChannel, port: u8, pin: u8) {
    let gpio = pac::GPIO;
    match ch.index() {
        0 => gpio.prs0_asynch0route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        1 => gpio.prs0_asynch1route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        2 => gpio.prs0_asynch2route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        3 => gpio.prs0_asynch3route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        4 => gpio.prs0_asynch4route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        5 => gpio.prs0_asynch5route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        6 => gpio.prs0_asynch6route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        7 => gpio.prs0_asynch7route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        8 => gpio.prs0_asynch8route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        9 => gpio.prs0_asynch9route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        10 => gpio.prs0_asynch10route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        11 => gpio.prs0_asynch11route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        12 => gpio.prs0_asynch12route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        13 => gpio.prs0_asynch13route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        14 => gpio.prs0_asynch14route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        15 => gpio.prs0_asynch15route().modify(|w| {
            w.set_port(port);
            w.set_pin(pin);
        }),
        _ => unreachable!(),
    }

    gpio.prs0_routeen().modify(|w| match ch.index() {
        0 => w.set_asynch0pen(true),
        1 => w.set_asynch1pen(true),
        2 => w.set_asynch2pen(true),
        3 => w.set_asynch3pen(true),
        4 => w.set_asynch4pen(true),
        5 => w.set_asynch5pen(true),
        6 => w.set_asynch6pen(true),
        7 => w.set_asynch7pen(true),
        8 => w.set_asynch8pen(true),
        9 => w.set_asynch9pen(true),
        10 => w.set_asynch10pen(true),
        11 => w.set_asynch11pen(true),
        12 => w.set_asynch12pen(true),
        13 => w.set_asynch13pen(true),
        14 => w.set_asynch14pen(true),
        15 => w.set_asynch15pen(true),
        _ => unreachable!(),
    });
}
