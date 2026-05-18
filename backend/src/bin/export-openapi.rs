use shelfie_backend::openapi::ApiDoc;
use std::fs;
use std::path::PathBuf;
use utoipa::OpenApi;

fn main() {
    let spec = ApiDoc::openapi().to_pretty_json().expect("serialize openapi");
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("openapi")
        .join("openapi.json");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).expect("create openapi dir");
    }
    fs::write(&out, spec).expect("write openapi.json");
    println!("{}", out.display());
}
