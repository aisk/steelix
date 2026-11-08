// Exposes PyO3's interpreter cfgs (`Py_GIL_DISABLED` and friends) to this
// crate, see the free-threading check in `lib.rs`.
fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    link_python_on_windows();
}

/// The runtime declares a few CPython functions that `pyo3-ffi` does not
/// have (`function.rs`, `coroutine/finalize.rs`). On Windows they have to
/// come from the interpreter's import library (`libs/python314.lib`); PyO3
/// links its own declarations with `raw-dylib` instead, which needs the DLL
/// name as a literal. Elsewhere the symbols resolve at load time.
fn link_python_on_windows() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let config = pyo3_build_config::get();
    if let Some(lib_dir) = config.lib_dir() {
        println!("cargo:rustc-link-search=native={lib_dir}");
    }
    if let Some(lib_name) = config.lib_name() {
        println!("cargo:rustc-link-lib={lib_name}");
    }
}
