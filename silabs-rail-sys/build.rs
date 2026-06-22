use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sdk_path() -> PathBuf {
    env::var("SILABS_SDK")
        .map(PathBuf::from)
        .expect("SILABS_SDK must be set to the Simplicity SDK root directory")
}

fn link_rail_blob(sdk: &Path, multiprotocol: bool) {
    let blob_dir = sdk.join("rail_library/autogen/librail_release");
    let blob_name = if multiprotocol {
        "librail_multiprotocol_efr32xg24_gcc_release.a"
    } else {
        "librail_efr32xg24_gcc_release.a"
    };
    let blob_path = blob_dir.join(blob_name);

    if blob_path.exists() {
        println!("cargo:rustc-link-search=native={}", blob_dir.display());
        let lib = blob_name.trim_start_matches("lib").trim_end_matches(".a");
        println!("cargo:rustc-link-lib=static={lib}");
        println!("cargo:warning=Linking RAIL blob: {}", blob_path.display());
    } else {
        println!(
            "cargo:warning=RAIL blob not found at {}. Set SILABS_SDK to your Simplicity SDK install.",
            blob_path.display()
        );
    }
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
    println!("cargo:rustc-link-arg=-Wl,--whole-archive");
    println!("cargo:rustc-link-lib=static={lib_name}");
    println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
}

/// Pull RAIL radio IRQ handler objects into the link (cortex-m-rt device.x references them).
fn link_rail_irq_handlers(sdk: &Path, out_dir: &Path, multiprotocol: bool) {
    let blob_name = if multiprotocol {
        "librail_multiprotocol_efr32xg24_gcc_release.a"
    } else {
        "librail_efr32xg24_gcc_release.a"
    };
    let archive = sdk
        .join("rail_library/autogen/librail_release")
        .join(blob_name);
    if !archive.exists() {
        println!("cargo:warning=RAIL archive missing for IRQ handlers: {}", archive.display());
        return;
    }

    let extract_dir = out_dir.join("silabs_rail_irq");
    fs::create_dir_all(&extract_dir).unwrap();
    println!("cargo:rerun-if-changed={}", archive.display());

    let members = ["generic_phy.o", "rfhal_mailbox_handlers.o"];
    let mut object_paths = Vec::new();
    for member in members {
        let obj_path = extract_dir.join(member);
        if !obj_path.exists() {
            let status = Command::new("arm-none-eabi-ar")
                .arg("x")
                .arg(&archive)
                .arg(member)
                .current_dir(&extract_dir)
                .status()
                .unwrap_or_else(|err| panic!("failed to run arm-none-eabi-ar: {err}"));
            if !status.success() {
                panic!("failed to extract {member} from {}", archive.display());
            }
        }
        object_paths.push(obj_path);
    }

    write_static_lib(out_dir, "silabs_rail_irq", &object_paths);

    for sym in [
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
    ] {
        println!("cargo:rustc-link-arg=-Wl,-u,{sym}");
    }
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sdk = sdk_path();

    println!("cargo:rerun-if-env-changed=SILABS_SDK");
    println!("cargo:rerun-if-changed=build.rs");

    if target.starts_with("thumbv") {
        let multiprotocol = env::var("CARGO_FEATURE_RAIL_MULTIPROTOCOL").is_ok();
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        link_rail_irq_handlers(&sdk, &out_path, multiprotocol);
        link_rail_blob(&sdk, multiprotocol);
    }

    let header_path = sdk.join("rail_library/common/rail.h");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    if header_path.exists() {
        let pc = sdk.join("platform_core/platform");
        let clang_args = vec![
            format!("-I{}", manifest_dir.join("include").display()),
            format!("-I{}", sdk.join("rail_library/common").display()),
            format!(
                "-I{}",
                sdk.join("rail_library/chip/efr32/efr32xg2x").display()
            ),
            format!("-I{}", pc.join("common/inc").display()),
            format!("-I{}", pc.join("common/inc/sli").display()),
            format!(
                "-I{}",
                pc.join("Device/SiliconLabs/EFR32MG24/Include").display()
            ),
            format!("-I{}", sdk.join("cmsis/Core/Include").display()),
            "-ffreestanding".to_string(),
            "-Wno-everything".to_string(),
            "-DEFR32MG24B220F1536IM48".to_string(),
            "-D__GNUC__".to_string(),
        ];

        let bindings = bindgen::Builder::default()
            .header(header_path.to_str().unwrap())
            .clang_args(&clang_args)
            .use_core()
            .ctypes_prefix("cty")
            .layout_tests(false)
            .allowlist_function("RAIL_.*")
            .allowlist_function("sl_rail_.*")
            .allowlist_type("RAIL_.*")
            .allowlist_type("sl_rail_.*")
            .allowlist_var("RAIL_.*")
            .allowlist_var("SL_RAIL_.*")
            .generate()
            .expect("Unable to generate RAIL bindings");

        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings");

        println!(
            "cargo:warning=Generated RAIL bindings from {}",
            header_path.display()
        );
    } else {
        std::fs::write(
            out_path.join("bindings.rs"),
            "// RAIL bindings not generated (SDK headers not found)\n",
        )
        .expect("Couldn't write placeholder bindings");
        println!(
            "cargo:warning=RAIL headers not found at {}. Set SILABS_SDK.",
            header_path.display()
        );
    }
}
