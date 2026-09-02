use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sdk_path() -> PathBuf {
    env::var("SILABS_SDK")
        .map(PathBuf::from)
        .expect("SILABS_SDK must be set to the Simplicity SDK root directory")
}

fn link_lib(search: &Path, name: &str) {
    if search.join(format!("lib{name}.a")).exists() {
        println!("cargo:rustc-link-search=native={}", search.display());
        println!("cargo:rustc-link-lib=static={name}");
    } else {
        println!("cargo:warning=BLE library lib{name}.a not found in {}", search.display());
    }
}

/// Bundle objects into a static library (link-lib propagates to bins).
fn link_archive_objects(out_dir: &Path, archive: &Path, members: &[&str], lib_name: &str) {
    if !archive.exists() {
        println!(
            "cargo:warning=BLE archive {} not found; advertiser commands may be stubbed",
            archive.display()
        );
        return;
    }

    let extract_dir = out_dir.join(lib_name);
    fs::create_dir_all(&extract_dir).unwrap();
    println!("cargo:rerun-if-changed={}", archive.display());

    let mut object_paths = Vec::new();
    for member in members {
        let obj_path = extract_dir.join(member);
        if !obj_path.exists() {
            let status = Command::new("arm-none-eabi-ar")
                .arg("x")
                .arg(archive)
                .arg(member)
                .current_dir(&extract_dir)
                .status()
                .unwrap_or_else(|err| panic!("failed to run arm-none-eabi-ar: {err}"));
            if !status.success() {
                panic!(
                    "failed to extract {member} from {}",
                    archive.display()
                );
            }
        }
        object_paths.push(obj_path);
    }

    write_static_lib(out_dir, lib_name, &object_paths);
}

