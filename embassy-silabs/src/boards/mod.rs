//! Board-specific configurations.
//!
//! Each board feature selects that board's pinout and the matching MG24 PA / RAIL
//! sequencer profile. Enable exactly one board feature.
//!
//! ## Supported Boards
//!
//! | Feature | Board | Radio PA | Part |
//! |---------|-------|----------|------|
//! | `board-brd4186c` | BRD4186C | +10 dBm | EFR32MG24B210F1536IM48 |
//! | `board-brd4187c` | BRD4187C | +20 dBm | EFR32MG24B220F1536IM48 |
//! | `board-brd2601b` | Thunderboard Sense 2 | (see board module) | EFR32MG24 |
//!
//! ## Usage
//!
//! ```toml
//! embassy-silabs = { version = "0.1", features = ["board-brd4186c"] }
//! ```
//!
//! ```rust,ignore
//! use embassy_silabs::boards::{Board, BOARD_NAME, MG24_PART, RADIO_PA_DBM};
//!
//! let (board, remaining) = Board::new(embassy_silabs::init());
//! ```

#[cfg(all(feature = "board-brd4186c", feature = "board-brd4187c"))]
compile_error!("enable only one of `board-brd4186c` or `board-brd4187c`");

#[cfg(feature = "board-brd4186c")]
pub mod brd4186c;

#[cfg(feature = "board-brd4187c")]
pub mod brd4187c;

#[cfg(feature = "board-brd2601b")]
pub mod brd2601b;

#[cfg(feature = "board-brd4186c")]
pub use brd4186c::{Board, LedPolarity, RemainingPeripherals};

#[cfg(feature = "board-brd4187c")]
pub use brd4187c::{Board, LedPolarity, RemainingPeripherals};

#[cfg(feature = "board-brd4186c")]
pub use brd4186c::{BOARD_NAME, MG24_PART, RADIO_PA_DBM};

#[cfg(feature = "board-brd4187c")]
pub use brd4187c::{BOARD_NAME, MG24_PART, RADIO_PA_DBM};
