use std::path::Path;
use std::process::Command;

fn main() {
    // ── Paths ───────────────────────────────────────────────────────────────
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let data_dir = Path::new(&manifest_dir).join("data");
    let xml_path = data_dir.join("resources.gresource.xml");
    let out_path = data_dir.join("resources.gresource");

    // ── Tell Cargo to re-run this script when icons or the manifest change ──
    println!("cargo:rerun-if-changed=data/resources.gresource.xml");
    println!("cargo:rerun-if-changed=data/icons/");

    // ── Compile the GResource bundle ────────────────────────────────────────
    let status = Command::new("glib-compile-resources")
        .arg("--sourcedir")
        .arg(&data_dir)
        .arg(xml_path.to_str().unwrap())
        .arg("--target")
        .arg(out_path.to_str().unwrap())
        .status()
        .expect(
            "Failed to run `glib-compile-resources`. \
             Make sure the `libglib2.0-dev-bin` (or equivalent) package is installed.",
        );

    if !status.success() {
        panic!("`glib-compile-resources` exited with status: {status}");
    }
}
