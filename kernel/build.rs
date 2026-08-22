fn main() {
    let manifest =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let linker = format!("{}/linker_x86_64.ld", manifest);

    // Hand the Limine higher-half linker script to the linker (rust-lld on
    // x86_64-unknown-none).
    println!("cargo:rustc-link-arg=-T{}", linker);

    println!("cargo:rerun-if-changed={}", linker);
    println!("cargo:rerun-if-changed=src");
}
