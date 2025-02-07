extern crate embed_resource;

fn main() {
    embed_resource::compile("src/logo.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
