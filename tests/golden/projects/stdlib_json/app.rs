use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// app.py:23
/// Which functions came from `_json`.
pub fn accelerated<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "accelerated", 23, |_| {
        Ok([
            (
                "scanner",
                m.global("scanner")?
                    .attr("make_scanner")?
                    .is(m.global("scanner")?.attr("c_make_scanner")?),
            ),
            (
                "scanstring",
                m.global("decoder")?
                    .attr("scanstring")?
                    .is(m.global("decoder")?.attr("c_scanstring")?),
            ),
            (
                "encode_basestring_ascii",
                m.global("encoder")?
                    .attr("encode_basestring_ascii")?
                    .is(m.global("encoder")?.attr("c_encode_basestring_ascii")?),
            ),
            (
                "make_encoder",
                !m.global("encoder")?.attr("c_make_encoder")?.is_none(),
            ),
        ]
        .into_py_dict(m.py())?
        .into_any())
    })
}

// app.py:35
pub fn roundtrips<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "roundtrips", 35, |_| {
        let value = m
            .global("json")?
            .attr("loads")?
            .call1((m.global("DOC")?,))?;
        m.global("print")?.call1((&value,))?;
        m.global("print")?
            .call1((m.global("json")?.method1("dumps", (&value,))?,))?;
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("dumps")?,
            &value,
            sort_keys = true,
            ensure_ascii = false
        )?,))?;
        // `indent` uses the pure-Python encoder `_make_iterencode`.
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("dumps")?,
            &value,
            indent = 2,
            sort_keys = true,
            separators = PyTuple::new(py, [",", ": "])?
        )?,))?;
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("dumps")?,
            PyList::new(
                py,
                [
                    &rt::int(py, 1),
                    [("x", PyTuple::new(py, [2, 3])?.as_any())]
                        .into_py_dict(py)?
                        .as_any()
                ]
            )?,
            indent = "\t"
        )?,))?;
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("loads")?,
            m.global("DOC")?,
            object_pairs_hook = m.global("OrderedDict")?
        )?,))?;
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("loads")?,
            "[1.10, 2]",
            parse_float = m.global("Decimal")?,
            parse_int = m.global("str")?
        )?,))?;
        m.global("print")?.call1((m
            .global("json")?
            .method1("loads", ("{\"k\": NaN, \"j\": -Infinity}",))?,))?;
        m.global("print")?.call1((
            m.global("json")?
                .attr("dumps")?
                .call1((m.global("float")?.call1(("inf",))?,))?,
            m.global("json")?.attr("dumps")?.call1(([
                (&rt::int(py, 1), 2),
                (&rt::none(py), 3),
                (&rt::float(py, 2.5), 4),
                (&rt::bool(py, true), 5),
            ]
            .into_py_dict(py)?,))?,
        ))?;
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("dumps")?,
            [
                ("z", &rt::int(py, 1)),
                ("y", PyList::new(py, [1, 2])?.as_any())
            ]
            .into_py_dict(py)?,
            sort_keys = true,
            indent = 1
        )?,))?;
        Ok(rt::none(py))
    })
}

// app.py:50
/// The fallbacks the package uses without `_json`.
pub fn pure_python<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pure_python", 50, |frame| {
        let dec = m.global("json")?.method0("JSONDecoder")?;
        dec.set_attr("parse_string", m.global("decoder")?.attr("py_scanstring")?)?;
        dec.set_attr(
            "scan_once",
            m.global("scanner")?.method1("py_make_scanner", (&dec,))?,
        )?;
        m.global("print")?
            .call1((dec.attr("decode")?.call1((m.global("DOC")?,))?,))?;
        m.global("print")?.call1((m
            .global("decoder")?
            .method1("py_scanstring", ("\"abc\\n\\u00e9\" tail", 1))?,))?;
        m.global("print")?.call1((m
            .global("encoder")?
            .method1("py_encode_basestring_ascii", ("café 😀 \n",))?,))?;
        m.global("print")?.call1((m
            .global("encoder")?
            .method1("py_encode_basestring", ("café \n",))?,))?;
        let enc = rt::call!(m.global("json")?.attr("JSONEncoder")?, sort_keys = true)?;
        m.global("print")?
            .call1((rt::str(py, "").attr("join")?.call1((rt::call!(
                enc.attr("iterencode")?,
                [
                    ("b", PyList::new(py, [1, 2])?.as_any()),
                    ("a", &rt::none(py))
                ]
                .into_py_dict(py)?,
                _one_shot = false
            )?,))?,))?;
        match dec.method1("decode", ("{\"a\": [1, 2,]}",)) {
            Ok(_) => {}
            Err(err)
                if frame.matches(&err, 50, || m.global("json")?.attr("JSONDecodeError"))? =>
            {
                let err = frame.catch(err);
                m.global("print")?.call1((
                    "pure:",
                    &err,
                    err.attr("pos")?,
                    err.attr("lineno")?,
                    err.attr("colno")?,
                ))?;
            }
            Err(err2) => return Err(err2),
        }
        Ok(rt::none(py))
    })
}

