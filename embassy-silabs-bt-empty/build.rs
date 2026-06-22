use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    // Remove stale custom defmt.x if present (breaks _defmt_timestamp linking).
    let stale_defmt = out.join("defmt.x");
    if stale_defmt.exists() {
        fs::remove_file(&stale_defmt).unwrap();
    }
    fs::copy("memory.x", out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    if env::var("CARGO_FEATURE_BLE_EMPTY").is_ok() {
        println!("cargo:rustc-env=BLE_EMPTY_BUILD_TAG=ble-empty-discover-v60");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_btctrl_init_functional");
        println!("cargo:rustc-link-arg-bins=--wrap=ubt_run");
        println!("cargo:rustc-link-arg-bins=--wrap=hci_le_set_extended_advertising_enable");
        println!("cargo:rustc-link-arg-bins=--wrap=usch_ScheduleProcess");
        println!("cargo:rustc-link-arg-bins=--wrap=usch_ScheduleReqCB");
        println!("cargo:rustc-link-arg-bins=--wrap=sli_ll_adv_set_advertising_enable");
        println!("cargo:rustc-link-arg-bins=--wrap=ll_execTimingInit");
        println!("cargo:rustc-link-arg-bins=--wrap=usch_AddTask");
        println!("cargo:rustc-link-arg-bins=--wrap=sli_ll_mbox_message_cb");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_btctrl_raise_events");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_btctrl_process_events");
        println!("cargo:rustc-link-arg-bins=--wrap=sli_ll_radio_raise_error");
        println!("cargo:rustc-link-arg-bins=--wrap=sli_ll_radio_raise_ll_events");
        println!("cargo:rustc-link-arg-bins=--wrap=sli_ll_radio_schedule_tx");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_rail_init");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_rail_ble_init");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_rail_start_tx");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_rail_start_scheduled_tx");
        println!("cargo:rustc-link-arg-bins=--wrap=sl_rail_ble_config_channel_radio_params");
        println!("cargo:rustc-link-arg-bins=--wrap=bg_message_queue_wait_time");
        // Force-link RAIL radio IRQ handlers and cortex-m-rt vector wrapper symbols.
        for sym in [
            "AGC",
            "BUFC",
            "EMUDG",
            "FRC_PRI",
            "FRC",
            "MODEM",
            "PROTIMER",
            "RAC_RSM",
            "RAC_SEQ",
            "HOSTMAILBOX",
            "SYNTH",
            "RFECA0",
            "RFECA1",
            "SYSRTC_SEQ",
            "SYSRTC_APP",
            "AGC_IRQHandler",
            "BUFC_IRQHandler",
            "EMUDG_IRQHandler",
            "FRC_PRI_IRQHandler",
            "FRC_IRQHandler",
            "MODEM_IRQHandler",
            "PROTIMER_IRQHandler",
            "RAC_RSM_IRQHandler",
            "RAC_SEQ_IRQHandler",
            "HOSTMAILBOX_IRQHandler",
            "SYNTH_IRQHandler",
            "RFECA0_IRQHandler",
            "RFECA1_IRQHandler",
            "SYSRTC_SEQ_IRQHandler",
            "SYSRTC_APP_IRQHandler",
        ] {
            println!("cargo:rustc-link-arg-bins=-u{sym}");
        }
    }

    println!("cargo:rerun-if-changed=memory.x");
}
