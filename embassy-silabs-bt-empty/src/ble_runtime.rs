use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use defmt::info;
use embassy_executor::Spawner;
use embassy_silabs::ble;
use embassy_time::Timer;

unsafe extern "C" {
    fn silabs_em01grpcclk_hz() -> u32;
    fn silabs_ble_scheduler_set_enabled(enabled: u8);
    fn silabs_ble_schedule_allow_real(allow: u8);
    fn silabs_ble_try_real_schedule_once();
    fn silabs_ble_startup_ll_pump();
    fn silabs_ble_post_stack_init_pump_step();
    fn silabs_ble_post_stack_init_pump_finish();
    fn silabs_bgapi_system_boot_seen() -> u8;
    fn silabs_ble_force_shm_link();
    fn silabs_bgapi_ble_adv_setup_status() -> u32;
    fn silabs_bgapi_ble_identity_address_read(out: *mut u8);
}

/// BLE identity address (little-endian), updated after stack init.
pub static BLE_ADDR: [AtomicU8; 6] = [
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
    AtomicU8::new(0),
];

/// Deferred-adv / setup status word from C glue (`adv4` in boot logs).
pub static ADV_SETUP: AtomicU32 = AtomicU32::new(0);

pub fn read_ble_addr() -> [u8; 6] {
    [
        BLE_ADDR[0].load(Ordering::Relaxed),
        BLE_ADDR[1].load(Ordering::Relaxed),
        BLE_ADDR[2].load(Ordering::Relaxed),
        BLE_ADDR[3].load(Ordering::Relaxed),
        BLE_ADDR[4].load(Ordering::Relaxed),
        BLE_ADDR[5].load(Ordering::Relaxed),
    ]
}

fn log_identity_and_adv() {
    let mut addr = [0u8; 6];
    unsafe { silabs_bgapi_ble_identity_address_read(addr.as_mut_ptr()) };
    let adv4 = unsafe { silabs_bgapi_ble_adv_setup_status() };
    for (i, b) in addr.iter().enumerate() {
        BLE_ADDR[i].store(*b, Ordering::Relaxed);
    }
    ADV_SETUP.store(adv4, Ordering::Relaxed);
    info!(
        "ble identity {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} adv4={:#010x}",
        addr[5], addr[4], addr[3], addr[2], addr[1], addr[0], adv4
    );
}

/// Measured EM01GRPCCLK frequency for display SPI SDIV (via emlib).
pub fn display_spi_ref_hz() -> u32 {
    unsafe { silabs_em01grpcclk_hz() }
}

/// Full BLE stack bring-up. Call once from `main` before display init.
pub fn init_stack() {
    for step in 1..=ble::INIT_STEPS {
        ble::init_step(step);
    }
    ble::mark_initialized();

    const MAX_PUMP: u32 = 64;
    for i in 0..MAX_PUMP {
        unsafe { silabs_ble_post_stack_init_pump_step() };
        if unsafe { silabs_bgapi_system_boot_seen() } != 0 && i >= 3 {
            break;
        }
    }
    unsafe { silabs_ble_post_stack_init_pump_finish() };
    unsafe { silabs_ble_force_shm_link() };
    log_identity_and_adv();
}

fn pump_startup() {
    unsafe {
        silabs_ble_scheduler_set_enabled(1);
        silabs_ble_schedule_allow_real(0);
        silabs_ble_try_real_schedule_once();
        silabs_ble_schedule_allow_real(1);
        silabs_ble_startup_ll_pump();
    }
}

async fn pump_run() {
    pump_startup();
    loop {
        ble::step();
        Timer::after_millis(5).await;
    }
}

/// Spawn the non-blocking BLE stack pump (5 ms period). Stack must already be initialized.
pub fn spawn_ble_pump(spawner: &Spawner) {
    spawner.spawn(ble_pump_task().expect("ble pump task"));
}

#[embassy_executor::task]
async fn ble_pump_task() {
    info!("ble pump task start");
    pump_run().await;
}
