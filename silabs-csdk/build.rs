use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sdk_path() -> PathBuf {
    env::var("SILABS_SDK")
        .map(PathBuf::from)
        .expect("SILABS_SDK must be set to the Simplicity SDK root directory")
}

fn push_if_exists(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.exists() {
        paths.push(path);
    }
}

fn apply_common_cc_flags(build: &mut cc::Build, includes: &[PathBuf]) {
    build
        .compiler(env::var("CC").unwrap_or_else(|_| "arm-none-eabi-gcc".to_string()))
        .archiver(env::var("AR").unwrap_or_else(|_| "arm-none-eabi-ar".to_string()))
        .target("arm-none-eabi")
        .flag("-mcpu=cortex-m33")
        .flag("-mthumb")
        .flag("-ffunction-sections")
        .flag("-fdata-sections")
        .flag("-fno-strict-aliasing")
        .define("EFR32MG24B220F1536IM48", None)
        .define("HFXO_FREQ", "39000000")
        .define("SL_COMPONENT_CATALOG_PRESENT", None)
        .define("SL_RAIL_LIB_MULTIPROTOCOL_SUPPORT", "0");

    for inc in includes {
        if inc.is_dir() {
            build.include(inc);
        }
    }
}

fn sdk_rail_sources(sdk: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let rail = sdk.join("rail_library");

    let mut sources = vec![
        pc.join("emlib/src/em_cmu.c"),
        pc.join("emlib/src/em_emu.c"),
        pc.join("emlib/src/em_gpio.c"),
        pc.join("emlib/src/em_ldma.c"),
        pc.join("emlib/src/em_prs.c"),
        pc.join("emlib/src/em_system.c"),
        pc.join("emlib/src/em_timer.c"),
        pc.join("emlib/src/em_msc.c"),
        pc.join("common/src/sl_core_cortexm.c"),
        pc.join("service/device_init/src/sl_device_init_hfxo_s2.c"),
        pc.join("service/device_init/src/sl_device_init_dcdc_s2.c"),
        pc.join("service/device_init/src/sl_device_init_emu_s2.c"),
        pc.join("service/device_init/src/sl_device_init_dpll_s2.c"),
        rail.join("hal/efr32/hal_efr.c"),
        rail.join("plugin/pa-conversions/pa_conversions_efr32.c"),
        rail.join("plugin/pa-conversions/pa_curves_efr32.c"),
        rail.join("plugin/pa-auto-mode/pa_auto_mode.c"),
        rail.join("plugin/sl_rail_util_power_manager_init/sl_rail_util_power_manager_init.c"),
        rail.join("plugin/sl_rail_util_sequencer/sl_rail_util_sequencer.c"),
        rail.join("plugin/sl_rail_util_pti/sl_rail_util_pti.c"),
        // BLE 39 MHz channel configs; pointers forced via silabs_rail_ble_phy_force.c + -u.
        rail.join("plugin/sl_rail_util_built_in_phys/efr32xg24/sl_rail_ble_config_39MHz.c"),
        rail.join("plugin/sl_rail_util_built_in_phys/efr32xg24/sl_rail_ieee802154_config_39MHz.c"),
        rail.join("plugin/sl_rail_util_built_in_phys/efr32xg24/sl_rail_rfsense_ook_config_39MHz.c"),
    ];

    if let Ok(dir) = env::var("SILABS_RAIL_CONFIG_DIR") {
        push_if_exists(&mut sources, PathBuf::from(dir).join("rail_config.c"));
    }
    let local_rail_config = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("rail_config/rail_config.c");
    push_if_exists(&mut sources, local_rail_config);

    sources.retain(|p| p.exists());
    sources
}

fn sdk_app_timer_sources(sdk: &Path) -> Vec<PathBuf> {
    vec![sdk.join(
        "platform_core/app/common/util/app_timer/bm/app_timer.c",
    )]
}

fn sdk_sleeptimer_sources(sdk: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let dm = pc.join("service/device_manager");
    vec![
        pc.join("service/sleeptimer/src/sl_sleeptimer.c"),
        pc.join("service/sleeptimer/src/sl_sleeptimer_hal_sysrtc.c"),
        pc.join("peripheral/src/sl_hal_sysrtc.c"),
        pc.join("peripheral/src/sl_hal_sysrtc_subsystem.c"),
        dm.join("clocks/sl_device_clock_efr32xg24.c"),
        dm.join("devices/sl_device_peripheral_hal_efr32xg24.c"),
    ]
}

