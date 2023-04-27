use std::path::Path;
use std::{env, fs, path::PathBuf, process::Command};

fn lrc_include_dir() -> String{
    match env::var("LRC_INCLUDE_DIR") {
        Ok(val) => val,
        Err(_) => "lrc-erasure-code/include".to_string()
    }
}
fn fail_on_empty_directory(name: &str) {
    if fs::read_dir(name).unwrap().count() == 0 {
        println!("The `{name}` directory is empty, did you forget to pull the submodules?");
        println!("Try `git submodule update --init --recursive`");
        panic!();
    }
}
fn try_to_find_and_link_lib(lib_name: &str) -> bool {
    println!("cargo:rerun-if-env-changed={lib_name}_COMPILE");
    if let Ok(v) = env::var(format!("{}_COMPILE", lib_name)) {
        if v.to_lowercase() == "true" || v == "1" {
            return false
        }
    }
    println!("cargo:rerun-if-env-changed={lib_name}_STATIC");
    println!("cargo:rerun-if-env-changed={lib_name}_LIB_DIR");
    if let Ok(lib_dir) = env::var(format!("{}_LIB_DIR", lib_name)) {
        println!("cargo:rustc-link-search=native={}", lib_dir);
        let mode = match env::var_os(format!("{lib_name}_STATIC")) {
            Some(_) => "static",
            None => "dylib"
        };
        println!("cargo:rustc-link-lib={}={}", mode, lib_name.to_lowercase());        
        return true
    }
    false
}

fn bindgen_lrc(){
    let bindings = bindgen::Builder::default()
        .header(lrc_include_dir() + "/lrc.h")
        .derive_debug(true)
        .derive_default(true)
        .allowlist_function("lrc_.*")
        .allowlist_type("lrc_.*")
        .allowlist_var("LRC_.*")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate bindings for lrc.h");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings for lrc.h!");
}

fn update_submodules() {
    let program = "git";
    let dir = "../";
    let args = &["submodule", "update", "--init", "--recursive"];
    println!("Running command: \"{} {}\" in dir {}", program, args.join(" "), dir);
    let ret = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status();
    match ret.map(|status| (status.success(), status.code())) {
        Ok((true, _)) => (),
        Ok((false, Some(c))) => panic!("Command failed with exit code {}", c),
        Ok((false, None)) => panic!("Command got killed"),
        Err(e) => panic!("Failed to run command: {}", e),
    }
    
}

fn main() {
    if !Path::new("lrc-erasure-code/README.md").exists() {
        update_submodules();
    }
    bindgen_lrc();
    if !try_to_find_and_link_lib("LRC") {
        println!("cargo:rerun-if-changed=lrc-erasure-code");
        fail_on_empty_directory("lrc-erasure-code");
        build_lrc();
    }
    println!(
        "cargo:cargo_manifest_dir={}",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );
    println!("cargo:out_dir={}", env::var("OUT_DIR").unwrap());

}

fn build_lrc(){
    let target = env::var("TARGET").unwrap();
    let mut config = cc::Build::new();
    config.include("lrc-erasure-code/include");
    config.include("lrc-erasure-code/src");
    config.warnings(false);
    let mut lib_sources = include_str!("lrc_lib_sources.txt")
        .trim()
        .split("\n")
        .map(str::trim)
        .collect::<Vec<_>>();
    if let(true, Ok(target_feature_value)) = (target.contains("x86_64"), env::var("CARGO_CFG_TARGET_FEATURE")) {
        println!("target_feature_value: {:?}", target_feature_value);
        let target_feature = target_feature_value.split(",").collect::<Vec<_>>();
        println!("target_feature: {:?}", target_feature);
        if target_feature.contains(&"mmx"){
            config.flag_if_supported("-mmmx");
        }
        if target_feature.contains(&"sse"){
            config.flag_if_supported("-msse");
            config.define("INTEL_SSE", "1");
        }
        if target_feature.contains(&"sse2"){
            config.flag_if_supported("-msse2");
            config.define("INTEL_SSE2", "1");
        }
        if target_feature.contains(&"sse3"){
            config.flag_if_supported("-msse3");
            config.define("INTEL_SSE3", "1");
        }
        if target_feature.contains(&"pclmulqdq"){
            config.flag_if_supported("-mpclmul");
            config.define("INTEL_SSE4_PCLMUL", "1");
        }
        if target_feature.contains(&"ssse3"){
            config.flag_if_supported("-mssse3");
            config.define("INTEL_SSSE3", "1");
        }
        if target_feature.contains(&"sse4.1"){
            config.flag_if_supported("-msse4.1");
            config.define("INTEL_SSE4", "1");
        }
        if target_feature.contains(&"sse4.2"){
            config.flag_if_supported("-msse4.2");
            config.define("INTEL_SSE4", "1");
        }
        if target_feature.contains(&"avx"){
            config.flag_if_supported("-mavx");
        }

        // exclude neon
        lib_sources.retain(|&x| !x.contains("neon"));
    }
    
    println!("lib_sources: {:?}", lib_sources);
    for file in lib_sources {
        config.file(format!("lrc-erasure-code/src/{file}"));
    }
    config.compile("liblrc.a");
}