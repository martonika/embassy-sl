#![no_std]
#![no_main]

#[path = "../delay.rs"]
mod delay;
#[path = "../ble_app.rs"]
mod ble_app;
#[path = "../ble_runtime.rs"]
mod ble_runtime;

use defmt::*;
use embassy_executor::Spawner;
use embassy_silabs::boards::brd4186c::Board;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("ble empty boot tag={}", env!("BLE_EMPTY_BUILD_TAG"));

    delay::init();
    let p = embassy_silabs::init();

    let (board, _) = Board::new(p);
    board.route_rf_activity_leds();

    ble_runtime::init_stack();
    ble_runtime::pump_loop(|| {});
}
