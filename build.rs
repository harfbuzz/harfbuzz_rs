#[cfg(feature = "build-native-harfbuzz")]
extern crate cc;
#[cfg(feature = "build-native-harfbuzz")]
extern crate pkg_config;

#[cfg(feature = "build-native-harfbuzz")]
fn main() {
    use std::env;

    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-env-changed=HARFBUZZ_SYS_NO_PKG_CONFIG");
    if (target.contains("wasm32") || env::var_os("HARFBUZZ_SYS_NO_PKG_CONFIG").is_none())
        && pkg_config::probe_library("harfbuzz").is_ok()
    {
        return;
    }

    let mut cfg = cc::Build::new();
    cfg.cpp(true)
        .flag("-std=c++11")
        .warnings(false)
        .include("harfbuzz/src")
        .file("harfbuzz/src/harfbuzz.cc");

    if !target.contains("windows") {
        cfg.define("HAVE_PTHREAD", "1");
    }

    // if target.contains("apple") {
    //     cfg.define("HAVE_CORETEXT", "1");
    // }

    if target.contains("windows-gnu") {
        cfg.flag("-Wa,-mbig-obj");
    }

    cfg.compile("embedded_harfbuzz");
}

#[cfg(not(feature = "build-native-harfbuzz"))]
fn main() {}
