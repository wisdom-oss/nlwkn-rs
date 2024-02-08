/// This build script will download the required dependencies during build time.
/// This is not considered best practice but other options seem way more bloated
/// than this.
use std::path::PathBuf;
use std::{env, fs};

use static_toml::static_toml;

static_toml! {
    static CARGO_TOML = include_toml!("Cargo.toml");
}

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("set by cargo"));
    let target_dir = out_dir
        .parent()
        .expect("out")
        .parent()
        .expect("build id")
        .parent()
        .expect("build")
        .parent()
        .expect("profile");
    let mut resource_dir = target_dir.to_path_buf();
    resource_dir.push("resources");
    fs::create_dir_all(&resource_dir).unwrap();

    let client = reqwest::blocking::Client::new();
    for resource in CARGO_TOML.package.metadata.resources.iter() {
        let out_path = resource_dir.join(resource.path);
        if let Ok(meta) = fs::metadata(&out_path) {
            if meta.is_file() {
                continue;
            }
        }

        let res = client.get(resource.url).send().unwrap();
        let text = res.text().unwrap();

        fs::write(&out_path, text).unwrap();
        let out_path = out_path.to_string_lossy();
        println!("cargo:rerun-if-changed={}", out_path);
    }
}
