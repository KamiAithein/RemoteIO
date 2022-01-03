#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::CString;
use std::ffi::c_void;

fn main() {
    unsafe {
        // hello_world();
        helloFromHaskell();
        jomamaFromHaskell();
        let mut pass: CString = CString::new("Hello World!").expect("could not create CString");

        let mut fail: CString = CString::new("jomama!").expect("could not create CString");

        println!("pass: {}\nfail: {}", isHelloWorld(pass.as_ptr() as *mut c_void), isHelloWorld(fail.as_ptr() as *mut c_void));
    }
}
