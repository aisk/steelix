//! Parts of `exceptions_uncaught.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 6, `def resource`: yield

use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyLookupError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use steelix_runtime as rt;

// exceptions_uncaught.py:13
pub fn load<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "load", 13, |frame| {
        frame.line(14);
        let table = [("a", 1)].into_py_dict(m.py())?.into_any();
        frame.line(15);
        table.get_item(key)
    })
}

// exceptions_uncaught.py:18
pub fn lookup<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "lookup", 18, |frame| {
        frame.line(20);
        match load(m, key) {
            Ok(value) => Ok(value),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(22);
                Err(frame.raise_from(
                    PyLookupError::new_err((rt::str(py, "no such key: ").add(key)?.unbind(),)),
                    &e,
                ))
            }
            Err(err) => Err(err),
        }
    })
}

// exceptions_uncaught.py:25
pub fn process<'py>(m: &rt::Module<'py>, keys: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "process", 25, |frame| {
        let mut total = rt::int(py, 0);
        frame.line(27);
        frame.with(&m.global("resource")?.call1(("db",))?, |_| {
            frame.line(28);
            for key in keys.try_iter()? {
                let key = key?;
                let result = (|| -> PyResult<()> {
                    frame.line(30);
                    total = rt::iadd(&total, lookup(m, &key)?)?;
                    Ok(())
                })();
                let finally = frame.finally(result);
                frame.line(32);
                rt::builtins::print(py, ("visited", &key))?;
                finally.resume()?;
                py.check_signals()?;
                frame.line(28);
            }
            Ok(())
        })?;
        Ok(total)
    })
}

// exceptions_uncaught.py:36
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 36, |frame| {
        let cleanup: Bound<'py, PyAny>;
        frame.line(38);
        match (|| process(m, PyList::new(py, ["a", "b"])?.as_any()))() {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyLookupError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                cleanup = rt::none(py);
                frame.line(41);
                cleanup.call_method0("close")?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

/// Executes the body of `exceptions_uncaught.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |frame| {
        // An uncaught exception with a cause and a context, raised through native
        // handlers, `finally` and `with`: CPython prints all three tracebacks.
        frame.line(3);
        m.set_global("contextlib", rt::import(m, "contextlib")?)?;
        // exceptions_uncaught.py:6, kept as Python: yield at 9:5
        rt::exec_python!(
            m,
            line = 6,
            r#"
@contextlib.contextmanager
def resource(name):
    print("open", name)
    yield name
    print("close", name)
"#
        )?;
        rt::def!(m, load(key))?;
        rt::def!(m, lookup(key))?;
        rt::def!(m, process(keys))?;
        rt::def!(m, main())?;
        frame.line(44);
        main(m)?;
        Ok(())
    })
}
