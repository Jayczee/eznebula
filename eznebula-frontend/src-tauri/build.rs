fn main() {
    tauri_build::build();

    #[cfg(windows)]
    {
        use std::path::PathBuf;
        // Use bundled Npcap SDK .lib files for linking
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let lib_dir = manifest_dir.join("npcap").join("Lib").join("x64");
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=wpcap");
        println!("cargo:rustc-link-lib=Packet");
    }
}
