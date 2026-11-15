fn main() {
    println!("cargo:rerun-if-changed=assets/winy.ico");
    println!("cargo:rerun-if-changed=assets/winy.rc");
    embed_resource::compile("assets/winy.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("Compile application icon");
}
