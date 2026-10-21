use std::env;
use std::path::{Path, PathBuf};

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
        println!("cargo:warning=Mesh library lib{name}.a not found in {}", search.display());
    }
}

fn link_mesh_libraries(sdk: &Path) {
    let mesh = sdk.join("bluetooth_mesh/build/gcc/cortex-m33");
    let mesh_libs: [(&str, &str); 8] = [
        ("ble_mesh/release", "btmesh_core"),
        ("ble_mesh/release", "btmesh_crypto_common_cache"),
        ("ble_mesh/release", "btmesh_crypto_psa"),
        ("ble_mesh/release", "btmesh_its_keystorage"),
        ("hal/release", "btmesh_hal"),
        ("hal/release", "btmesh_hal_obfuscated_nvm"),
        ("hal/release", "btmesh_hal_psa"),
        ("hal/release", "btmesh_model_storage_v2"),
    ];

    for (subdir, lib) in mesh_libs {
        link_lib(&mesh.join(subdir), lib);
    }

    link_lib(
        &mesh.join("ble_mesh/release"),
        "btmesh_crypto_key_cache",
    );
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sdk = sdk_path();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-env-changed=SILABS_SDK");
    println!("cargo:rerun-if-changed=build.rs");

    if target.starts_with("thumbv") && env::var("CARGO_FEATURE_BTMESH").is_ok() {
        link_mesh_libraries(&sdk);
    }

    let header_path = sdk.join("bluetooth_mesh/inc/sl_btmesh_api.h");
    if header_path.exists() {
        let pc = sdk.join("platform_core/platform");
        let clang_args = vec![
            format!("-I{}", manifest_dir.join("include").display()),
            format!("-I{}", sdk.join("bluetooth_mesh/inc").display()),
            format!("-I{}", sdk.join("bluetooth_mesh/config").display()),
            format!(
                "-I{}",
                sdk.join("bluetooth_le_host/inc").display()
            ),
            format!(
                "-I{}",
                sdk.join("bgapi_protocol/protocol/inc").display()
            ),
            format!(
                "-I{}",
                sdk.join("bgapi_protocol/config").display()
            ),
            format!("-I{}", pc.join("common/inc").display()),
            format!(
                "-I{}",
                pc.join("Device/SiliconLabs/EFR32MG24/Include").display()
            ),
            format!("-I{}", sdk.join("cmsis/Core/Include").display()),
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
            .allowlist_type("sl_btmesh_msg_t")
            .allowlist_type("sl_btmesh_.*")
            .allowlist_var("sl_btmesh_.*_id")
            .allowlist_function("sl_btmesh_.*")
            .generate()
            .expect("Unable to generate Mesh bindings");

        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings");
    } else {
        std::fs::write(
            out_path.join("bindings.rs"),
            "// Mesh bindings not generated (SDK headers not found)\n",
        )
        .expect("Couldn't write placeholder bindings");
        println!(
            "cargo:warning=Mesh headers not found. Set SILABS_SDK to your Simplicity SDK install."
        );
    }
}
