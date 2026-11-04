use pyo3::exceptions::PyImportError;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyStopIteration;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// json/scanner.py:18
pub fn py_make_scanner<'py>(
    m: &rt::Module<'py>,
    context: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "py_make_scanner", 18, |_| {
        let _scan_once = rt::Cell::empty(m.py(), "_scan_once")?;
        let parse_object = context.attr("parse_object")?;
        let parse_array = context.attr("parse_array")?;
        let parse_string = context.attr("parse_string")?;
        let match_number = m.global("NUMBER_RE")?.attr("match")?;
        let strict = context.attr("strict")?;
        let parse_float = context.attr("parse_float")?;
        let parse_int = context.attr("parse_int")?;
        let parse_constant = context.attr("parse_constant")?;
        let object_hook = context.attr("object_hook")?;
        let object_pairs_hook = context.attr("object_pairs_hook")?;
        let memo = context.attr("memo")?;
        // json/scanner.py:31
        fn _scan_once_<'py>(
            module: &rt::Module<'py>,
            parse_string: &Bound<'py, PyAny>,
            strict: &Bound<'py, PyAny>,
            parse_object: &Bound<'py, PyAny>,
            _scan_once: &rt::Cell<'py>,
            object_hook: &Bound<'py, PyAny>,
            object_pairs_hook: &Bound<'py, PyAny>,
            memo: &Bound<'py, PyAny>,
            parse_array: &Bound<'py, PyAny>,
            match_number: &Bound<'py, PyAny>,
            parse_float: &Bound<'py, PyAny>,
            parse_int: &Bound<'py, PyAny>,
            parse_constant: &Bound<'py, PyAny>,
            string: &Bound<'py, PyAny>,
            idx: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = module.py();
            rt::frame!(module, "py_make_scanner.<locals>._scan_once", 31, |frame| {
                let nextchar = match string.get_item(idx) {
                    Ok(value) => value,
                    Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                        let _caught = frame.catch(err);
                        return Err(frame.raise_from(
                            PyStopIteration::new_err((idx.clone().unbind(),)),
                            py.None(),
                        ));
                    }
                    Err(err) => return Err(err),
                };
                if nextchar.eq("\"")? {
                    return parse_string.call1((string, idx.add(1)?, strict));
                } else if nextchar.eq("{")? {
                    return parse_object.call1((
                        PyTuple::new(py, [string, &idx.add(1)?])?,
                        strict,
                        _scan_once.get()?,
                        object_hook,
                        object_pairs_hook,
                        memo,
                    ));
                } else if nextchar.eq("[")? {
                    return parse_array
                        .call1((PyTuple::new(py, [string, &idx.add(1)?])?, _scan_once.get()?));
                } else if nextchar.eq("n")?
                    && string
                        .get_item(rt::slice(py, idx, idx.add(4)?)?)?
                        .eq("null")?
                {
                    return Ok(PyTuple::new(py, [&rt::none(py), &idx.add(4)?])?.into_any());
                } else if nextchar.eq("t")?
                    && string
                        .get_item(rt::slice(py, idx, idx.add(4)?)?)?
                        .eq("true")?
                {
                    return Ok(PyTuple::new(py, [&rt::bool(py, true), &idx.add(4)?])?.into_any());
                } else if nextchar.eq("f")?
                    && string
                        .get_item(rt::slice(py, idx, idx.add(5)?)?)?
                        .eq("false")?
                {
                    return Ok(PyTuple::new(py, [&rt::bool(py, false), &idx.add(5)?])?.into_any());
                }
                let m = match_number.call1((string, idx))?;
                let integer: Bound<'py, PyAny>;
                let frac: Bound<'py, PyAny>;
                let exp: Bound<'py, PyAny>;
                let res: Bound<'py, PyAny>;
                if !m.is_none() {
                    [integer, frac, exp] = rt::unpack(&m.method0("groups")?)?;
                    if frac.is_truthy()? || exp.is_truthy()? {
                        res = parse_float.call1((integer
                            .add(if frac.is_truthy()? {
                                frac.clone()
                            } else {
                                rt::str(py, "")
                            })?
                            .add(if exp.is_truthy()? {
                                exp.clone()
                            } else {
                                rt::str(py, "")
                            })?,))?;
                    } else {
                        res = parse_int.call1((&integer,))?;
                    }
                    Ok(PyTuple::new(py, [&res, &m.method0("end")?])?.into_any())
                } else if nextchar.eq("N")?
                    && string
                        .get_item(rt::slice(py, idx, idx.add(3)?)?)?
                        .eq("NaN")?
                {
                    Ok(
                        PyTuple::new(py, [&parse_constant.call1(("NaN",))?, &idx.add(3)?])?
                            .into_any(),
                    )
                } else if nextchar.eq("I")?
                    && string
                        .get_item(rt::slice(py, idx, idx.add(8)?)?)?
                        .eq("Infinity")?
                {
                    Ok(
                        PyTuple::new(py, [&parse_constant.call1(("Infinity",))?, &idx.add(8)?])?
                            .into_any(),
                    )
                } else if nextchar.eq("-")?
                    && string
                        .get_item(rt::slice(py, idx, idx.add(9)?)?)?
                        .eq("-Infinity")?
                {
                    Ok(
                        PyTuple::new(py, [&parse_constant.call1(("-Infinity",))?, &idx.add(9)?])?
                            .into_any(),
                    )
                } else {
                    Err(frame.raise(PyStopIteration::new_err((idx.clone().unbind(),))))
                }
            })
        }
        _scan_once.set(rt::function!(
            m,
            _scan_once_[parse_string, strict, parse_object, &_scan_once, object_hook, object_pairs_hook, memo, parse_array, match_number, parse_float, parse_int, parse_constant](string, idx),
            qualname = "py_make_scanner.<locals>._scan_once"
        )?)?;
        // json/scanner.py:68
        fn scan_once<'py>(
            m: &rt::Module<'py>,
            _scan_once: &rt::Cell<'py>,
            memo: &Bound<'py, PyAny>,
            string: &Bound<'py, PyAny>,
            idx: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "py_make_scanner.<locals>.scan_once", 68, |frame| {
                let result =
                    (|| -> PyResult<Bound<'py, PyAny>> { _scan_once.get()?.call1((string, idx)) })(
                    );
                let finally = frame.finally(result);
                memo.method0("clear")?;
                finally.resume()
            })
        }
        let scan_once = rt::function!(
            m,
            scan_once[&_scan_once, memo](string, idx),
            qualname = "py_make_scanner.<locals>.scan_once"
        )?;
        Ok(scan_once)
    })
}

