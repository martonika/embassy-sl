#![no_std]
#![no_main]

#[path = "../ble_app.rs"]
mod ble_app;
#[path = "../ble_runtime.rs"]
mod ble_runtime;

use core::sync::atomic::Ordering;

use defmt::*;
use embassy_executor::Spawner;
use embassy_silabs::boards::brd4186c::Board;
use embassy_silabs::drivers::display::memlcd::{
    MemLcd, MemLcdConfig, SpiConfig, extcomin_task_owned,
};
use embassy_silabs::gpio::{Level, Output};
use embassy_silabs::peripherals::EUSART1;
use embassy_time::Timer;
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text, TextStyleBuilder},
};
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

const SPI_FREQ: u32 = 1_100_000;

fn draw_ble_status<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let text_style = TextStyleBuilder::new().alignment(Alignment::Center).build();

    Text::with_text_style("Embassy BLE", Point::new(64, 20), style, text_style).draw(display)?;

    let addr = ble_runtime::read_ble_addr();
    let mut mac: String<20> = String::new();
    core::fmt::write(
        &mut mac,
        format_args!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            addr[5], addr[4], addr[3], addr[2], addr[1], addr[0],
        ),
    )
    .ok();
    Text::with_text_style(&mac, Point::new(64, 40), style, text_style).draw(display)?;

    let adv4 = ble_runtime::ADV_SETUP.load(Ordering::Relaxed);
    let mut status: String<24> = String::new();
    core::fmt::write(&mut status, format_args!("adv4={:#010x}", adv4)).ok();
    Text::with_text_style(&status, Point::new(64, 60), style, text_style).draw(display)?;

    Text::with_text_style("advertising", Point::new(64, 85), style, text_style).draw(display)?;

    Ok(())
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_silabs::init();
    info!("ble empty boot tag={}", env!("BLE_EMPTY_BUILD_TAG"));

    let (board, remaining) = Board::new(p);

    // SDK clock/GPIO setup must finish before EUSART1 display init.
    ble_runtime::init_stack();

    let mut spi_config = SpiConfig::default();
    spi_config.bitrate = SPI_FREQ;
    spi_config.reverse_bits = true;
    //spi_config.ref_freq_hz = Some(ble_runtime::display_spi_ref_hz());
    //info!("display spi ref={} Hz", spi_config.ref_freq_hz.unwrap());

    // Configure MemLcd with auto EXTCOMIN toggling enabled
    let lcd_config = MemLcdConfig::default();

    let display = MemLcd::new_without_extcomin(
        remaining.eusart1,
        board.display_clk,
        board.display_mosi,
        board.display_cs,
        board.display_enable,
        spi_config,
        lcd_config,
    );

    let disp_extcomin = Output::new(board.display_extcomin, Level::Low);

    ble_runtime::spawn_ble_pump(&spawner);
    spawner.spawn(unwrap!(extcomin_task_owned(disp_extcomin, 60)));
    spawner.spawn(unwrap!(display_task(display)));
}

#[embassy_executor::task]
async fn display_task(mut display: MemLcd<'static, EUSART1>) {
    info!("display task start");

    display.power_on();
    Timer::after_millis(100).await;
    display.clear_hw();
    Timer::after_millis(100).await;

    loop {
        display.clear_buffer();
        draw_ble_status(&mut display).unwrap();
        display.flush_display();
        Timer::after_millis(500).await;
    }
}
