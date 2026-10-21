use std::env;

fn use_system_harfbuzz() -> bool {
    println!("cargo:rerun-if-env-changed=HARFBUZZ_RS_USE_PKG_CONFIG");
    if !cfg!(feature = "build-native-harfbuzz") {
        return true;
    }
    match env::var_os("HARFBUZZ_RS_USE_PKG_CONFIG") {
        Some(v) => !v.is_empty() && v != "0",
        None => false,
    }
}

fn main() {
    if use_system_harfbuzz() {
        pkg_config::probe_library("harfbuzz").unwrap();
        return;
    }

    let target = env::var("TARGET").unwrap();

    let mut cfg = cc::Build::new();
    cfg.cpp(true)
        .flag("-std=c++11")
        .flag("-O2")
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
