```
LD_LIBRARY_PATH=c-src cargo run
```
Compilation of Haskell to Rust:
+ ~/sip-processor/hs_lib.c from https://www.vex.net/~trebla/haskell/so.xhtml
+ ~/sip-processor/Lib.hs contains all haskell exported functions
+ ~/RemoteIO-app/c-src contains wrapper.h which `#include`s all haskell libs with format `<lib>_stub_act.h`
+ ~/RemoteIO-app/c-src also contains `HsFFI.h` which has typedefs for haskell types
    + `HsFFI.h` will contain c typedefs for any value types passed between foreign export calls Haskell -> Rust
+ ~/RemoteIO-app/build.rs does the following tasks:
    + copies hs_lib.c, Lib.hs into ~/sip-processor/c-bind (tmp dir)
    + compiles Lib.hs into a shared library `libLib.so` generating `Lib_stub.h` as well
    + copies `libLib.so` and `Lib_stub.h` into ~/RemoteIO-app/c-src
    + creates file `Lib_stub_act.h` which excludes the first line (hard coded dont include their first include, it assumes the stub will be compiled with ghc instead of being dynamically linked with a .so file which has the appropriate libraries at runtime)
    + links rust compilation with `libLib.so` and uses bindgen to allow rust to use haskell bindings
NOTE: Normally `hs_init` and `hs_exit` must be used to initialize and teardown the haskell runtime environment. The code provided by `trebla` does this at initialization and teardown of the program via the shared library (don't totally understand this ngl but i hope it doesnt break for now!) 