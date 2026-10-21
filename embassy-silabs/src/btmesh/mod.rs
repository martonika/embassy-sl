//! Bluetooth Mesh (provisionee node on top of the BLE stack).
//!
//! Enable the `btmesh` feature (implies `ble`). Mesh shares the same
//! [`crate::ble::step`] pump as the BLE host.

pub use crate::ble;
