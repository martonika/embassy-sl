use embassy_silabs::ble;

use crate::delay;

unsafe extern "C" {
    fn silabs_ble_scheduler_set_enabled(enabled: u8);
    fn silabs_ble_schedule_allow_real(allow: u8);
    fn silabs_ble_try_real_schedule_once();
    fn silabs_ble_startup_ll_pump();
    fn silabs_ble_post_stack_init_pump_step();
    fn silabs_ble_post_stack_init_pump_finish();
    fn silabs_bgapi_system_boot_seen() -> u8;
    fn silabs_ble_force_shm_link();
}

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
}

pub fn pump_loop<F>(mut on_tick: F)
where
    F: FnMut(),
{
    unsafe {
        silabs_ble_scheduler_set_enabled(1);
        // Deferred adv queues the task with schedule gated. Run one real
        // ScheduleProcess while ll time is still before task_start, then leave
        // scheduling enabled for steady-state advertising.
        silabs_ble_schedule_allow_real(0);
        silabs_ble_try_real_schedule_once();
        silabs_ble_schedule_allow_real(1);
        silabs_ble_startup_ll_pump();
    }
    // Reference bt_soc_empty runs sl_bt_step() in a tight superloop with no delay.
    // Pump aggressively for the first ~2 s after init while adv enable propagates.
    let mut tight_steps: u32 = 0;
    loop {
        ble::step();
        on_tick();
        tight_steps = tight_steps.saturating_add(1);
        if tight_steps < 400 {
            continue;
        }
        delay::delay_ms_blocking(5);
    }
}
