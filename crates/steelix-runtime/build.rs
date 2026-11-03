use pyo3_build_config::{PythonAbiKind, PythonImplementation};

// Exposes PyO3's interpreter cfgs (`Py_GIL_DISABLED` and friends) to this
// crate, see the free-threading check in `lib.rs`.
fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    // `python_recursion`: the layout of CPython 3.14's thread state is
    // known, so native coroutines can count Python recursion like CPython
    // (`Depth` in `coroutine.rs`). Elsewhere only the C stack is checked.
    println!("cargo::rustc-check-cfg=cfg(python_recursion)");
    let abi = pyo3_build_config::get().target_abi();
    let version = abi.version();
    if abi.implementation() == PythonImplementation::CPython
        && matches!(abi.kind(), PythonAbiKind::VersionSpecific(_))
        && (version.major, version.minor) == (3, 14)
    {
        println!("cargo::rustc-cfg=python_recursion");
    }
}