fn sleeptimer_includes(sdk: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    vec![
        pc.join("peripheral/inc"),
        pc.join("service/clock_manager/inc"),
        pc.join("service/device_manager/inc"),
    ]
}

fn app_timer_includes(sdk: &Path) -> Vec<PathBuf> {
    let app = sdk.join("platform_core/app/common/util/app_timer");
    vec![app.clone(), app.join("bm")]
}

fn sdk_ble_sources(sdk: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let ble_host = sdk.join("bluetooth_le_host");
    let ble_common = sdk.join("bluetooth_common");
    let mem = pc.join("service/memory_manager");

    let ble_ctrl = sdk.join("bluetooth_le_controller");

    let mut sources = vec![
        ble_ctrl.join("src/sl_btctrl_init.c"),
        ble_ctrl.join("src/sl_btctrl_init_tasklets.c"),
        ble_host.join("src/sl_bt_stack_init.c"),
        ble_host.join("src/sli_bt_hci_event_table.c"),
        ble_host.join("src/sli_bt_host_adaptation.c"),
        ble_host.join("src/sli_bt_advertiser_config.c"),
        ble_host.join("src/sli_bt_connection_config.c"),
        ble_host.join("src/sli_bt_dynamic_gattdb_config.c"),
        ble_host.join("src/sli_bt_l2cap_config.c"),
        ble_host.join("src/sli_bt_sync_config.c"),
        ble_host.join("src/sli_bt_accept_list_config.c"),
        ble_host.join("src/sli_bt_external_bondingdb_config.c"),
        ble_common.join("src/sli_bluetooth_common_config.c"),
        ble_common.join("src/sli_bgcommon_debug_efr32.c"),
        mem.join("src/sl_memory_manager.c"),
        mem.join("src/sl_memory_manager_dynamic_reservation.c"),
        mem.join("src/sli_memory_manager_common.c"),
        mem.join("src/sl_memory_manager_retarget.c"),
        mem.join("src/sl_memory_manager_pool_common.c"),
        mem.join("src/sl_memory_manager_pool.c"),
        mem.join("src/sl_memory_manager_region.c"),
        // 2026.6: profiler/src/sli_memory_profiler_stubs.c (removed in 2026.12)
        mem.join("hal/sli_memory_manager_retention_control_hal_none.c"),
        pc.join("common/src/sl_slist.c"),
    ];

    sources.retain(|p| p.exists());
    sources
}

fn sdk_btmesh_sources(sdk: &Path, out_dir: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let nvm3 = pc.join("emdrv/nvm3");

    let mut sources = vec![
        nvm3.join("src/nvm3.c"),
        nvm3.join("src/nvm3_cache.c"),
        nvm3.join("src/nvm3_hal_flash.c"),
        nvm3.join("src/nvm3_lock.c"),
        nvm3.join("src/nvm3_object.c"),
        nvm3.join("src/nvm3_page.c"),
        nvm3.join("src/nvm3_utils.c"),
        nvm3.join("src/nvm3_default_common_linker.c"),
    ];

    push_if_exists(&mut sources, out_dir.join("sl_btmesh_dcd.c"));
    push_if_exists(&mut sources, out_dir.join("gatt_db.c"));

    sources.retain(|p| p.exists());
    sources
}

fn crypto_includes(sdk: &Path, manifest_dir: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let sec = pc.join("security/sl_component");
    let mbedtls = sdk.join("security_mbedtls_source");
    vec![
        manifest_dir.join("include/config/autogen"),
        mbedtls.join("include"),
        mbedtls.join("library"),
        sec.join("sl_mbedtls_support/inc"),
        sec.join("sl_mbedtls_support/config"),
        sec.join("sl_psa_driver/inc"),
        sec.join("se_manager/inc"),
        sec.join("sl_protocol_crypto/src"),
        sec.join("sli_crypto/inc"),
        sec.join("sli_psec_osal/inc"),
        pc.join("security/sl_component/sl_protocol_crypto/inc"),
        pc.join("hal/inc"),
        pc.join("service/clock_manager/inc"),
    ]
}

