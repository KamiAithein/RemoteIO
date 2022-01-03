extern crate bindgen;

use std::fs::File;
use std::io::prelude::*;
use std::io::{Write, BufReader, BufRead};
use std::env;
use std::path::{PathBuf, Path};
use std::{process::Command};

//reference https://github.com/mgattozzi/curryrs/blob/master/rust/build.rs
//          https://github.com/mgattozzi/curryrs

fn create_library(/*_name: &str, _dir: &str*/) {
    let _cp_c_out = Command::new("cp")
                          .args(["hs_lib.c", "../c-bind/hs_lib.c"])
                          .current_dir("../sip-processor/c-src")
                          .output()
                          .expect("failed to move link c-src files!");

    let _cp_hs_out = Command::new("cp")
        .args(["Lib.hs", "../c-bind/Lib.hs"])
        .current_dir("../sip-processor/src")
        .output()
        .expect("failed to move link src files!");

    let _output = Command::new("ghc")
                         .args(["-dynamic", "-shared", "-fPIC", "-o", "libLib.so", "Lib.hs", "hs_lib.c", "-lHSrts-ghc8.6.5"]) //@TODO magic ghc version
                         .current_dir("../sip-processor/c-bind")
                         .output()
                         .expect("failed to create library!");
    return ();
}

fn move_library(/*_name: &str, _dir_from: &str, _dir_to: &str*/) -> std::io::Result<()>{
    Command::new("cp")
            .current_dir("../sip-processor/c-bind")
            .args(["libLib.so", "../../RemoteIO-app/c-src/libLib.so"])
            .output()
            .expect("could not move dynamic library!");

    Command::new("cp")
            .current_dir("../sip-processor/c-bind")
            .args(["Lib_stub.h", "../../RemoteIO-app/c-src/Lib_stub.h"])
            .output()
            .expect("could not move dynamic library!");

    return Ok(());
}

fn main() {
    create_library();
    move_library();
    println!("cargo:rustc-link-search=native=c-src");
    println!("cargo:rustc-link-lib=dylib=Lib");

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