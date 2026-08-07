fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations(concat!(env!("CARGO_MANIFEST_DIR"), "/locales/"));
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint build failed");
}