fn crypto_sources(sdk: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let sec = pc.join("security/sl_component");
    let mbedtls = sdk.join("security_mbedtls_source/library");

    let mut sources = vec![
        mbedtls.join("cipher.c"),
        mbedtls.join("cipher_wrap.c"),
        mbedtls.join("constant_time.c"),
        mbedtls.join("platform.c"),
        mbedtls.join("platform_util.c"),
        mbedtls.join("psa_crypto.c"),
        mbedtls.join("psa_crypto_aead.c"),
        mbedtls.join("psa_crypto_cipher.c"),
        mbedtls.join("psa_crypto_client.c"),
        mbedtls.join("psa_crypto_driver_wrappers_no_static.c"),
        mbedtls.join("psa_crypto_ecp.c"),
        mbedtls.join("psa_crypto_ffdh.c"),
        mbedtls.join("psa_crypto_hash.c"),
        mbedtls.join("psa_crypto_mac.c"),
        mbedtls.join("psa_crypto_pake.c"),
        mbedtls.join("psa_crypto_rsa.c"),
        mbedtls.join("psa_crypto_se.c"),
        mbedtls.join("psa_crypto_slot_management.c"),
        mbedtls.join("psa_crypto_storage.c"),
        mbedtls.join("psa_util.c"),
        mbedtls.join("threading.c"),
        sec.join("se_manager/src/sl_se_manager.c"),
        sec.join("se_manager/src/sl_se_manager_util.c"),
        sec.join("se_manager/src/sl_se_manager_cipher.c"),
        sec.join("se_manager/src/sl_se_manager_entropy.c"),
        sec.join("se_manager/src/sl_se_manager_hash.c"),
        sec.join("se_manager/src/sl_se_manager_key_derivation.c"),
        sec.join("se_manager/src/sl_se_manager_key_handling.c"),
        sec.join("se_manager/src/sl_se_manager_signature.c"),
        sec.join("se_manager/src/sl_se_manager_attestation.c"),
        sec.join("se_manager/src/sli_se_manager_mailbox.c"),
        sec.join("sl_mbedtls_support/src/sl_mbedtls.c"),
        sec.join("sl_mbedtls_support/src/sl_psa_crypto.c"),
        sec.join("sl_mbedtls_support/src/sli_psa_crypto.c"),
        sec.join("sl_protocol_crypto/src/sli_protocol_crypto_radioaes.c"),
        sec.join("sl_protocol_crypto/src/sli_radioaes_management.c"),
        sec.join("sl_psa_driver/src/sl_psa_its_nvm3.c"),
        sec.join("sl_psa_driver/src/sli_psa_driver_common.c"),
        sec.join("sl_psa_driver/src/sli_psa_driver_init.c"),
        sec.join("sl_psa_driver/src/sli_psa_trng.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_aead.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_builtin_keys.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_cipher.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_key_derivation.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_key_management.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_mac.c"),
        sec.join("sl_psa_driver/src/sli_se_driver_signature.c"),
        sec.join("sl_psa_driver/src/sli_se_opaque_driver_aead.c"),
        sec.join("sl_psa_driver/src/sli_se_opaque_driver_cipher.c"),
        sec.join("sl_psa_driver/src/sli_se_opaque_driver_mac.c"),
        sec.join("sl_psa_driver/src/sli_se_opaque_key_derivation.c"),
        sec.join("sl_psa_driver/src/sli_se_transparent_driver_aead.c"),
        sec.join("sl_psa_driver/src/sli_se_transparent_driver_cipher.c"),
        sec.join("sl_psa_driver/src/sli_se_transparent_driver_hash.c"),
        sec.join("sl_psa_driver/src/sli_se_transparent_driver_mac.c"),
        sec.join("sl_psa_driver/src/sli_se_transparent_key_derivation.c"),
        sec.join("sl_psa_driver/src/sli_se_version_dependencies.c"),
        sec.join("sli_crypto/src/sl_crypto_s2.c"),
    ];

    sources.retain(|p| p.exists());
    sources
}

fn generate_btmesh_dcd(sdk: &Path, manifest_dir: &Path, out_dir: &Path) {
    let generator = sdk.join("bluetooth_mesh_middleware/script/generator/BtMeshGenerator.py");
    let input = manifest_dir.join("btmesh/dcd_config.btmeshconf");
    if !generator.exists() || !input.exists() {
        println!("cargo:warning=BtMeshGenerator or dcd_config.btmeshconf not found; mesh DCD not generated");
        return;
    }

    let _ = std::fs::create_dir_all(out_dir);
    println!("cargo:rerun-if-changed={}", input.display());
    let status = Command::new("python3")
        .arg(&generator)
        .arg(&input)
        .arg(out_dir)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => println!(
            "cargo:warning=BtMeshGenerator failed with status {s}; mesh may not link"
        ),
        Err(err) => println!("cargo:warning=Could not run BtMeshGenerator: {err}"),
    }
}

