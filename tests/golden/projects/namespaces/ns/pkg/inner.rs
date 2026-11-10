use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `ns/pkg/inner.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let helper = rt::import_names(m, "..helper", &["double"])?;
    m.set_global("double", rt::import_from(&helper, "double")?)?;
    m.set_global(
        "NAME",
        rt::fstring(
            m.py(),
            (
                "inner ",
                rt::format(&m.global("double")?.call1((21,))?, "")?,
            ),
        )?,
    )?;
    Ok(())
}
