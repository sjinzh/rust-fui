use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
    let target = env::var("TARGET").unwrap();
    let current_dir = env::current_dir().unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap();

    println!("target: {}", target);
    println!("current_dir: {}", current_dir.to_string_lossy());
    println!("out_dir: {}", out_dir);
    println!("env: {}", out_dir);

    run_cbindgen();

    run_qmake(
        &current_dir.join("src/platform/qt/qt_wrapper/cpp"),
        &out_dir,
    );
    run_make(&env, &out_dir);

    cargo_link_static(&out_dir, "qt_wrapper");
    cargo_link_qt(&env);

    generate_bindings("src/platform/qt/qt_wrapper/cpp/qt_wrapper.h", &out_dir);
}

fn run_cbindgen() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&Path::new(&out_dir).join("rust_ffi.h"));
}

fn run_qmake(src_dir: &Path, out_dir: &str) {
    let output = Command::new("qmake")
        .args(&[src_dir])
        .current_dir(&Path::new(&out_dir))
        .output()
        .expect("failed to execute 'qmake' process");

    println!("qmake.status: {}", output.status);
    println!("qmake.stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("qmake.stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "failed to execute qmake process");
}

fn run_make(env: &str, dir: &str) {
    let output = if env == "msvc" {
        Command::new("nmake")
            .current_dir(&Path::new(&dir))
            .output()
            .expect("failed to execute 'make' process")
    } else {
        Command::new("make")
            .args(&["-j16"])
            .current_dir(&Path::new(&dir))
            .output()
            .expect("failed to execute 'make' process")
    };

    println!("make.status: {}", output.status);
    println!("make.stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("make.stderr: {}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "failed to execute make process");
}

fn cargo_link_static(dir: &str, lib: &str) {
    println!("cargo:rustc-link-search={}", dir);
    println!("cargo:rustc-link-lib={}", lib);
}

fn qmake_query(var: &str) -> String {
    String::from_utf8(
        Command::new("qmake")
            .args(&["-query", var])
            .output()
            .expect("Failed to execute qmake. Make sure 'qmake' is in your path")
            .stdout,
    )
    .expect("UTF-8 conversion failed")
}

fn cargo_link_qt(env: &str) {
    let qt_library_path = qmake_query("QT_INSTALL_LIBS");

    println!("cargo:rustc-link-search={}", qt_library_path);
    println!("cargo:rustc-link-lib={}", "Qt5Widgets");
    println!("cargo:rustc-link-lib={}", "Qt5Gui");
    println!("cargo:rustc-link-lib={}", "Qt5Core");

    if env != "msvc" {
        println!("cargo:rustc-link-lib=stdc++");
    }
}

fn generate_bindings(src: &str, out_dir: &str) {
    let bindings = bindgen::Builder::default()
        .header(src)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(&Path::new(out_dir).join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