fn generate_gatt_db_ble(sdk: &Path, manifest_dir: &Path, out_dir: &Path) {
    let generator = sdk.join("bluetooth_bgbuild/bgbuild.py");
    let btconf = manifest_dir.join("ble/gatt_configuration.btconf");

    if !generator.exists() || !btconf.exists() {
        println!("cargo:warning=bgbuild.py or gatt_configuration.btconf not found; BLE GATT DB not generated");
        return;
    }

    let _ = std::fs::create_dir_all(out_dir);
    println!("cargo:rerun-if-changed={}", btconf.display());

    let status = Command::new("python3")
        .arg(&generator)
        .arg("-o")
        .arg(out_dir)
        .arg(&btconf)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => println!(
            "cargo:warning=bgbuild.py failed with status {s}; BLE GATT DB not generated"
        ),
        Err(err) => println!("cargo:warning=Could not run bgbuild.py: {err}"),
    }
}

fn generate_gatt_db(sdk: &Path, manifest_dir: &Path, out_dir: &Path) {
    let generator = sdk.join("bluetooth_bgbuild/bgbuild.py");
    let btconf = manifest_dir.join("ble/gatt_configuration.btconf");
    let mesh = sdk.join("bluetooth_mesh/component");
    let inputs = [
        btconf.clone(),
        mesh.join("gatt_service_mesh_default.xml"),
        mesh.join("gatt_service_mesh_proxy.xml"),
        mesh.join("gatt_service_mesh_prov.xml"),
    ];

    if !generator.exists() || !btconf.exists() {
        println!("cargo:warning=bgbuild.py or gatt_configuration.btconf not found; GATT DB not generated");
        return;
    }

    let _ = std::fs::create_dir_all(out_dir);
    println!("cargo:rerun-if-changed={}", btconf.display());
    for input in &inputs[1..] {
        println!("cargo:rerun-if-changed={}", input.display());
    }

    let mut cmd = Command::new("python3");
    cmd.arg(&generator).arg("-o").arg(out_dir);
    for input in &inputs {
        if input.exists() {
            cmd.arg(input);
        }
    }

    match cmd.status() {
        Ok(s) if s.success() => {}
        Ok(s) => println!("cargo:warning=bgbuild.py failed with status {s}; BLE GATT DB not generated"),
        Err(err) => println!("cargo:warning=Could not run bgbuild.py: {err}"),
    }
}

fn base_includes(sdk: &Path, manifest_dir: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    let rail = sdk.join("rail_library");

    vec![
        manifest_dir.join("include"),
        manifest_dir.join("include/config"),
        pc.join("Device/SiliconLabs/EFR32MG24/Include"),
        sdk.join("cmsis/Core/Include"),
        pc.join("emlib/inc"),
        pc.join("common/inc"),
        pc.join("common/config"),
        pc.join("service/sleeptimer/config"),
        pc.join("service/power_manager/config"),
        pc.join("driver/gpio/inc"),
        pc.join("service/interrupt_manager/inc"),
        pc.join("service/sleeptimer/inc"),
        pc.join("service/power_manager/inc"),
        pc.join("service/device_init/inc"),
        pc.join("service/device_manager/inc"),
        pc.join("service/device_init/config/s2/sdid230"),
        pc.join("service/device_init/config/s2"),
        rail.join("common"),
        rail.join("protocol/ble"),
        rail.join("protocol/ieee802154"),
        rail.join("chip/efr32/efr32xg2x"),
        rail.join("hal"),
        rail.join("hal/efr32"),
        rail.join("plugin/sl_rail_util_protocol"),
        rail.join("plugin/sl_rail_util_protocol/config/efr32xg24"),
        rail.join("plugin/sl_rail_util_callbacks"),
        rail.join("plugin/sl_rail_util_callbacks/config"),
        rail.join("plugin/pa-conversions"),
        rail.join("plugin/pa-auto-mode"),
        rail.join("plugin/sl_rail_util_compatible_pa"),
        rail.join("plugin/sl_rail_util_power_manager_init"),
        rail.join("plugin/sl_rail_util_power_manager_init/config"),
        rail.join("plugin/sl_rail_util_pti"),
        rail.join("plugin/sl_rail_util_pti/config"),
        rail.join("plugin/sl_rail_util_built_in_phys/efr32xg24"),
    ]
}

