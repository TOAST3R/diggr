//! Regenerates the bundled default skin: `cargo run -p ui --bin skin-gen`.

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/skin/default");
    std::fs::create_dir_all(&dir).expect("create skin dir");
    let (img, def) = ui::skin::generate::generate();
    img.save(dir.join(&def.atlas)).expect("write atlas");
    let text =
        ron::ser::to_string_pretty(&def, ron::ser::PrettyConfig::default()).expect("serialize");
    std::fs::write(dir.join("skin.ron"), text).expect("write skin.ron");
    println!(
        "wrote {} ({}×{}) and skin.ron to {}",
        def.atlas,
        img.width(),
        img.height(),
        dir.display()
    );
}