// app.py:80
pub fn errors<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "errors", 80, |frame| {
        for text in rt::iter(
            PyList::new(
                py,
                ["{\"a\": }", "[1, 2", "{\"a\" 1}", "\"\\x\"", "", "[1] x"],
            )?
            .as_any(),
        )? {
            let text = text?;
            match (|| m.global("json")?.method1("loads", (&text,)))() {
                Ok(_) => {}
                Err(err)
                    if frame.matches(&err, 80, || m.global("json")?.attr("JSONDecodeError"))? =>
                {
                    let err = frame.catch(err);
                    m.global("print")?.call1((
                        m.global("repr")?.call1((&text,))?,
                        "->",
                        err.attr("msg")?,
                        err.attr("pos")?,
                        err.attr("lineno")?,
                        err.attr("colno")?,
                    ))?;
                }
                Err(err2) => return Err(err2),
            }
        }
        match (|| {
            m.global("json")?
                .attr("dumps")?
                .call1(([("p", &m.global("object")?.call0()?)].into_py_dict(py)?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 80, || m.global("TypeError"))? => {
                let err = frame.catch(err);
                m.global("print")?
                    .call1(("TypeError:", &err, err.attr("__notes__")?))?;
            }
            Err(err2) => return Err(err2),
        }
        match (|| {
            rt::call!(
                m.global("json")?.attr("dumps")?,
                PyList::new(py, [&m.global("float")?.call1(("nan",))?])?,
                allow_nan = false
            )
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 80, || m.global("ValueError"))? => {
                let err = frame.catch(err);
                m.global("print")?.call1(("ValueError:", &err))?;
            }
            Err(err2) => return Err(err2),
        }
        let circular = PyList::empty(py).into_any();
        circular.method1("append", (&circular,))?;
        match (|| m.global("json")?.method1("dumps", (&circular,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 80, || m.global("ValueError"))? => {
                let err = frame.catch(err);
                m.global("print")?.call1(("ValueError:", &err))?;
            }
            Err(err2) => return Err(err2),
        }
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("dumps")?,
            PyList::new(py, [&m.global("Point")?.call1((1, 2))?])?,
            cls = m.global("PointEncoder")?
        )?,))?;
        m.global("print")?.call1((rt::call!(
            m.global("json")?.attr("dumps")?,
            m.global("Point")?.call1((3, 4))?,
            default = m.global("vars")?,
            indent = 0
        )?,))?;
        Ok(rt::none(py))
    })
}

// app.py:104
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "main", 104, |_| {
        m.global("print")?
            .call1((m.global("accelerated")?.call0()?,))?;
        m.global("roundtrips")?.call0()?;
        m.global("pure_python")?.call0()?;
        m.global("errors")?.call0()?;
        m.global("print")?.call1((
            m.global("json")?.attr("decoder")?.attr("__name__")?,
            m.global("json")?.attr("decoder")?.attr("__package__")?,
            m.global("json")?
                .attr("scanner")?
                .attr("__spec__")?
                .attr("name")?,
        ))?;
        Ok(rt::none(m.py()))
    })
}

pub mod point {
    use super::*;

