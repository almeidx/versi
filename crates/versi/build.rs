use std::path::Path;

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let icon_path = Path::new(manifest_dir)
        .join("../../assets/icon.ico")
        .canonicalize()
        .expect("Failed to find icon.ico");

    println!("cargo:rerun-if-changed={}", icon_path.display());

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon_path.to_str().expect("Invalid icon path"));
        res.compile().expect("Failed to compile Windows resources");
    }
}
