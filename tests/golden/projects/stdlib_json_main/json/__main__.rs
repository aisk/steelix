use pyo3::exceptions::PyBrokenPipeError;
use pyo3::exceptions::PySystemExit;
use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

/// Executes the body of `json/__main__.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    m.set_global(
        "__doc__",
        "Command-line tool to validate and pretty-print JSON\n\
            \n\
            Usage::\n\
            \n\
            \x20   $ echo '{\"json\":\"obj\"}' | python -m json\n\
            \x20   {\n\
            \x20       \"json\": \"obj\"\n\
            \x20   }\n\
            \x20   $ echo '{ 1.2:3.4}' | python -m json\n\
            \x20   Expecting property name enclosed in double quotes: line 1 column 3 (char 2)\n\
            \n",
    )?;
    // Copied unmodified (below this header) from Lib/json/__main__.py of CPython 3.14.9,
    // licensed under the PSF License Agreement. A steelix test project:
    // tests/test_projects.py runs it transpiled and with CPython and compares.
    m.set_global("json", rt::import(m, "json.tool")?)?;
    if m.global("__name__")?.eq("__main__")? {
        match (|| m.global("json")?.attr("tool")?.method0("main"))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyBrokenPipeError>(m.py()) => {
                let exception = frame.catch(err);
                m.set_global("exc", &exception)?;
                return Err(frame.raise(PySystemExit::new_err((m
                    .global("exc")?
                    .attr("errno")?
                    .unbind(),))));
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}
