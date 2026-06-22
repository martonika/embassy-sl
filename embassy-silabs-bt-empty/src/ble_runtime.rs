use defmt::*;
use embassy_silabs::ble;

use crate::delay;

#[allow(dead_code)]
unsafe extern "C" {
    fn silabs_bgapi_bt_peek_len() -> u32;
    fn silabs_bgapi_ble_adv_start_pump_done() -> u8;
    fn silabs_ble_step_phase_read() -> u32;
    fn silabs_bgapi_ble_adv_setup_status() -> u32;
    fn silabs_ble_scheduler_set_enabled(enabled: u8);
    fn silabs_ble_schedule_allow_real(allow: u8);
    fn silabs_ble_startup_ll_pump();
    fn silabs_bgapi_hci_adv_enable_calls() -> u32;
    fn silabs_bgapi_hci_adv_enable_on_calls() -> u32;
    fn silabs_bgapi_hci_adv_enable_off_calls() -> u32;
    fn silabs_bgapi_hci_adv_enable_last_num_sets() -> u8;
    fn silabs_bgapi_hci_adv_enable_last_maxevents() -> u8;
    fn silabs_bgapi_hci_adv_enable_last_handle() -> u8;
    fn silabs_bgapi_ll_raise_cb_ptr_last() -> u32;
    fn silabs_bgapi_ll_raise_skipped_null_cb() -> u32;
    fn silabs_bgapi_ll_raise_skipped_invalid_cb() -> u32;
    fn silabs_bgapi_ll_raise_cb_ptr_first_nonzero() -> u32;
    fn silabs_bgapi_ll_raise_cb_compat_ptr_last() -> u32;
    fn silabs_bgapi_ll_raise_cb_host_adapt_ptr_last() -> u32;
    fn silabs_bgapi_ll_raise_cb_original_last() -> u32;
    fn silabs_bgapi_ll_raise_cb_trampoline_installs() -> u32;
    fn silabs_bgapi_ll_raise_cb_trampoline_enter() -> u32;
    fn silabs_bgapi_ll_raise_cb_trampoline_exit() -> u32;
    fn silabs_bgapi_ll_raise_cb_fallback_pendsv_calls() -> u32;
    fn silabs_bgapi_ll_radio_raise_error_calls() -> u32;
    fn silabs_bgapi_ll_radio_raise_error_last() -> u32;
    fn silabs_bgapi_ll_radio_raise_error14_skip_once_calls() -> u32;
    fn silabs_bgapi_ll_radio_raise_error_caller_last() -> u32;
    fn silabs_bgapi_ll_radio_raise_error14_caller_first() -> u32;
    fn silabs_bgapi_ll_radio_raise_error14_caller_last() -> u32;
    fn silabs_bgapi_ll_radio_raise_ll_events_calls() -> u32;
    fn silabs_bgapi_ll_radio_raise_ll_events_last() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_calls() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_last_params() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_last_caller() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_last_err_before() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_last_err_after() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_error_transition_calls() -> u32;
    fn silabs_bgapi_rail_start_tx_calls() -> u32;
    fn silabs_bgapi_rail_start_tx_last_status() -> u32;
    fn silabs_bgapi_rail_start_scheduled_tx_calls() -> u32;
    fn silabs_bgapi_rail_start_scheduled_tx_last_status() -> u32;
    fn silabs_bgapi_rail_ble_cfg_calls() -> u32;
    fn silabs_bgapi_rail_ble_cfg_last_status() -> u32;
    fn silabs_bgapi_rail_ble_cfg_last_radio_state() -> u32;
    fn silabs_bgapi_rail_init_calls() -> u32;
    fn silabs_bgapi_rail_init_last_status() -> u32;
    fn silabs_bgapi_rail_ble_init_calls() -> u32;
    fn silabs_bgapi_rail_ble_init_last_status() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_param_w0() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_param_w1() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_param_w2() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_param_w3() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_param_w4() -> u32;
    fn silabs_bgapi_ll_radio_schedule_tx_param_w5() -> u32;
    fn silabs_bgapi_usch_schedule_calls() -> u32;
    fn silabs_bgapi_usch_schedule_req_calls() -> u32;
    fn silabs_bgapi_usch_add_task_enter_calls() -> u32;
    fn silabs_bgapi_usch_add_task_return_calls() -> u32;
    fn silabs_bgapi_ll_adv_set_enable_calls() -> u32;
    fn silabs_bgapi_usch_add_task_skip_once_calls() -> u32;
    fn silabs_bgapi_usch_add_task_last_task_ptr() -> u32;
    fn silabs_bgapi_usch_add_task_last_next_ptr() -> u32;
    fn silabs_bgapi_usch_add_task_last_schedule_ptr() -> u32;
    fn silabs_bgapi_usch_add_task_last_start() -> u32;
    fn silabs_bgapi_usch_add_task_last_init0() -> u32;
    fn silabs_bgapi_usch_add_task_last_init1() -> u32;
    fn silabs_bgapi_usch_add_task_last_min_runtime() -> u32;
    fn silabs_bgapi_usch_add_task_last_max_runtime() -> u32;
    fn silabs_bgapi_usch_add_task_last_handler_ptr() -> u32;
    fn silabs_bgapi_usch_add_task_last_flags() -> u8;
    fn silabs_bgapi_usch_add_task_last_priority() -> u8;
    fn silabs_bgapi_usch_add_task_last_id() -> u16;
    fn silabs_bgapi_ll_adv_set_enable_last() -> u32;
    fn silabs_bgapi_ll_mbox_cb_calls() -> u32;
    fn silabs_ble_post_stack_init_pump_step();
    fn silabs_ble_post_stack_init_pump_finish();
    fn silabs_bgapi_system_boot_seen() -> u8;
    fn silabs_bgapi_post_init_pump_steps() -> u32;
    fn silabs_ble_force_shm_link();
}

