#![no_std]
#![no_main]

use embassy_executor::Spawner;

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use defmt::*;
use embassy_silabs::drivers::display::memlcd::{
    extcomin_task_owned, MemLcd, MemLcdConfig, SpiConfig,
};
use embassy_silabs::drivers::sensor::Si7021;
use embassy_silabs::gpio::*;
use embassy_silabs::i2c::{self, I2c};
use embassy_silabs::{bind_interrupts, peripherals};
use embassy_time::Timer;
use heapless::String;
use {defmt_rtt as _, panic_probe as _}; // global logger

// Simple monotonic counter for defmt timestamps (safe before time driver init)
static LOG_COUNT: AtomicU32 = AtomicU32::new(0);
defmt::timestamp!("{=u32}", LOG_COUNT.fetch_add(1, Ordering::Relaxed));

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

// Bind I2C1 interrupt to the I2C interrupt handler
bind_interrupts!(struct Irqs {
    I2C1 => i2c::InterruptHandler<peripherals::I2C1>;
});

// Shared sensor readings (temperature in centi-degrees, humidity in centi-percent)
// Using AtomicI32 for lock-free access between tasks
static TEMPERATURE_CENTI_C: AtomicI32 = AtomicI32::new(0);
static HUMIDITY_CENTI_PERCENT: AtomicI32 = AtomicI32::new(0);
static SENSOR_VALID: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Draw the static text and sensor readings on the display
fn draw_text<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    let text_style = TextStyleBuilder::new().alignment(Alignment::Center).build();

    // "embassy-silabs" centered at top
    Text::with_text_style("embassy-silabs", Point::new(64, 20), style, text_style).draw(display)?;

    // "hello from Rust" centered below
    Text::with_text_style("hello from Rust", Point::new(64, 35), style, text_style).draw(display)?;

    // Display sensor readings if available
    let sensor_valid = SENSOR_VALID.load(Ordering::Relaxed);
    if sensor_valid {
        let temp_centi = TEMPERATURE_CENTI_C.load(Ordering::Relaxed);
        let humidity_centi = HUMIDITY_CENTI_PERCENT.load(Ordering::Relaxed);

        // Format temperature (e.g., "Temp: 25.3 C")
        let temp_whole = temp_centi / 100;
        let temp_frac = (temp_centi % 100).abs() / 10;
        let mut temp_str: String<20> = String::new();
        if temp_centi < 0 && temp_whole == 0 {
            core::fmt::write(&mut temp_str, format_args!("Temp: -{}.{} C", temp_whole.abs(), temp_frac)).ok();
        } else {
            core::fmt::write(&mut temp_str, format_args!("Temp: {}.{} C", temp_whole, temp_frac)).ok();
        }
        Text::with_text_style(&temp_str, Point::new(64, 55), style, text_style).draw(display)?;

        // Format humidity (e.g., "RH: 45.2 %")
        let rh_whole = humidity_centi / 100;
        let rh_frac = (humidity_centi % 100) / 10;
        let mut rh_str: String<20> = String::new();
        core::fmt::write(&mut rh_str, format_args!("RH: {}.{} %", rh_whole, rh_frac)).ok();
        Text::with_text_style(&rh_str, Point::new(64, 70), style, text_style).draw(display)?;
    } else {
        // Sensor not yet ready
        Text::with_text_style("Sensor: --", Point::new(64, 55), style, text_style).draw(display)?;
    }

    Ok(())
}

/// Draw a graphical progress wheel (spinner) at the specified frame
fn draw_spinner<D>(display: &mut D, frame: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let center = Point::new(64, 110);
    let radius = 10i32;

    // Draw outer circle
    Circle::with_center(center, (radius * 2) as u32)
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)?;

    // Draw rotating indicator - 8 positions around the circle
    let num_positions = 8u32;
    let current_pos = frame % num_positions;

    for i in 0..num_positions {
        let angle = (i as f32) * 2.0 * core::f32::consts::PI / (num_positions as f32);
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

    // Enable the RHT sensor (Si7021) - requires PD_03 HIGH per board config
    let _sensor_enable = Output::new(p.PD_03, Level::High);

    // Configure I2C1 for the Si7021 sensor
    // SCL: PC_05, SDA: PC_07 (from board config)
    let i2c_config = i2c::Config::default();
    let i2c = I2c::new(
        p.I2C1,
        p.PC_05, // SCL
        p.PC_07, // SDA
        Irqs,
        i2c_config,
    );

    // Create the Si7021 sensor driver
    let sensor = Si7021::new(i2c);

    // Spawn tasks
    unwrap!(spawner.spawn(blink_led(led0)));
    unwrap!(spawner.spawn(extcomin_task_owned(disp_extcomin, 60)));
    unwrap!(spawner.spawn(sensor_task(sensor)));
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

/// Sensor task: periodically read temperature and humidity from Si7021
#[embassy_executor::task]
async fn sensor_task(mut sensor: Si7021<I2c<'static, peripherals::I2C1>>) {
    info!("Starting sensor task");

    // Give the sensor some time to initialize after power-on
    Timer::after_millis(100).await;

    loop {
        match sensor.measure().await {
            Ok(measurement) => {
                HUMIDITY_CENTI_PERCENT.store(measurement.humidity_centi_percent, Ordering::Relaxed);
                TEMPERATURE_CENTI_C.store(measurement.temperature_centi_c, Ordering::Relaxed);
                SENSOR_VALID.store(true, Ordering::Relaxed);

                info!(
                    "Humidity: {}.{}%, Temperature: {}.{} C",
                    measurement.humidity_centi_percent / 100,
                    (measurement.humidity_centi_percent % 100) / 10,
                    measurement.temperature_centi_c / 100,
                    (measurement.temperature_centi_c % 100).abs() / 10
                );
            }
            Err(e) => {
                warn!("Failed to read sensor: {:?}", e);
            }
        }

        // Read sensor every second
        Timer::after_millis(1000).await;
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
