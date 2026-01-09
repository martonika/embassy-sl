//! Si7021 Temperature and Humidity Sensor Driver
//!
//! This driver supports the Silicon Labs Si7021 relative humidity and temperature
//! sensor, commonly found on Silicon Labs development kits.
//!
//! # Features
//! - Async and blocking I2C support via embedded-hal traits
//! - Hold Master Mode for measurements (clock stretching)
//! - Accurate humidity and temperature readings
//!
//! # Example
//! ```no_run
//! use embassy_silabs::drivers::sensor::Si7021;
//!
//! async fn read_sensor<I2C>(i2c: I2C) -> Result<(), I2C::Error>
//! where
//!     I2C: embedded_hal_async::i2c::I2c,
//! {
//!     let mut sensor = Si7021::new(i2c);
//!     
//!     let humidity = sensor.read_humidity().await?;
//!     let temperature = sensor.read_temperature().await?;
//!     
//!     Ok(())
//! }
//! ```

/// Default I2C address for the Si7021 sensor
pub const DEFAULT_ADDRESS: u8 = 0x40;

/// Si7021 I2C Commands
#[allow(dead_code)]
mod commands {
    /// Measure Relative Humidity, Hold Master Mode
    pub const MEASURE_RH_HOLD: u8 = 0xE5;
    /// Measure Relative Humidity, No Hold Master Mode
    pub const MEASURE_RH_NOHOLD: u8 = 0xF5;
    /// Measure Temperature, Hold Master Mode
    pub const MEASURE_TEMP_HOLD: u8 = 0xE3;
    /// Measure Temperature, No Hold Master Mode
    pub const MEASURE_TEMP_NOHOLD: u8 = 0xF3;
    /// Read Temperature from Previous RH Measurement
    pub const READ_TEMP_FROM_RH: u8 = 0xE0;
    /// Reset
    pub const RESET: u8 = 0xFE;
    /// Read Electronic ID 1st Byte
    pub const READ_ID1: [u8; 2] = [0xFA, 0x0F];
    /// Read Electronic ID 2nd Byte
    pub const READ_ID2: [u8; 2] = [0xFC, 0xC9];
    /// Read Firmware Revision
    pub const READ_FW_REV: [u8; 2] = [0x84, 0xB8];
}

/// Measurement result containing both humidity and temperature
#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    /// Relative humidity in centi-percent (e.g., 4523 = 45.23%)
    pub humidity_centi_percent: i32,
    /// Temperature in centi-degrees Celsius (e.g., 2534 = 25.34°C)
    pub temperature_centi_c: i32,
}

impl Measurement {
    /// Get humidity as a floating point percentage
    pub fn humidity_percent(&self) -> f32 {
        self.humidity_centi_percent as f32 / 100.0
    }

    /// Get temperature as floating point degrees Celsius
    pub fn temperature_celsius(&self) -> f32 {
        self.temperature_centi_c as f32 / 100.0
    }
}

/// Si7021 Temperature and Humidity Sensor Driver
///
/// This driver uses the Hold Master Mode for measurements, where the sensor
/// holds the SCL line low during conversion. This is the most reliable method
/// and matches the Silicon Labs reference implementation.
pub struct Si7021<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> Si7021<I2C> {
    /// Create a new Si7021 driver with the default I2C address (0x40)
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            address: DEFAULT_ADDRESS,
        }
    }

    /// Create a new Si7021 driver with a custom I2C address
    pub fn new_with_address(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    /// Release the I2C bus
    pub fn release(self) -> I2C {
        self.i2c
    }

    /// Convert raw humidity data to centi-percent
    /// Formula from Si7021 datasheet: RH% = ((125 * raw) / 65536) - 6
    /// Returns result in centi-percent for integer math
    fn convert_humidity(raw: u16) -> i32 {
        // RH% = ((raw * 15625) >> 13) - 6000, result is milli-percent
        let raw = raw as u32;
        let milli_percent = ((raw * 15625) >> 13).saturating_sub(6000);
        (milli_percent / 10) as i32
    }

    /// Convert raw temperature data to centi-degrees Celsius
    /// Formula from Si7021 datasheet: T°C = ((175.72 * raw) / 65536) - 46.85
    /// Returns result in centi-degrees for integer math
    fn convert_temperature(raw: u16) -> i32 {
        // T°C = ((raw * 21965) >> 13) - 46850, result is milli-degrees
        let raw = raw as u32;
        let milli_c = ((raw * 21965) >> 13) as i32 - 46850;
        milli_c / 10
    }
}

// ============================================================================
// Async Implementation
// ============================================================================

