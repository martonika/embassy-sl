#![no_std]
#![no_main]

use embassy_executor::Spawner;

use defmt::*;
use embassy_silabs::gpio::*;
use embassy_time::{Instant, Timer};
use {defmt_rtt as _, panic_probe as _}; // global logger

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Initialize peripherals");
    let p = embassy_silabs::init();
    let led0 = Output::new(p.PA_04, Level::Low);
    let led1 = Output::new(p.PA_07, Level::Low);
    //let led0 = Output::new(p.PB_02, Level::Low);
    //let led1 = Output::new(p.PB_04, Level::Low);
    unwrap!(spawner.spawn(blink_1(led0)));
    unwrap!(spawner.spawn(blink_2(led1)));
}

#[embassy_executor::task]
async fn blink_1(mut led: Output<'static>) {
    loop {
        let now = Instant::now();
        info!("Blink 1 triggered at {}", now.as_millis());
        led.toggle();
        Timer::after_millis(1500).await;
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
