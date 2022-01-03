extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn main() {

    cc::Build::new()
        .file("c-src/hello-world.c")
        .compile("libhello-world.a");
    
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("c-src/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings!");
    
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}