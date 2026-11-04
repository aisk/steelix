// Exposes PyO3's interpreter cfgs (`Py_GIL_DISABLED` and friends) to this
// crate, see the free-threading check in `lib.rs`.
fn main() {
    pyo3_build_config::use_pyo3_cfgs();
}
