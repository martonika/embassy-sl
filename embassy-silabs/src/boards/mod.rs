//! Board-specific configurations.
//!
//! Each board module defines pin assignments and peripheral configurations
//! for a specific Silabs development board.
//!
//! ## Supported Boards
//!
//! - `brd4186c` - xG24 Dev Kit (EFR32MG24B210F1536IM48)
//! - `brd2601b` - Thunderboard Sense 2 (EFR32MG24)
//!
//! ## Usage
//!
//! Enable the appropriate board feature in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! embassy-silabs = { version = "0.1", features = ["board-brd4186c"] }
//! ```
//!
//! Then use the board configuration in your application:
//!
//! ```rust,ignore
//! use embassy_silabs::boards::brd4186c::Board;
//!
//! #[embassy_executor::main]
//! async fn main(_spawner: Spawner) {
//!     let p = embassy_silabs::init();
//!     let (board, remaining) = Board::new(p);
//!
//!     // Use board.led0, board.btn0, etc.
//! }
//! ```

#[cfg(feature = "board-brd4186c")]
pub mod brd4186c;

#[cfg(feature = "board-brd2601b")]
pub mod brd2601b;
