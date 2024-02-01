
fn main() {
    /*
    let lib_dir = var("DITTOFFI_SEARCH_PATH")
        .expect("Set env var DITTOFFI_SEARCH_PATH");
    let out_dir = var("OUT_DIR").unwrap();
    let lib_path = PathBuf::from(lib_dir).join("libdittoffi.so");

    Command::new("cp")
            .arg(lib_path.clone())
            .arg(out_dir.clone())
            .status()
            .expect(format!("failed to copy {} to {}", lib_path.display(), out_dir).as_str());
    */

    // Experimenting with static linking...
    //println!("cargo:rustc-link-search=/lib/gcc/x86_64-linux-gnu/11");
    //println!("cargo:rustc-link-search=/usr/lib/x86_64-linux-gnu/");
}
