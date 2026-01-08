#![no_std]
#![no_main]

use embassy_executor::Spawner;

use defmt::*;
use embassy_silabs::drivers::display::memlcd::{Config as SpiConfig, MemLcdSpi};
use embassy_silabs::gpio::*;
use embassy_time::{Duration, Ticker, Timer};
use {defmt_rtt as _, panic_probe as _}; // global logger

// LS013B7DH03 Memory LCD constants
const DISPLAY_WIDTH: usize = 128;
const DISPLAY_HEIGHT: usize = 128;
const SPI_FREQ: u32 = 1_100_000; // 1.1 MHz max

// Display commands
const CMD_UPDATE: u8 = 0x01;
const CMD_ALL_CLEAR: u8 = 0x04;

// SCS timing (from datasheet)
const SCS_SETUP_US: u64 = 6;
const SCS_HOLD_US: u64 = 2;

/// Memory LCD display driver
struct MemLcd<'d, T: embassy_silabs::drivers::display::memlcd::Instance> {
    spi: MemLcdSpi<'d, T>,
    cs: Output<'d>,
    enable: Output<'d>,
}

impl<'d, T: embassy_silabs::drivers::display::memlcd::Instance> MemLcd<'d, T> {
    /// Power on the display by setting DISP_ENABLE high
    pub fn power_on(&mut self) {
        self.enable.set_high();
    }

    /// Power off the display (give control back to board controller)
    #[allow(dead_code)]
    pub fn power_off(&mut self) {
        self.enable.set_low();
    }

    /// Clear the entire display
    pub async fn clear(&mut self) {
        // Assert CS
        self.cs.set_high();

        // SCS setup time
        Timer::after_micros(SCS_SETUP_US).await;

        // Send clear command (2 bytes: command + dummy)
        let cmd: [u8; 2] = [CMD_ALL_CLEAR, 0x00];
        self.spi.tx(&cmd).unwrap();
        self.spi.wait();

        // SCS hold time
        Timer::after_micros(SCS_HOLD_US).await;

        // Deassert CS
        self.cs.set_low();

        // Flush any RX garbage
        self.spi.rx_flush();
    }

    /// Draw pixel data to the display starting at the specified row
    ///
    /// Each row is 128 bits (16 bytes) of pixel data.
    /// Data format: 1 bit per pixel, LSB first within each byte.
    pub async fn draw(&mut self, data: &[u8], row_start: u8, row_count: u8) {
        let row_len = DISPLAY_WIDTH / 8; // 16 bytes per row

        // Assert CS
        self.cs.set_high();

        // SCS setup time
        Timer::after_micros(SCS_SETUP_US).await;

        // Line addresses are 1-indexed
        let mut line_addr = row_start + 1;

        // Send update command with first line address
        // Format: [CMD_UPDATE, line_address]
        let cmd: [u8; 2] = [CMD_UPDATE, line_addr];
        self.spi.tx(&cmd).unwrap();

        for i in 0..row_count {
            // Send pixel data for this line
            let start = (i as usize) * row_len;
            let end = start + row_len;
            if end <= data.len() {
                self.spi.tx(&data[start..end]).unwrap();
            }

            // Send dummy data or next line address
            if i == row_count - 1 {
                // Last line: send dummy bytes
                let dummy: [u8; 2] = [0xFF, 0xFF];
                self.spi.tx(&dummy).unwrap();
            } else {
                // Next line address
                line_addr += 1;
                let next_line: [u8; 2] = [0xFF, line_addr];
                self.spi.tx(&next_line).unwrap();
            }
        }

        self.spi.wait();

        // SCS hold time
        Timer::after_micros(SCS_HOLD_US).await;

        // Deassert CS
        self.cs.set_low();

        // Flush RX
        self.spi.rx_flush();
    }