    // app.py:67
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 67, |_| {
            rt::def!(ns, __init__(slf as "self", x, y))?;
            Ok(())
        })
    }

    // app.py:68
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.__init__", 68, |_| {
            slf.set_attr("x", x)?;
            slf.set_attr("y", y)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod point_encoder {
    use super::*;

    // app.py:73
    /// The body of `class PointEncoder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "PointEncoder", 73, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, default[&__class__](slf as "self", o))?;
            Ok(())
        })
    }

    // app.py:74
    pub fn default<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "PointEncoder.default", 74, |_| {
            if m.global("isinstance")?
                .call1((o, m.global("Point")?))?
                .is_truthy()?
            {
                return Ok([("x", &o.attr("x")?), ("y", &o.attr("y")?)]
                    .into_py_dict(m.py())?
                    .into_any());
            }
            rt::super_(__class__, slf)?.method1("default", (o,))
        })
    }
}

/// Executes the body of `app.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Drives the `json` package next to it, a copy of the standard library's.\n\
                \n\
                The copy shadows the standard library's `json` (it is in `sys.path[0]`), so\n\
                the transpiled program runs the package's transpiled modules. The package\n\
                picks the C accelerator `_json` at import time where it can; the pure-Python\n\
                scanner, string functions and encoder are exercised explicitly as well.\n\
                \n\
                \x20   python app.py            exercises the package\n\
                \x20   python app.py tool ARGS  runs `json.tool.main()` with ARGS\n\
                \x20   python app.py crash      ends with an uncaught `JSONDecodeError`\n",
        )?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        let collections = rt::import_names(m, "collections", &["OrderedDict"])?;
        m.set_global("OrderedDict", rt::import_from(&collections, "OrderedDict")?)?;
        let decimal = rt::import_names(m, "decimal", &["Decimal"])?;
        m.set_global("Decimal", rt::import_from(&decimal, "Decimal")?)?;
        m.set_global("json", rt::import(m, "json")?)?;
        let json = rt::import_names(m, "json", &["decoder", "encoder", "scanner"])?;
        m.set_global("decoder", rt::import_from(&json, "decoder")?)?;
        m.set_global("encoder", rt::import_from(&json, "encoder")?)?;
        m.set_global("scanner", rt::import_from(&json, "scanner")?)?;
        m.set_global("DOC", "{\"b\": [1, 2.5, \"caf\\u00e9\", null, true], \"a\": {\"nested\": \"\\ud83d\\ude00\"}, \"n\": -0.0}")?;
        rt::def!(m, accelerated(), doc = "Which functions came from `_json`.")?;
        rt::def!(m, roundtrips())?;
        rt::def!(
            m,
            pure_python(),
            doc = "The fallbacks the package uses without `_json`."
        )?;
        m.set_global(
            "Point",
            rt::class!(
                m,
                "Point"(),
                point::body,
                line = 67,
                attributes = ["x", "y"]
            )?,
        )?;
        m.set_global(
            "PointEncoder",
            rt::class!(
                m,
                "PointEncoder"(m.global("json")?.attr("JSONEncoder")?),
                point_encoder::body,
                line = 73
            )?,
        )?;
        rt::def!(m, errors())?;
        rt::def!(m, main())?;
        if m.global("__name__")?.eq("__main__")? {
            if m.global("sys")?
                .attr("argv")?
                .get_item(rt::slice(py, 1, 2)?)?
                .eq(PyList::new(py, ["tool"])?)?
            {
                m.set_global("json", rt::import(m, "json.tool")?)?;
                let value = PyList::empty(py);
                m.global("sys")?
                    .attr("argv")?
                    .set_item(rt::slice(py, 1, 2)?, value)?;
                m.global("json")?.attr("tool")?.method0("main")?;
            } else if m
                .global("sys")?
                .attr("argv")?
                .get_item(rt::slice(py, 1, 2)?)?
                .eq(PyList::new(py, ["crash"])?)?
            {
                m.global("json")?
                    .method1("loads", ("{\"ok\": [1, 2, ]}",))?;
            } else {
                m.global("main")?.call0()?;
            }
        }
        Ok(())
    })
}
