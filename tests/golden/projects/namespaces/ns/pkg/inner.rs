use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `ns/pkg/inner.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
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
    })
}
