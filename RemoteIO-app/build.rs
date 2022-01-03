extern crate bindgen;

use std::env;
use std::path::{PathBuf, Path};
use std::{process::Command};

//reference https://github.com/mgattozzi/curryrs/blob/master/rust/build.rs
//          https://github.com/mgattozzi/curryrs

fn create_library(/*_name: &str, _dir: &str*/) {
    let _output = Command::new("ghc")
                         .args(["-dynamic", "-shared", "-fPIC", "-o", "libLib.so", "Lib.hs", "hs_lib.c", "-lHSrts-ghc8.6.5"]) //@TODO magic ghc version
                         .current_dir("../sip-processor/c-bind")
                         .output()
                         .expect("failed to create library!");
    return ();
}

fn move_library(/*_name: &str, _dir_from: &str, _dir_to: &str*/) {
    Command::new("touch").current_dir("../sip-processor/c-bind").args(["jomama"]).output().expect("fail!");
    Command::new("cp").current_dir("../sip-processor/c-bind").args(["libLib.so", "../../RemoteIO-app/c-src/libLib.so"]).output().expect("aa");
    // Command::new("cp")
    //         .args(["libLib.so", "../../RemoteIO-app/c-bind/libLib.so"])
    //         .current_dir("../sip-processor/c-bind")
    //         .output()
    //         .expect("failed to move library!");
    return ();
}

fn main() {
    // cc::Build::new()
    //     .file("../sip-processor/c-bind/hs_lib.c")
    //     .compile("libhs_lib.a");
    
    // println!("cargo:rerun-if-changed=wrapper.h");

    // let bindings = bindgen::Builder::default()
    //     .header("../sip-processor/c-bind/wrapper.h")
    //     .parse_callbacks(Box::new(bindgen::CargoCallbacks))
    //     .generate()
    //     .expect("Unable to generate bindings!");
    
    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // bindings
    //     .write_to_file(out_path.join("bindings.rs"))
    //     .expect("Couldn't write bindings!");
    
    // cc::Build::new()
    //     .file("c-src/hello-world.c")
    //     .compile("libhello-world.a");
    // let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // println!("cargo:rustc-link-search=native={}", Path::new(&dir).join("c-src").display());
    create_library();
    move_library();
    println!("cargo:rustc-link-search=native=c-src");
    println!("cargo:rustc-link-lib=dylib=Lib");
    // println!("cargo:rustc-link-lib=dylib=hello-world");

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