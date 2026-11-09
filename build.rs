// On Windows, embed a manifest that asks for Administrator rights, as the
// original does (its manifest: requestedExecutionLevel requireAdministrator),
// so clicks reach a game that runs as Administrator.
fn main() {
    println!("cargo:rerun-if-changed=windows-res/app.rc");
    println!("cargo:rerun-if-changed=windows-res/app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("windows-res/app.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
