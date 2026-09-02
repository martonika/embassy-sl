#![allow(non_upper_case_globals)]

use silabs_bluetooth_sys::{
    sl_bt_advertiser_create_set, sl_bt_advertiser_set_timing,
    sl_bt_advertiser_discovery_mode_t_sl_bt_advertiser_general_discoverable,
    sl_bt_evt_advertiser_scan_request_id, sl_bt_evt_connection_closed_id, sl_bt_evt_system_boot_id,
    sl_bt_gap_get_identity_address, sl_bt_legacy_advertiser_generate_data, sl_bt_msg_t,
};

const SL_BT_LEGACY_ADVERTISER_CONNECTABLE: u8 = 0x2;

static mut ADV_SET_HANDLE: u8 = 0xff;
static mut PENDING_ADV_START: u8 = 0;
const SL_STATUS_OK: u32 = 0;

const fn sl_bt_msg_id(header: u32) -> u32 {
    header & 0xffff00f8
}

unsafe extern "C" {
    fn silabs_bgapi_note_ble_adv_setup_status(status: u32);
    fn silabs_bgapi_note_ble_identity_address(address: *const u8, addr_type: u8);
    fn silabs_bgapi_note_ble_identity_status(status: u32);
    fn silabs_bgapi_note_ble_on_event_called();
    fn silabs_bgapi_note_ble_scan_request();
    fn silabs_bgapi_note_ble_system_boot_handler_done(done: u8);
    fn silabs_ble_step_phase_write(phase: u32);
}

fn note_adv_status(step: u8, status: u32) {
    unsafe { silabs_bgapi_note_ble_adv_setup_status(((step as u32) << 24) | status) }
}

fn note_identity() {
    let mut address = silabs_bluetooth_sys::bd_addr { addr: [0; 6] };
    let mut addr_type = 0u8;
    let sc = unsafe { sl_bt_gap_get_identity_address(&mut address, &mut addr_type) };
    unsafe {
        silabs_bgapi_note_ble_identity_status(sc);
        if sc == SL_STATUS_OK {
            silabs_bgapi_note_ble_identity_address(address.addr.as_ptr(), addr_type);
        }
    }
}

fn generate_adv_data() -> u32 {
    unsafe {
        sl_bt_legacy_advertiser_generate_data(
            ADV_SET_HANDLE,
            sl_bt_advertiser_discovery_mode_t_sl_bt_advertiser_general_discoverable as u8,
        )
    }
}

fn set_pending_adv_start() {
    unsafe { PENDING_ADV_START = 1 };
    // defmt::info!("ble adv pending=1");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn silabs_ble_adv_handle_read() -> u8 {
    unsafe { ADV_SET_HANDLE }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn silabs_ble_pending_adv_start_read() -> u8 {
    unsafe { PENDING_ADV_START }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn silabs_ble_pending_adv_start_clear() {
    unsafe { PENDING_ADV_START = 0 };
}

/// Legacy hook kept for C glue compatibility.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn silabs_ble_start_after_init() {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sl_bt_on_event(evt: *mut sl_bt_msg_t) {
    if evt.is_null() {
        return;
    }

    unsafe { silabs_bgapi_note_ble_on_event_called() };
    let evt = unsafe { &*evt };

    match sl_bt_msg_id(evt.header) {
        sl_bt_evt_system_boot_id => {
            unsafe { silabs_ble_step_phase_write(30) };
            note_identity();
            unsafe {
                silabs_bgapi_note_ble_system_boot_handler_done(0);
                silabs_ble_step_phase_write(31);
            }

            let sc = unsafe { sl_bt_advertiser_create_set(core::ptr::addr_of_mut!(ADV_SET_HANDLE)) };
            note_adv_status(1, sc);
            if sc != SL_STATUS_OK {
                return;
            }

            unsafe { silabs_ble_step_phase_write(41) };
            let sc = generate_adv_data();
            note_adv_status(2, sc);
            if sc != SL_STATUS_OK {
                return;
            }

            let sc = unsafe { sl_bt_advertiser_set_timing(ADV_SET_HANDLE, 160, 160, 0, 0) };
            note_adv_status(3, sc);
            if sc != SL_STATUS_OK {
                return;
            }

            unsafe { silabs_ble_step_phase_write(42) };
            // Defer advertiser_start until after post_stack_init_pump completes.
            // Starting inside the boot handler (during the 128-step init pump) can
            // block in usch_ScheduleProcess now that ll_hciCall reaches the LL.
            set_pending_adv_start();
            note_adv_status(4, SL_STATUS_OK);
        }
        sl_bt_evt_advertiser_scan_request_id => {
            unsafe { silabs_bgapi_note_ble_scan_request() };
        }
        sl_bt_evt_connection_closed_id => {
            let sc = generate_adv_data();
            note_adv_status(5, sc);
            if sc == SL_STATUS_OK {
                set_pending_adv_start();
            }
        }
        _ => {}
    }
}
