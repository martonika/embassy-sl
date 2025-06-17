#![no_std]
#![no_main]

use embassy_executor::Spawner;

use defmt::*;
use embassy_silabs::gpio::*;
use embassy_time::{Instant, Timer};
use silabs_pac::gpio::vals::PortMode;
use {defmt_rtt as _, panic_probe as _}; // global logger

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Initialize peripherals");
    let p = embassy_silabs::init();
    //let led0 = Output::new(p.PA_04, Level::Low); // BRD2703A, MG24 Explorer Kit
    //let led1 = Output::new(p.PA_07, Level::Low); // BRD2703A, MG24 Explorer Kit
    let led0 = Output::new(p.PB_02, Level::Low); // BRD4187C + WPK
    let led1 = Output::new(p.PB_04, Level::Low); // BRD4187C + WPK
    let _button0 = Input::new(p.PB_01, PortMode::INPUT); // BRD4187C + WPK
    let _button1 = Input::new(p.PB_03, PortMode::INPUT); // BRD4187C + WPK
    unwrap!(spawner.spawn(blink_1(led0)));
    unwrap!(spawner.spawn(blink_2(led1)));
}

#[embassy_executor::task]
async fn blink_1(mut led: Output<'static>) {
    loop {
        let now = Instant::now();
        info!("Blink 1 triggered at {}", now.as_millis());
        led.toggle();
        Timer::after_millis(1200).await;
    }
}

#[embassy_executor::task]
async fn blink_2(mut led: Output<'static>) {
    loop {
        let now = Instant::now();
        info!("Blink 2 triggered at {}", now.as_millis());
        led.toggle();
        Timer::after_millis(2000).await;
    }
}
