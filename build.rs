use std::env;
use std::path::PathBuf;

fn main() {
    let kernel_path = env::vars_os()
        .find(|(key, _)| key.to_string_lossy().starts_with("CARGO_BIN_FILE_ATULYAOS_KERNEL"))
        .map(|(_, value)| PathBuf::from(value))
        .expect("kernel artifact path not found");

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
    println!("cargo:rerun-if-changed=kernel/src");
    println!("cargo:rerun-if-changed=assets/boot");
    println!("cargo:rerun-if-changed=assets/boot_frames");
}
