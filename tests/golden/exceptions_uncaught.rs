use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyLookupError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// exceptions_uncaught.py:6
#[rt::frame(m, "resource", 6)]
pub async fn resource<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("open", name))?;
    rt::yield_(name).await?;
    rt::builtins::print(py, ("close", name))?;
    Ok(rt::none(py))
}

// exceptions_uncaught.py:13
#[rt::frame(m, "load", 13)]
pub fn load<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let table = [("a", 1)].into_py_dict(m.py())?.into_any();
    table.get_item(key)
}

// exceptions_uncaught.py:18
#[rt::frame(m, "lookup", 18, frame)]
pub fn lookup<'py>(m: &rt::Module<'py>, key: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match load(m, key) {
        Ok(value) => Ok(value),
        Err(err) if err.is_instance_of::<PyKeyError>(py) => {
            let e = frame.catch(err);
            Err(frame.raise_from(
                PyLookupError::new_err((rt::str(py, "no such key: ").add(key)?.unbind(),)),
                &e,
            ))
        }
        Err(err) => Err(err),
    }
}

// exceptions_uncaught.py:25
#[rt::frame(m, "process", 25, frame)]
pub fn process<'py>(m: &rt::Module<'py>, keys: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut total = rt::int(py, 0);
    frame.with(&m.global("resource")?.call1(("db",))?, |_| {
        for key in rt::iter(keys)? {
            let key = key?;
            let result = (|| -> PyResult<()> {
                total = rt::iadd(&total, lookup(m, &key)?)?;
                Ok(())
            })();
            let finally = frame.finally(result);
            rt::builtins::print(py, ("visited", &key))?;
            finally.resume()?;
        }
        Ok(())
    })?;
    Ok(total)
}

// exceptions_uncaught.py:36
#[rt::frame(m, "main", 36, frame)]
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let cleanup: Bound<'py, PyAny>;
    match (|| process(m, PyList::new(py, ["a", "b"])?.as_any()))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyLookupError>(py) => {
            let _caught = frame.catch(err);
            cleanup = rt::none(py);
            cleanup.method0("close")?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

/// Executes the body of `exceptions_uncaught.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    // An uncaught exception with a cause and a context, raised through native
    // handlers, `finally` and `with`: CPython prints all three tracebacks.
    m.set_global("contextlib", rt::import(m, "contextlib")?)?;
    rt::def!(m, @contextlib.contextmanager gen resource(name))?;
    rt::def!(m, load(key))?;
    rt::def!(m, lookup(key))?;
    rt::def!(m, process(keys))?;
    rt::def!(m, main())?;
    main(m)?;
    Ok(())
}