impl<I2C, E> Si7021<I2C>
where
    I2C: embedded_hal_async::i2c::I2c<Error = E>,
{
    /// Measure relative humidity using Hold Master Mode
    ///
    /// Returns humidity in centi-percent (e.g., 4523 = 45.23%)
    pub async fn read_humidity(&mut self) -> Result<i32, E> {
        let mut data = [0u8; 2];
        self.i2c
            .write_read(self.address, &[commands::MEASURE_RH_HOLD], &mut data)
            .await?;

        let raw = u16::from_be_bytes([data[0], data[1] & 0xFC]);
        Ok(Self::convert_humidity(raw))
    }

    /// Measure temperature using Hold Master Mode
    ///
    /// Returns temperature in centi-degrees Celsius (e.g., 2534 = 25.34°C)
    pub async fn read_temperature(&mut self) -> Result<i32, E> {
        let mut data = [0u8; 2];
        self.i2c
            .write_read(self.address, &[commands::MEASURE_TEMP_HOLD], &mut data)
            .await?;

        let raw = u16::from_be_bytes([data[0], data[1] & 0xFC]);
        Ok(Self::convert_temperature(raw))
    }

    /// Read temperature from the previous humidity measurement
    ///
    /// This is more efficient if you need both values, as the Si7021
    /// measures temperature during every humidity measurement.
    ///
    /// Returns temperature in centi-degrees Celsius (e.g., 2534 = 25.34°C)
    pub async fn read_temperature_from_humidity(&mut self) -> Result<i32, E> {
        let mut data = [0u8; 2];
        self.i2c
            .write_read(self.address, &[commands::READ_TEMP_FROM_RH], &mut data)
            .await?;

        let raw = u16::from_be_bytes([data[0], data[1] & 0xFC]);
        Ok(Self::convert_temperature(raw))
    }

    /// Measure both humidity and temperature in a single operation
    ///
    /// This first measures humidity (which also captures temperature internally),
    /// then reads the temperature from that measurement. More efficient than
    /// calling read_humidity() and read_temperature() separately.
    pub async fn measure(&mut self) -> Result<Measurement, E> {
        let humidity = self.read_humidity().await?;
        let temperature = self.read_temperature_from_humidity().await?;

        Ok(Measurement {
            humidity_centi_percent: humidity,
            temperature_centi_c: temperature,
        })
    }

    /// Send a soft reset command to the sensor
    ///
    /// The sensor will reset to default settings. Allow 15ms after reset
    /// before sending commands.
    pub async fn reset(&mut self) -> Result<(), E> {
        self.i2c.write(self.address, &[commands::RESET]).await
    }
}

// ============================================================================
// Blocking Implementation
// ============================================================================

impl<I2C, E> Si7021<I2C>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    /// Measure relative humidity using Hold Master Mode (blocking)
    ///
    /// Returns humidity in centi-percent (e.g., 4523 = 45.23%)
    pub fn read_humidity_blocking(&mut self) -> Result<i32, E> {
        let mut data = [0u8; 2];
        self.i2c
            .write_read(self.address, &[commands::MEASURE_RH_HOLD], &mut data)?;

        let raw = u16::from_be_bytes([data[0], data[1] & 0xFC]);
        Ok(Self::convert_humidity(raw))
    }

    /// Measure temperature using Hold Master Mode (blocking)
    ///
    /// Returns temperature in centi-degrees Celsius (e.g., 2534 = 25.34°C)
    pub fn read_temperature_blocking(&mut self) -> Result<i32, E> {
        let mut data = [0u8; 2];
        self.i2c
            .write_read(self.address, &[commands::MEASURE_TEMP_HOLD], &mut data)?;

        let raw = u16::from_be_bytes([data[0], data[1] & 0xFC]);
        Ok(Self::convert_temperature(raw))
    }

    /// Read temperature from the previous humidity measurement (blocking)
    ///
    /// Returns temperature in centi-degrees Celsius (e.g., 2534 = 25.34°C)
    pub fn read_temperature_from_humidity_blocking(&mut self) -> Result<i32, E> {
        let mut data = [0u8; 2];
        self.i2c
            .write_read(self.address, &[commands::READ_TEMP_FROM_RH], &mut data)?;

        let raw = u16::from_be_bytes([data[0], data[1] & 0xFC]);
        Ok(Self::convert_temperature(raw))
    }

    /// Measure both humidity and temperature in a single operation (blocking)
    pub fn measure_blocking(&mut self) -> Result<Measurement, E> {
        let humidity = self.read_humidity_blocking()?;
        let temperature = self.read_temperature_from_humidity_blocking()?;

        Ok(Measurement {
            humidity_centi_percent: humidity,
            temperature_centi_c: temperature,
        })
    }

    /// Send a soft reset command to the sensor (blocking)
    pub fn reset_blocking(&mut self) -> Result<(), E> {
        self.i2c.write(self.address, &[commands::RESET])
    }
}