    /// Draw a test pattern (checkerboard)
    pub async fn draw_checkerboard(&mut self) {
        // Create checkerboard pattern: alternating 8x8 blocks
        let mut framebuf = [0u8; DISPLAY_WIDTH / 8 * DISPLAY_HEIGHT];

        for row in 0..DISPLAY_HEIGHT {
            for col_byte in 0..(DISPLAY_WIDTH / 8) {
                let block_row = row / 8;
                let block_col = col_byte;

                // Alternate pattern based on block position
                let pattern = if (block_row + block_col) % 2 == 0 {
                    0xFF // White block
                } else {
                    0x00 // Black block
                };

                framebuf[row * (DISPLAY_WIDTH / 8) + col_byte] = pattern;
            }
        }

        self.draw(&framebuf, 0, DISPLAY_HEIGHT as u8).await;
    }

    /// Fill the entire display with a solid color (true = white, false = black)
    pub async fn fill(&mut self, white: bool) {
        let pattern = if white { 0xFF } else { 0x00 };
        let framebuf = [pattern; DISPLAY_WIDTH / 8 * DISPLAY_HEIGHT];
        self.draw(&framebuf, 0, DISPLAY_HEIGHT as u8).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Initialize peripherals");
    let p = embassy_silabs::init();

    // LED outputs for status indication
    let led0 = Output::new(p.PB_02, Level::Low); // BRD4187C + WPK
    let led1 = Output::new(p.PB_04, Level::Low); // BRD4187C + WPK

    // Memory LCD pins (BRD4187C)
    // DISP_ENABLE / PC09 - Controls display ownership
    // DISP_EXTCOMIN / PC06 - COM inversion signal (must toggle at ~60Hz)
    // DISP_SI / PC01 - SPI MOSI
    // DISP_SCLK / PC03 - SPI Clock
    // DISP_SCS / PC08 - Chip Select (directly controlled, active high)

    let disp_enable = Output::new(p.PC_09, Level::Low);
    let disp_extcomin = Output::new(p.PC_06, Level::Low);
    let disp_cs = Output::new(p.PC_08, Level::Low);

    // Configure SPI for memory LCD
    let mut spi_config = SpiConfig::default();
    spi_config.bitrate = SPI_FREQ;
    // The LS013B7DH03 requires LSB-first bit order within each byte
    spi_config.reverse_bits = true;

    // Create SPI driver using EUSART1 (per board config sl_memlcd_eusart_config.h)
    // DISP_SI (MOSI) = PC01, DISP_SCLK = PC03
    let spi = MemLcdSpi::new(p.EUSART1, p.PC_03, p.PC_01, spi_config);

    // Create the display driver
    let display = MemLcd {
        spi,
        cs: disp_cs,
        enable: disp_enable,
    };

    // Spawn tasks
    unwrap!(spawner.spawn(blink_led(led0)));
    unwrap!(spawner.spawn(extcomin_toggle(disp_extcomin)));
    unwrap!(spawner.spawn(display_demo(display, led1)));
}

/// Blink LED0 as a heartbeat indicator
#[embassy_executor::task]
async fn blink_led(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after_millis(500).await;
    }
}

/// Toggle EXTCOMIN at ~60Hz to prevent display static buildup
#[embassy_executor::task]
async fn extcomin_toggle(mut extcomin: Output<'static>) {
    // 60Hz = toggle every ~8.3ms (we toggle twice per cycle)
    let mut ticker = Ticker::every(Duration::from_hz(120));

    loop {
        extcomin.toggle();
        ticker.next().await;
    }
}

/// Display demo: cycle through patterns
#[embassy_executor::task]
async fn display_demo(mut display: MemLcd<'static, embassy_silabs::peripherals::EUSART1>, mut led: Output<'static>) {
    info!("Starting display demo");

    // Power on the display
    display.power_on();
    Timer::after_millis(100).await; // Delay for power stabilization

    // Clear the display first
    info!("Clearing display...");
    display.clear().await;
    led.toggle();
    Timer::after_millis(1000).await;

    loop {
        // Show white fill
        info!("Fill white");
        display.fill(true).await;
        led.toggle();
        Timer::after_millis(2000).await;

        // Show black fill
        info!("Fill black");
        display.fill(false).await;
        led.toggle();
        Timer::after_millis(2000).await;

        // Show checkerboard
        info!("Checkerboard pattern");
        display.draw_checkerboard().await;
        led.toggle();
        Timer::after_millis(2000).await;

        // Clear
        info!("Clear");
        display.clear().await;
        led.toggle();
        Timer::after_millis(2000).await;
    }
}
