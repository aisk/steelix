use pyo3::exceptions::PyBrokenPipeError;
use pyo3::exceptions::PySystemExit;
use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `json/__main__.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Command-line tool to validate and pretty-print JSON\n\nUsage::\n\n    $ echo '{\"json\":\"obj\"}' | python -m json\n    {\n        \"json\": \"obj\"\n    }\n    $ echo '{ 1.2:3.4}' | python -m json\n    Expecting property name enclosed in double quotes: line 1 column 3 (char 2)\n\n")?;
        // Copied unmodified (below this header) from Lib/json/__main__.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        frame.line(16);
        m.set_global("json", rt::import(m, "json.tool")?)?;
        frame.line(19);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(21);
            match (|| m.global("json")?.getattr("tool")?.call_method0("main"))() {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyBrokenPipeError>(m.py()) => {
                    let exception = frame.catch(err);
                    m.set_global("exc", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(23);
                    return Err(frame.raise(PySystemExit::new_err((m
                        .global("exc")?
                        .getattr("errno")?
                        .unbind(),))));
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    })
}
