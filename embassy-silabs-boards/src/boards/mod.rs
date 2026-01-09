//! Board-specific configurations.
//!
//! Each board module defines pin assignments and peripheral configurations
//! for a specific Silabs development board.

#[cfg(feature = "brd4186c")]
pub mod brd4186c;

#[cfg(feature = "brd2601b")]
pub mod brd2601b;
