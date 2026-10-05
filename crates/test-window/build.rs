fn main() {
    // The manifest asks for common controls 6 (wxWidgets insists on it) and
    // per-monitor DPI awareness. Windows only; the other systems need none.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo::rerun-if-changed=test-window.rc");
        println!("cargo::rerun-if-changed=test-window.exe.manifest");
        embed_resource::compile("test-window.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