fn ble_includes(sdk: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    vec![
        sdk.join("bluetooth_le_controller/inc"),
        sdk.join("bluetooth_le_controller/config"),
        sdk.join("bluetooth_le_host/inc"),
        sdk.join("bluetooth_le_host/config"),
        // Present in older SDKs; absent in 2026.12 (apply_common_cc_flags skips missing dirs)
        sdk.join("bluetooth_le_host/legacy_to_refactor/bgstack/ubt"),
        sdk.join("bluetooth_common/inc"),
        sdk.join("bluetooth_common/config"),
        sdk.join("bgapi_protocol/protocol/inc"),
        sdk.join("bgapi_protocol/config"),
        // New in 2026.12: memory_manager includes sli_memory_manager_log.h → sl_log
        pc.join("service/sl_log/inc"),
        pc.join("service/sl_log/config"),
        pc.join("service/memory_manager/inc"),
        pc.join("service/memory_manager/src"),
        pc.join("service/memory_manager/config"),
        pc.join("service/memory_manager/config/legacy"),
        // Present in older SDKs; removed in 2026.12
        pc.join("service/memory_manager/profiler/inc"),
        pc.join("service/memory_manager/profiler/config"),
    ]
}

fn btmesh_middleware_includes(sdk: &Path) -> Vec<PathBuf> {
    let mw = sdk.join("bluetooth_mesh_middleware/common");
    let app = sdk.join("platform_core/app/common/util");
    vec![
        mw.join("btmesh_provisionee"),
        mw.join("app_btmesh_util"),
        app.join("app_assert"),
    ]
}

fn btmesh_middleware_sources(sdk: &Path) -> Vec<PathBuf> {
    vec![sdk.join(
        "bluetooth_mesh_middleware/common/btmesh_provisionee/sl_btmesh_provisionee.c",
    )]
}

fn btmesh_includes(sdk: &Path, out_dir: &Path) -> Vec<PathBuf> {
    let pc = sdk.join("platform_core/platform");
    vec![
        sdk.join("bluetooth_mesh/inc"),
        sdk.join("bluetooth_mesh/config"),
        pc.join("emdrv/common/inc"),
        pc.join("emdrv/nvm3/inc"),
        pc.join("emdrv/nvm3/config"),
        pc.join("emdrv/nvm3/config/s2"),
        out_dir.to_path_buf(),
    ]
}