fn write_static_lib(out_dir: &Path, lib_name: &str, object_paths: &[PathBuf]) {
    let lib_path = out_dir.join(format!("lib{lib_name}.a"));
    let mut ar = Command::new("arm-none-eabi-ar");
    ar.arg("rcs").arg(&lib_path);
    for obj in object_paths {
        ar.arg(obj);
    }
    let status = ar
        .status()
        .unwrap_or_else(|err| panic!("failed to run arm-none-eabi-ar: {err}"));
    if !status.success() {
        panic!("failed to create {}", lib_path.display());
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static={lib_name}");
}

fn link_ble_libraries(sdk: &Path, out_dir: &Path, psa_crypto: bool) {
    let host = sdk.join("bluetooth_le_host/build/gcc/cortex-m33");

    // Real advertiser + GAP code must be linked before libble_bgapi.a, which also
    // contains weak *-stubs.c.obj files that return SL_STATUS_NOT_SUPPORTED (0x0f).
    link_archive_objects(
        out_dir,
        &host.join("bgstack/release/libble_host.a"),
        &["gap_adv.c.obj"],
        "silabs_ble_gap_adv",
    );
    link_archive_objects(
        out_dir,
        &host.join("ble_bgapi/release/libble_bgapi.a"),
        &[
            "bgapi_advertiser.c.obj",
            "bgapi_legacy_advertiser.c.obj",
        ],
        "silabs_ble_bgapi_advertiser",
    );

    let crypto_lib: (&str, &str) = if psa_crypto {
        (
            "crypto/crypto_lib/crypto_lib_psa/release",
            "ble_host_crypto_lib_psa",
        )
    } else {
        ("crypto/crypto_lib/release", "ble_host_crypto_lib_stub")
    };
    let host_libs: [(&str, &str); 15] = [
        ("accept_list/release", "ble_host_accept_list_stub"),
        ("bgstack/release", "ble_host"),
        ("bgstack/release", "bondingdb_stub"),
        ("ble_bgapi/release", "ble_bgapi"),
        ("ble_system/release", "ble_system"),
        ("core/release", "ble_host_core"),
        crypto_lib,
        ("crypto/release", "ble_host_crypto_stub"),
        ("hal/release", "ble_host_hal_series2"),
        ("hci/release", "ble_host_hci"),
        ("privacy/local_privacy/release", "ble_host_local_privacy"),
        (
            "privacy/resolving_list/release",
            "ble_host_resolving_list_stub",
        ),
        (
            "privacy/rpa_resolution/release",
            "ble_host_rpa_resolution_stub",
        ),
        ("system/release", "ble_host_system"),
        (
            "connection_subrating/release",
            "ble_host_connection_subrating_stub",
        ),
    ];

    for (subdir, lib) in host_libs {
        link_lib(&host.join(subdir), lib);
    }

    link_lib(
        &host.join("advertiser/periodic_advertiser/release"),
        "ble_host_periodic_advertiser_stub",
    );

    // Mesh PB-GATT provisioning uses BGAPI GATT server helpers.
    link_lib(
        &host.join("ble_bgapi/release"),
        "ble_bgapi_gatt_server",
    );
    link_lib(
        &host.join("ble_bgapi/release"),
        "ble_bgapi_stub_gatt_client",
    );

    link_lib(
        &sdk.join("bluetooth_le_controller/build/gcc/xg24/release"),
        "linklayer",
    );

    link_lib(
        &sdk.join("bluetooth_common/lib/build/gcc/cortex-m33/bgcommon/release"),
        "bgcommon",
    );

    let bgapi = sdk.join("bgapi_protocol/build/gcc/cortex-m33");
    // 2026.12+: protocol/task (libbgapi_task.a) provides sli_bgapi_task_step /
    // sli_bgapi_shared_task used by host core timer + sl_bt_run.
    let bgapi_libs: [(&str, &str); 8] = [
        ("bgapi_trace/release", "bgapi_trace_stub"),
        ("protocol/command/release", "bgapi_command"),
        ("protocol/core/release", "bgapi_core"),
        ("protocol/device/release", "bgapi_device"),
        ("protocol/event/release", "bgapi_event"),
        ("protocol/task/release", "bgapi_task"),
        ("protocol/release", "bgapi_protocol"),
        (
            "rtos_adaptation/release",
            "bgapi_protocol_rtos_adaptation_stub",
        ),
    ];
    for (subdir, lib) in bgapi_libs {
        link_lib(&bgapi.join(subdir), lib);
    }
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sdk = sdk_path();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-env-changed=SILABS_SDK");
    println!("cargo:rerun-if-changed=build.rs");

    if target.starts_with("thumbv") && env::var("CARGO_FEATURE_BLE").is_ok() {
        let psa_crypto = env::var("CARGO_FEATURE_PSA_CRYPTO").is_ok();
        link_ble_libraries(&sdk, &out_path, psa_crypto);
    }

    let header_path = sdk.join("bluetooth_le_host/inc/sl_bt_api.h");
    if header_path.exists() {
        let pc = sdk.join("platform_core/platform");
        let clang_args = vec![
            format!("-I{}", manifest_dir.join("include").display()),
            format!(
                "-I{}",
                sdk.join("bluetooth_le_host/inc").display()
            ),
            format!(
                "-I{}",
                sdk.join("bluetooth_le_host/config").display()
            ),
            format!(
                "-I{}",
                sdk.join("bgapi_protocol/protocol/inc").display()
            ),
            format!(
                "-I{}",
                sdk.join("bgapi_protocol/config").display()
            ),
            format!("-I{}", sdk.join("bluetooth_common/inc").display()),
            format!(
                "-I{}",
                sdk.join("bluetooth_common/config").display()
            ),
            format!("-I{}", pc.join("common/inc").display()),
            format!(
                "-I{}",
                pc.join("Device/SiliconLabs/EFR32MG24/Include").display()
            ),
            format!("-I{}", sdk.join("cmsis/Core/Include").display()),
            format!("-I{}", manifest_dir.join("include").display()),
            "-ffreestanding".to_string(),
            "-Wno-everything".to_string(),
            "-DEFR32MG24B220F1536IM48".to_string(),
            "-DSL_COMPONENT_CATALOG_PRESENT".to_string(),
            "-D__GNUC__".to_string(),
        ];

        let bindings = bindgen::Builder::default()
            .header(header_path.to_str().unwrap())
            .clang_args(&clang_args)
            .use_core()
            .ctypes_prefix("cty")
            .layout_tests(false)
            .allowlist_type("sl_bt_msg_t")
            .allowlist_type("sl_bt_.*")
            .allowlist_var("sl_bt_.*_id")
            .allowlist_function("sl_bt_.*")
            .allowlist_type("sl_status_t")
            .generate()
            .expect("Unable to generate BLE bindings");

        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings");
    } else {
        std::fs::write(
            out_path.join("bindings.rs"),
            "// BLE bindings not generated (SDK headers not found)\n",
        )
        .expect("Couldn't write placeholder bindings");
        println!(
            "cargo:warning=BLE headers not found. Set SILABS_SDK to your Simplicity SDK install."
        );
    }
}
