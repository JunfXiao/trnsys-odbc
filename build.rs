use std::env;
use std::fs;
use std::path::Path;

fn read_toml_trnsys_metadata() -> toml::Value {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let cargo_toml_path = Path::new(&manifest_dir).join("Cargo.toml");

    let cargo_toml_content = fs::read_to_string(cargo_toml_path).unwrap();

    cargo_toml_content
        .parse::<toml::Value>()
        .unwrap()
        .get("package")
        .and_then(|pkg| pkg.get("metadata"))
        .and_then(|meta| meta.get("trnsys"))
        .expect("Failed to find 'package.metadata.trnsys' in Cargo.toml")
        .to_owned()
}

fn write_entrance_code(type_number: &str) {
    // Get Output Dir
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_entrance.rs");

    // Dynamic generate the type entrance
    let function_code = format!(
        r#"
        #[allow(non_snake_case)]
        #[unsafe(no_mangle)]
        pub extern "C" fn TYPE{}() {{
            entrance();
        }}
        const TYPE_NUMBER: u32 = {};
        "#,
        type_number, type_number
    );

    // Write code to the file
    fs::write(&dest_path, function_code).unwrap();
}
fn main() {
    // Tell Cargo that if the given file changes, to rerun this build script.
    watch_trndll64();

    // Read type number
    let metadata = read_toml_trnsys_metadata();

    if let Some(type_number) = metadata.get("type_number").unwrap().as_integer() {
        println!("Found type number: {}", type_number);

        write_entrance_code(&type_number.to_string());
    } else {
        panic!("Cannot find type number in `Cargo.toml`. Please add it before building.")
    }

    // Add Windows resources (icon, version info, etc.)
    let mut res = winres::WindowsResource::new();
    res.set(
        "FileDescription",
        "Type 395 HAMT Model",
    );
    res.set("ProductName", "TRNSYS HAMT");
    res.set("OriginalFilename", "trnsys_hamt_rs.dll");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap();

    let authors = std::env::var("CARGO_PKG_AUTHORS").unwrap();

    res.set(
        "LegalCopyright",
        &format!("Copyright © 2026 {authors}. All rights reserved.W"),
    );

    let version_parts: Vec<&str> = version.split('.').collect();
    if version_parts.len() >= 3 {
        res.set_version_info(
            winres::VersionInfo::FILEVERSION,
            parse_version(version_parts[0], version_parts[1], version_parts[2], "0"),
        );
        res.set_version_info(
            winres::VersionInfo::PRODUCTVERSION,
            parse_version(version_parts[0], version_parts[1], version_parts[2], "0"),
        );
    }

    res.compile().unwrap();
}

fn parse_version(major: &str, minor: &str, patch: &str, build: &str) -> u64 {
    let v1 = major.parse::<u16>().unwrap_or(0) as u64;
    let v2 = minor.parse::<u16>().unwrap_or(0) as u64;
    let v3 = patch.parse::<u16>().unwrap_or(0) as u64;
    let v4 = build.parse::<u16>().unwrap_or(0) as u64;
    
    (v1 << 48) | (v2 << 32) | (v3 << 16) | v4
}

/// Watch the TRNDll64.lib file and rebuild if it changes
fn watch_trndll64() {
    println!("cargo::rerun-if-changed=lib/TRNDll64.lib");
    println!("cargo:rustc-link-search=native=lib");
    println!("cargo:rustc-link-lib=static=TRNDll64");
}