/// Executes the body of `json/scanner.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "JSON token scanner\n")?;
        // Copied unmodified (below this header) from Lib/json/scanner.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        m.set_global("re", rt::import(m, "re")?)?;
        match (|| -> PyResult<()> {
            let _json = rt::import_names(m, "_json", &["make_scanner"])?;
            m.set_global("c_make_scanner", rt::import_from(&_json, "make_scanner")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _caught = frame.catch(err);
                m.set_global("c_make_scanner", py.None())?;
            }
            Err(err) => return Err(err),
        }
        m.set_global("__all__", PyList::new(py, ["make_scanner"])?)?;
        m.set_global(
            "NUMBER_RE",
            m.global("re")?.attr("compile")?.call1((
                "(-?(?:0|[1-9][0-9]*))(\\.[0-9]+)?([eE][-+]?[0-9]+)?",
                m.global("re")?
                    .attr("VERBOSE")?
                    .bitor(m.global("re")?.attr("MULTILINE")?)?
                    .bitor(m.global("re")?.attr("DOTALL")?)?,
            ))?,
        )?;
        rt::def!(m, py_make_scanner(context))?;
        {
            let c_make_scanner = m.global("c_make_scanner")?;
            m.set_global(
                "make_scanner",
                if c_make_scanner.is_truthy()? {
                    c_make_scanner
                } else {
                    m.global("py_make_scanner")?
                },
            )?;
        }
        Ok(())
    })
}
