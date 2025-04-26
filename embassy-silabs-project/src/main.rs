#![no_std]
#![no_main]

use cortex_m::asm::nop;
use embassy_executor::Spawner;
//use cortex_m_rt::entry;
//use embassy_time::{Duration, Timer};
use defmt::{error, info};
use embassy_silabs::gpio::*;
use silabs_pac as pac;
use {defmt_rtt as _, panic_probe as _}; // global logger

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    //#[entry]
    //fn main() -> ! {
    sysrtc_init();
    info!("Initialize pins");
    board_init_pins();
    let p = embassy_silabs::init();
    let mut led = Output::new(p.PA_04, Level::Low);
    let mut is_on = false;
    //let mut reg;
    loop {
        if is_on {
            info!("LED OFF");
            //reg = 0b0000_0000;
            led.set_low();
        } else {
            info!("LED ON");
            //reg = 0b0001_0000;
            led.set_high();
        }

        //p.GPIO_NS
        //    .porta_dout()
        //    .write(|w| unsafe { w.dout().bits(reg) });

        for _ in 0..400_000 {
            nop();
        }
        is_on = !is_on;
    }
}

fn board_init_pins() {
    // Enable GPIO clock if not already set
    //if p.CMU_NS.clken0().read().gpio().bit_is_clear() {
    //    p.CMU_NS.clken0().write(|w| w.gpio().set_bit());
    //}
    if !pac::cmu_ns::CmuNs::clken0(pac::CMU_NS).read().gpio() {
        pac::cmu_ns::CmuNs::clken0(pac::CMU_NS).write(|w| w.set_gpio(true));
    }
    // Set GPIO Port A pin 4 to push-pull output -> drives the LED
    // same check-before modify
    //if !p.GPIO_NS.porta_model().read().mode4().is_pushpull() {
    //    p.GPIO_NS.porta_model().write(|w| w.mode4().pushpull());
    //}
    if pac::gpio::ns::port_a(pac::GPIO_NS).model().read().mode4()
        != pac::gpio::vals::PortMode::PUSHPULL
    {
        pac::gpio::ns::port_a(pac::GPIO_NS)
            .model()
            .write(|w| w.set_mode4(pac::gpio::vals::PortMode::PUSHPULL));
    }
}

fn sysrtc_init() {
    // "The normal operation of SYSRTC is to configure it, enable it, start it, and then leave it running"
    // And then you find out that CMU has to be enabled here as well, otherwise hard fault
    //if p.CMU_NS.clken0().read().sysrtc0().bit_is_clear() {
    //    p.CMU_NS.clken0().write(|w| w.sysrtc0().set_bit());
    //}
    if !pac::cmu_ns::CmuNs::clken0(pac::CMU_NS).read().sysrtc0() {
        pac::cmu_ns::CmuNs::clken0(pac::CMU_NS).write(|w| w.set_sysrtc0(true));
    }
    // Config
    // Disable SysRTC during debug break
    //if p.SYSRTC0_NS.cfg().read().debugrun().is_enable() {
    //    p.SYSRTC0_NS.cfg().write(|w| w.debugrun().disable());
    //}
    if pac::sysrtc0_ns::Sysrtc0Ns::cfg(pac::SYSRTC0_NS)
        .read()
        .debugrun()
        == pac::sysrtc0_ns::vals::Debugrun::ENABLE
    {
        pac::sysrtc0_ns::Sysrtc0Ns::cfg(pac::SYSRTC0_NS)
            .write(|w| w.set_debugrun(silabs_pac::sysrtc0_ns::vals::Debugrun::DISABLE));
    }
    // Enable RTC
    //if p.SYSRTC0_NS.en().read().en().bit_is_clear() {
    //    p.SYSRTC0_NS.en().write(|w| w.en().set_bit());
    //}
    if !pac::sysrtc0_ns::Sysrtc0Ns::en(pac::SYSRTC0_NS).read().en() {
        pac::sysrtc0_ns::Sysrtc0Ns::en(pac::SYSRTC0_NS).write(|w| w.set_en(true));
    }

    // Start
    //p.SYSRTC0_NS.cmd().write(|w| w.start().set_bit());

    pac::sysrtc0_ns::Sysrtc0Ns::cmd(pac::SYSRTC0_NS).write(|w| w.set_start(true));
}
