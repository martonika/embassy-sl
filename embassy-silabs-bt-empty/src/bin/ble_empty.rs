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

#[allow(dead_code)]
unsafe extern "C" {
    fn silabs_bgapi_ble_scan_request_count() -> u32;
    fn silabs_bgapi_ble_system_boot_handler_done() -> u8;
    fn silabs_bgapi_bt_step_count() -> u32;
    fn silabs_bgapi_ble_ll_events_total() -> u32;
    fn silabs_bgapi_ble_hci_drain_iterations() -> u32;
    fn silabs_bgapi_irq_frc_count() -> u32;
    fn silabs_bgapi_irq_modem_count() -> u32;
    fn silabs_bgapi_irq_pendsv_count() -> u32;
    fn silabs_bgapi_irq_rac_seq_count() -> u32;
    fn silabs_bgapi_irq_sysrtc_seq_count() -> u32;
    fn silabs_bgapi_irq_sysrtc_app_count() -> u32;
    fn silabs_bgapi_ll_events_peek() -> u32;
    fn silabs_bgapi_ll_pending_peek() -> u32;
    fn silabs_bgapi_ll_mbox_cb_calls() -> u32;
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
    fn silabs_bgapi_usch_add_task_skip_once_calls() -> u32;
    fn silabs_bgapi_ll_adv_set_enable_calls() -> u32;
    fn silabs_bgapi_ll_adv_set_enable_last() -> u32;
    fn silabs_bgapi_hci_adv_enable_calls() -> u32;
    fn silabs_bgapi_hci_adv_enable_on_calls() -> u32;
    fn silabs_bgapi_hci_adv_enable_off_calls() -> u32;
    fn silabs_bgapi_hci_adv_enable_last_num_sets() -> u8;
    fn silabs_bgapi_hci_adv_enable_last_maxevents() -> u8;
    fn silabs_bgapi_hci_adv_enable_last_handle() -> u8;
    fn silabs_bgapi_primask() -> u32;
    fn silabs_bgapi_nvic_iser1() -> u32;
    fn silabs_bgapi_ll_raise_total() -> u32;
    fn silabs_bgapi_nvic_iser0() -> u32;
    fn silabs_bgapi_sysrtc_cnt() -> u32;
}

/// Two seconds at 32768 Hz.
const RF_LOG_TICKS: u32 = 65_536;

fn log_rf_stats(rtc: u32) {
    info!(
        "ble rf rtc={}: scan={} steps={} done={} ll_evt={}/{} usch={}/{} ll_en=0x{:x} hci_en={} irq m/f/p={}/{}/{} ll_err={}/0x{:x} ll_tx={}/{} rail cfg/stx/tx={}/{}/{} calls cfg/stx/tx={}/{}/{}",
        rtc,
        unsafe { silabs_bgapi_ble_scan_request_count() },
        unsafe { silabs_bgapi_bt_step_count() },
        unsafe { silabs_bgapi_ble_system_boot_handler_done() },
        unsafe { silabs_bgapi_ll_events_peek() },
        unsafe { silabs_bgapi_ble_ll_events_total() },
        unsafe { silabs_bgapi_usch_schedule_calls() },
        unsafe { silabs_bgapi_usch_schedule_req_calls() },
        unsafe { silabs_bgapi_ll_adv_set_enable_last() },
        unsafe { silabs_bgapi_hci_adv_enable_last_num_sets() },
        unsafe { silabs_bgapi_irq_modem_count() },
        unsafe { silabs_bgapi_irq_frc_count() },
        unsafe { silabs_bgapi_irq_pendsv_count() },
        unsafe { silabs_bgapi_ll_radio_raise_error_calls() },
        unsafe { silabs_bgapi_ll_radio_raise_error_last() },
        unsafe { silabs_bgapi_ll_radio_schedule_tx_calls() },
        unsafe { silabs_bgapi_ll_radio_schedule_tx_error_transition_calls() },
        unsafe { silabs_bgapi_rail_ble_cfg_last_status() },
        unsafe { silabs_bgapi_rail_start_scheduled_tx_last_status() },
        unsafe { silabs_bgapi_rail_start_tx_last_status() },
        unsafe { silabs_bgapi_rail_ble_cfg_calls() },
        unsafe { silabs_bgapi_rail_start_scheduled_tx_calls() },
        unsafe { silabs_bgapi_rail_start_tx_calls() },
    );
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("ble empty boot tag={}", env!("BLE_EMPTY_BUILD_TAG"));

    delay::init();
    info!("ble main step=after_delay_init");
    let p = embassy_silabs::init();
    info!("ble main step=after_embassy_init");

    let (board, _) = Board::new(p);
    board.route_rf_activity_leds();
    info!("ble main step=after_board_new rf_activity_leds=prs");

    info!("ble main step=before_init_stack");
    ble_runtime::init_stack();
    info!("ble main step=after_init_stack");

    let mut last_log_rtc = unsafe { silabs_bgapi_sysrtc_cnt() };
    let mut first_log = true;

    ble_runtime::pump_loop(|| {
        let rtc = unsafe { silabs_bgapi_sysrtc_cnt() };

        if first_log || rtc.wrapping_sub(last_log_rtc) >= RF_LOG_TICKS {
            first_log = false;
            last_log_rtc = rtc;
            log_rf_stats(rtc);
        }
    });
}
