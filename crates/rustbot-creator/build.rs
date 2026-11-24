//! Embeds the icon and the file details in the Windows program (D-155).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon = "../../packaging/windows/rustbot-creator.ico";
        println!("cargo:rerun-if-changed={icon}");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(icon)
            .set("FileDescription", "RustBot Creator")
            .set("ProductName", "RustBot Creator");
        if let Err(error) = resource.compile() {
            panic!("the Windows resources cannot be compiled: {error}");
        }
    }
}
