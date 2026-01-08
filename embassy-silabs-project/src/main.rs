#![no_std]
#![no_main]

use embassy_executor::Spawner;

use defmt::*;
use embassy_silabs::drivers::display::memlcd::{
    extcomin_task_owned, MemLcd, MemLcdConfig, SpiConfig,
};
use embassy_silabs::gpio::*;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _}; // global logger

// embedded-graphics imports
use embedded_graphics::{
    geometry::Point,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Alignment, Text, TextStyleBuilder},
    Drawable,
};

// SPI frequency for memory LCD (1.1 MHz max)
const SPI_FREQ: u32 = 1_100_000;

/// Draw the static text on the display
fn draw_text<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let text_style = TextStyleBuilder::new().alignment(Alignment::Center).build();

    // "embassy-silabs" centered
    Text::with_text_style("embassy-silabs", Point::new(64, 50), style, text_style).draw(display)?;

    // "hello from Rust" centered below
    Text::with_text_style("hello from Rust", Point::new(64, 65), style, text_style).draw(display)?;

    Ok(())
}

/// Draw a graphical progress wheel (spinner) at the specified frame
fn draw_spinner<D>(display: &mut D, frame: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let center = Point::new(64, 95);
    let radius = 10i32;

    // Draw outer circle
    Circle::with_center(center, (radius * 2) as u32)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;

    // Draw rotating indicator - 8 positions around the circle
    let num_positions = 8u32;
    let current_pos = frame % num_positions;

    for i in 0..num_positions {
        let angle = (i as f32) * 2.0 * 3.14159 / (num_positions as f32);
        let cos_a = libm::cosf(angle);
        let sin_a = libm::sinf(angle);

        let dot_x = center.x + ((radius - 3) as f32 * cos_a) as i32;
        let dot_y = center.y + ((radius - 3) as f32 * sin_a) as i32;

        // Draw larger dot for current position, smaller for others
        let dot_size = if i == current_pos { 4 } else { 2 };

        // Only draw current and adjacent positions for spinning effect
        let distance = ((i as i32 - current_pos as i32).abs()).min(
            num_positions as i32 - (i as i32 - current_pos as i32).abs(),
        );

        if distance <= 3 {
            Rectangle::new(
                Point::new(dot_x - dot_size / 2, dot_y - dot_size / 2),
                embedded_graphics::geometry::Size::new(dot_size as u32, dot_size as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(display)?;
        }
    }

    Ok(())
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Initialize peripherals");
    let p = embassy_silabs::init();

    // LED outputs for status indication
    let led0 = Output::new(p.PB_02, Level::Low); // BRD4187C + WPK

    // Configure SPI for memory LCD
    let mut spi_config = SpiConfig::default();
    spi_config.bitrate = SPI_FREQ;
    // The LS013B7DH03 requires LSB-first bit order within each byte
    spi_config.reverse_bits = true;

    // Configure MemLcd with auto EXTCOMIN toggling enabled
    let lcd_config = MemLcdConfig::default();

    // Create the display driver using the HAL MemLcd
    let display = MemLcd::new_without_extcomin(
        p.EUSART1,
        p.PC_03, // SCLK
        p.PC_01, // MOSI
        p.PC_08, // CS
        p.PC_09, // ENABLE
        spi_config,
        lcd_config,
    );

    // Create EXTCOMIN pin for the background task
    let disp_extcomin = Output::new(p.PC_06, Level::Low);

    // Spawn tasks
    unwrap!(spawner.spawn(blink_led(led0)));
    unwrap!(spawner.spawn(extcomin_task_owned(disp_extcomin, 60)));
    unwrap!(spawner.spawn(display_task(display)));
}

/// Blink LED0 as a heartbeat indicator
#[embassy_executor::task]
async fn blink_led(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after_millis(500).await;
    }
}

/// Display task: show text with animated spinner
#[embassy_executor::task]
async fn display_task(mut display: MemLcd<'static, embassy_silabs::peripherals::EUSART1>) {
    info!("Starting display");

    // Power on the display
    display.power_on();
    Timer::after_millis(100).await;

    // Clear the display first
    display.clear_hw();
    Timer::after_millis(100).await;

    let mut frame: u32 = 0;

    loop {
        // Redraw everything each frame
        display.clear_buffer();
        draw_text(&mut display).unwrap();
        draw_spinner(&mut display, frame).unwrap();
        display.flush_display();

        // Advance animation
        frame = frame.wrapping_add(1);

        // Update at ~8 fps for smooth animation
        Timer::after_millis(125).await;
    }
}
