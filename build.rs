use std::{fs, process};

const REQUIRED_SUBMODULE_FILES: &[&str] = &["deps/service/water-rights/resources/init.sql"];

fn main() {
    // ensure that required files may be found before actual code compiles and confuses the user
    for file in REQUIRED_SUBMODULE_FILES {
        let file_exists = fs::metadata(file).is_ok();
        if !file_exists {
            // TODO: clear this up when https://github.com/rust-lang/cargo/pull/11312 lands
            eprintln!("could not find {file:?}, make sure you also cloned submodules");
            process::exit(1);
        }
    }

    // print rerun-if-changed with files here to avoid spam in previous section
    for file in REQUIRED_SUBMODULE_FILES {
        println!("cargo:rerun-if-changed={file}");
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.gitmodules");
}
