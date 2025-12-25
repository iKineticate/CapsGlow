fn main() {
    embed_resource::compile("assets/CapsGlow.exe.manifest.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
