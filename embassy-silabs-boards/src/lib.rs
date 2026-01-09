//! # Embassy Silabs Board Support Package
//!
//! This crate provides board-specific configurations for Silabs development boards.
//! Each board module defines the pin assignments for LEDs, buttons, sensors, displays,
//! and other peripherals specific to that board.
//!
//! ## Supported Boards
//!
//! - `brd4186c` - xG24 Dev Kit (EFR32MG24B210F1536IM48)
//! - `brd2601b` - Thunderboard Sense 2 (EFR32MG24)
//!
//! ## Usage
//!
//! Add the appropriate board feature to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! embassy-silabs-boards = { version = "0.1", features = ["brd4186c"] }
//! ```
//!
//! Then use the board configuration in your application:
//!
//! ```rust,ignore
//! use embassy_silabs_boards::brd4186c::Board;
//!
//! #[embassy_executor::main]
//! async fn main(_spawner: Spawner) {
//!     let p = embassy_silabs::init();
//!     let board = Board::new(p);
//!     
//!     // Use board.led0, board.btn0, etc.
//! }
//! ```

#![no_std]

pub mod boards;

// Re-export board modules at the crate root for convenience
#[cfg(feature = "brd4186c")]
pub use boards::brd4186c;

#[cfg(feature = "brd2601b")]
pub use boards::brd2601b;