fn main() {
    let sdk = sdk_path();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ble = env::var("CARGO_FEATURE_BLE").is_ok();
    let ble_rust_handler = env::var("CARGO_FEATURE_BLE_RUST_HANDLER").is_ok();
    let btmesh = env::var("CARGO_FEATURE_BTMESH").is_ok();
    let btmesh_c_handlers = env::var("CARGO_FEATURE_BTMESH_C_HANDLERS").is_ok();

    println!("cargo:rerun-if-env-changed=SILABS_SDK");
    println!("cargo:rerun-if-env-changed=SILABS_RAIL_CONFIG_DIR");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=include/");
    println!("cargo:rerun-if-changed=ble/");
    println!("cargo:rerun-if-changed=btmesh/");

    if ble {
        generate_gatt_db_ble(&sdk, &manifest_dir, &out_dir);
    }
    if btmesh {
        generate_btmesh_dcd(&sdk, &manifest_dir, &out_dir);
        generate_gatt_db(&sdk, &manifest_dir, &out_dir);
    }

    let mut includes = base_includes(&sdk, &manifest_dir);
    if ble {
        includes.extend(ble_includes(&sdk));
        includes.extend(app_timer_includes(&sdk));
        includes.extend(sleeptimer_includes(&sdk));
        includes.push(out_dir.clone());
    }
    if btmesh {
        includes.extend(btmesh_includes(&sdk, &out_dir));
        includes.extend(btmesh_middleware_includes(&sdk));
        includes.extend(crypto_includes(&sdk, &manifest_dir));
    }

    let mut build = cc::Build::new();
    apply_common_cc_flags(&mut build, &includes);
    // Match dev-profile behaviour: optimized SDK glue faults on hardware at -O3.
    build.opt_level(0);

    if ble {
        build.define("SILABS_CSDK_BLE", None);
        build.define("SL_BT_CONFIG_SET_CUSTOM_ADDRESS_FROM_NVM3", "0");
    }
    if ble_rust_handler {
        build.define("SILABS_BLE_RUST_HANDLER", None);
    }
    if btmesh {
        build.define("SILABS_CSDK_BTMESH", None);
        build.define("MBEDTLS_CONFIG_FILE", "<sl_mbedtls_config.h>");
        build.define("MBEDTLS_PSA_CRYPTO_CONFIG_FILE", "<psa_crypto_config.h>");
    }

    build.file(manifest_dir.join("src/silabs_bgapi_debug.c"));
    build.file(manifest_dir.join("src/string.c"));
    build.file(manifest_dir.join("src/sl_gpio_stub.c"));
    build.file(manifest_dir.join("src/sl_power_manager_stub.c"));
    build.file(manifest_dir.join("src/em_system_stub.c"));
    build.file(manifest_dir.join("src/sl_rail_util_protocol_stub.c"));
    build.file(manifest_dir.join("src/rail_callbacks.c"));
    build.file(manifest_dir.join("src/rail_builtin_queue.c"));
    build.file(manifest_dir.join("src/rail_platform_init.c"));

    for src in sdk_rail_sources(&sdk) {
        build.file(src);
    }

    if ble {
        build.file(manifest_dir.join("src/silabs_bt_stack_start.c"));
        build.file(manifest_dir.join("src/sl_btctrl_pendsv.c"));
        build.file(manifest_dir.join("src/silabs_linklayer_pump.c"));
        build.file(manifest_dir.join("src/silabs_radio_irq_vectors.c"));
        build.file(manifest_dir.join("src/silabs_bgmessage_stub.c"));
        build.file(manifest_dir.join("src/sl_device_init_clocks.c"));
        build.file(manifest_dir.join("src/sl_bluetooth.c"));
        build.file(manifest_dir.join("src/ble_platform_init.c"));
        if ble_rust_handler {
            build.file(manifest_dir.join("src/silabs_ble_handler_stubs.c"));
            build.file(manifest_dir.join("src/silabs_ble_adv_start.c"));
            build.file(manifest_dir.join("src/silabs_ble_radio_irq_enable.c"));
            build.file(manifest_dir.join("src/silabs_sleeptimer_platform_stubs.c"));
            build.file(manifest_dir.join("src/silabs_ble_hci_usch.c"));
            build.file(manifest_dir.join("src/silabs_ll_hci_post_service.c"));
            build.file(manifest_dir.join("src/silabs_ll_hci_call.c"));
            build.file(manifest_dir.join("src/silabs_btctrl_init_wrap.c"));
            build.file(manifest_dir.join("src/silabs_ll_raise_wrap.c"));
            build.file(manifest_dir.join("src/silabs_rail_ble_phy_force.c"));
            build.file(manifest_dir.join("src/psa_crypto_stub.c"));
            build.file(manifest_dir.join("src/ble_crypto_stub.c"));
        } else if !btmesh {
            build.file(manifest_dir.join("src/silabs_ble_adv_handler.c"));
            build.file(manifest_dir.join("src/psa_crypto_stub.c"));
            build.file(manifest_dir.join("src/ble_crypto_stub.c"));
        }
        for src in sdk_ble_sources(&sdk) {
            build.file(src);
        }
        for src in sdk_app_timer_sources(&sdk) {
            build.file(src);
        }
        for src in sdk_sleeptimer_sources(&sdk) {
            build.file(src);
        }
        if let Some(gatt_db) = out_dir.join("gatt_db.c").exists().then(|| out_dir.join("gatt_db.c")) {
            build.file(gatt_db);
        }
    }

    if btmesh && btmesh_c_handlers {
        build.file(manifest_dir.join("src/silabs_btmesh_stack_handlers.c"));
    }
    if btmesh {
        build.file(manifest_dir.join("src/sl_btmesh.c"));
        build.file(manifest_dir.join("src/silabs_btmesh_node_init.c"));
        build.file(manifest_dir.join("src/silabs_btmesh_cmd_node_init.c"));
        build.file(manifest_dir.join("src/silabs_btmesh_factory_reset.c"));
        build.file(manifest_dir.join("src/crypto_platform_init.c"));
        for src in btmesh_middleware_sources(&sdk) {
            build.file(src);
        }
        for src in sdk_btmesh_sources(&sdk, &out_dir) {
            build.file(src);
        }
        for src in crypto_sources(&sdk) {
            build.file(src);
        }
    }

    build.compile("silabs_csdk");
    println!("cargo:rustc-link-lib=static=silabs_csdk");
}
