use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let kernel_path = manifest_dir
        .join("target")
        .join("x86_64-unknown-none")
        .join("release")
        .join("atulyaos-kernel");

    if !kernel_path.exists() {
        panic!(
            "AtulyaOS kernel ELF not found at {:?}.\n\
             Build it first with:\n\
             cargo build -p atulyaos-kernel --target x86_64-unknown-none --release",
            kernel_path
        );
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR missing"));
    let bios_image = out_dir.join("atulyaos-bios.img");

    let mut boot_config = bootloader::BootConfig::default();
    boot_config.frame_buffer.minimum_framebuffer_width = Some(1920);
    boot_config.frame_buffer.minimum_framebuffer_height = Some(1080);
    boot_config.frame_buffer_logging = false;

    let mut bios_boot = bootloader::BiosBoot::new(&kernel_path);
    bios_boot
        .set_boot_config(&boot_config)
        .create_disk_image(&bios_image)
        .expect("failed to create BIOS boot image");

    println!("cargo:rustc-env=ATULYAOS_BIOS_IMAGE={}", bios_image.display());
    println!("cargo:rerun-if-changed={}", kernel_path.display());
}