pub fn init_stack() {
    for step in 1..=ble::INIT_STEPS {
        info!("ble init step={} start", step);
        ble::init_step(step);
        info!("ble init step={} done", step);
    }
    info!("ble init post=mark_initialized start");
    ble::mark_initialized();
    info!("ble init post=mark_initialized done");
    info!("ble init post=post_stack_pump start");
    const MAX_PUMP: u32 = 64;
    for i in 0..MAX_PUMP {
        unsafe { silabs_ble_post_stack_init_pump_step() };
        let boot = unsafe { silabs_bgapi_system_boot_seen() };
        let steps = unsafe { silabs_bgapi_post_init_pump_steps() };
        if i == 0 || boot != 0 || (i + 1) % 8 == 0 {
            info!("ble init pump i={} boot={} steps={}", i, boot, steps);
        }
        if boot != 0 && i >= 3 {
            break;
        }
    }
    info!("ble init post=adv_start begin");
    unsafe { silabs_ble_post_stack_init_pump_finish() };
    info!(
        "ble init post=adv_start done phase={} adv4=0x{:x}",
        unsafe { silabs_ble_step_phase_read() },
        unsafe { silabs_bgapi_ble_adv_setup_status() },
    );
    info!("ble init post=post_stack_pump done");
    info!("ble init post=force_shm_link start");
    unsafe { silabs_ble_force_shm_link() };
    info!("ble init post=force_shm_link done");
    info!(
        "ble init done adv4=0x{:x} hci={}/{}/{} usch={}/{} add_task={}/{} ll_en={}/0x{:x} ll_err={}/0x{:x} lr=0x{:x} ll_tx={}/{} tx_lr=0x{:x} rail init/ble=0x{:x}/0x{:x} rail cfg/stx/tx=0x{:x}/0x{:x}/0x{:x} rail_state=0x{:x} rail calls init/ble/cfg/stx/tx={}/{}/{}/{}/{}",
        unsafe { silabs_bgapi_ble_adv_setup_status() },
        unsafe { silabs_bgapi_hci_adv_enable_on_calls() },
        unsafe { silabs_bgapi_hci_adv_enable_off_calls() },
        unsafe { silabs_bgapi_hci_adv_enable_calls() },
        unsafe { silabs_bgapi_usch_schedule_calls() },
        unsafe { silabs_bgapi_usch_schedule_req_calls() },
        unsafe { silabs_bgapi_usch_add_task_enter_calls() },
        unsafe { silabs_bgapi_usch_add_task_return_calls() },
        unsafe { silabs_bgapi_ll_adv_set_enable_calls() },
        unsafe { silabs_bgapi_ll_adv_set_enable_last() },
        unsafe { silabs_bgapi_ll_radio_raise_error_calls() },
        unsafe { silabs_bgapi_ll_radio_raise_error_last() },
        unsafe { silabs_bgapi_ll_radio_raise_error_caller_last() },
        unsafe { silabs_bgapi_ll_radio_schedule_tx_calls() },
        unsafe { silabs_bgapi_ll_radio_schedule_tx_error_transition_calls() },
        unsafe { silabs_bgapi_ll_radio_schedule_tx_last_caller() },
        unsafe { silabs_bgapi_rail_init_last_status() },
        unsafe { silabs_bgapi_rail_ble_init_last_status() },
        unsafe { silabs_bgapi_rail_ble_cfg_last_status() },
        unsafe { silabs_bgapi_rail_start_scheduled_tx_last_status() },
        unsafe { silabs_bgapi_rail_start_tx_last_status() },
        unsafe { silabs_bgapi_rail_ble_cfg_last_radio_state() },
        unsafe { silabs_bgapi_rail_init_calls() },
        unsafe { silabs_bgapi_rail_ble_init_calls() },
        unsafe { silabs_bgapi_rail_ble_cfg_calls() },
        unsafe { silabs_bgapi_rail_start_scheduled_tx_calls() },
        unsafe { silabs_bgapi_rail_start_tx_calls() },
    );
}

pub fn pump_loop<F>(mut on_tick: F)
where
    F: FnMut(),
{
    unsafe {
        silabs_ble_scheduler_set_enabled(1);
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
