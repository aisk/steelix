use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// app.py:23
/// Which functions came from `_json`.
pub fn accelerated<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "accelerated", 23, |frame| {
        frame.line(25);
        Ok([
            (
                "scanner",
                &rt::bool(
                    py,
                    m.global("scanner")?
                        .getattr("make_scanner")?
                        .is(m.global("scanner")?.getattr("c_make_scanner")?),
                ),
            ),
            (
                "scanstring",
                &rt::bool(
                    py,
                    m.global("decoder")?
                        .getattr("scanstring")?
                        .is(m.global("decoder")?.getattr("c_scanstring")?),
                ),
            ),
            (
                "encode_basestring_ascii",
                &rt::bool(
                    py,
                    m.global("encoder")?
                        .getattr("encode_basestring_ascii")?
                        .is(m.global("encoder")?.getattr("c_encode_basestring_ascii")?),
                ),
            ),
            (
                "make_encoder",
                &rt::bool(
                    py,
                    !m.global("encoder")?.getattr("c_make_encoder")?.is_none(),
                ),
            ),
        ]
        .into_py_dict(py)?
        .into_any())
    })
}

// app.py:35
pub fn roundtrips<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "roundtrips", 35, |frame| {
        frame.line(36);
        let value = m
            .global("json")?
            .getattr("loads")?
            .call1((m.global("DOC")?,))?;
        frame.line(37);
        m.global("print")?.call1((&value,))?;
        frame.line(38);
        m.global("print")?
            .call1((m.global("json")?.call_method1("dumps", (&value,))?,))?;
        frame.line(39);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("dumps")?,
            &value,
            sort_keys = true,
            ensure_ascii = false
        )?,))?;
        // `indent` uses the pure-Python encoder `_make_iterencode`.
        frame.line(41);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("dumps")?,
            &value,
            indent = 2,
            sort_keys = true,
            separators = PyTuple::new(py, [",", ": "])?
        )?,))?;
        frame.line(42);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("dumps")?,
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
        frame.line(43);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("loads")?,
            m.global("DOC")?,
            object_pairs_hook = m.global("OrderedDict")?
        )?,))?;
        frame.line(44);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("loads")?,
            "[1.10, 2]",
            parse_float = m.global("Decimal")?,
            parse_int = m.global("str")?
        )?,))?;
        frame.line(45);
        m.global("print")?.call1((m
            .global("json")?
            .call_method1("loads", ("{\"k\": NaN, \"j\": -Infinity}",))?,))?;
        frame.line(46);
        m.global("print")?.call1((
            m.global("json")?
                .getattr("dumps")?
                .call1((m.global("float")?.call1(("inf",))?,))?,
            m.global("json")?.getattr("dumps")?.call1(([
                (&rt::int(py, 1), 2),
                (&rt::none(py), 3),
                (&rt::float(py, 2.5), 4),
                (&rt::bool(py, true), 5),
            ]
            .into_py_dict(py)?,))?,
        ))?;
        frame.line(47);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("dumps")?,
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
        frame.line(52);
        let dec = m.global("json")?.call_method0("JSONDecoder")?;
        frame.line(53);
        dec.setattr(
            "parse_string",
            m.global("decoder")?.getattr("py_scanstring")?,
        )?;
        frame.line(54);
        dec.setattr(
            "scan_once",
            m.global("scanner")?
                .call_method1("py_make_scanner", (&dec,))?,
        )?;
        frame.line(55);
        m.global("print")?
            .call1((dec.getattr("decode")?.call1((m.global("DOC")?,))?,))?;
        frame.line(56);
        m.global("print")?.call1((m
            .global("decoder")?
            .call_method1("py_scanstring", ("\"abc\\n\\u00e9\" tail", 1))?,))?;
        frame.line(57);
        m.global("print")?.call1((m
            .global("encoder")?
            .call_method1("py_encode_basestring_ascii", ("café 😀 \n",))?,))?;
        frame.line(58);
        m.global("print")?.call1((m
            .global("encoder")?
            .call_method1("py_encode_basestring", ("café \n",))?,))?;
        frame.line(59);
        let enc = rt::call!(m.global("json")?.getattr("JSONEncoder")?, sort_keys = true)?;
        frame.line(60);
        m.global("print")?
            .call1((rt::str(py, "").getattr("join")?.call1((rt::call!(
                enc.getattr("iterencode")?,
                [
                    ("b", PyList::new(py, [1, 2])?.as_any()),
                    ("a", &rt::none(py))
                ]
                .into_py_dict(py)?,
                _one_shot = false
            )?,))?,))?;
        frame.line(62);
        match dec.call_method1("decode", ("{\"a\": [1, 2,]}",)) {
            Ok(_) => {}
            Err(err)
                if frame.matches(&err, 63, || m.global("json")?.getattr("JSONDecodeError"))? =>
            {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(64);
                m.global("print")?.call1((
                    "pure:",
                    &err,
                    err.getattr("pos")?,
                    err.getattr("lineno")?,
                    err.getattr("colno")?,
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
        frame.line(81);
        for text in PyList::new(
            py,
            ["{\"a\": }", "[1, 2", "{\"a\" 1}", "\"\\x\"", "", "[1] x"],
        )?
        .try_iter()?
        {
            let text = text?;
            frame.line(83);
            match (|| m.global("json")?.call_method1("loads", (&text,)))() {
                Ok(_) => {}
                Err(err)
                    if frame
                        .matches(&err, 84, || m.global("json")?.getattr("JSONDecodeError"))? =>
                {
                    let err = frame.catch(err);
                    let _handling = rt::Handling::new(&err);
                    frame.line(85);
                    m.global("print")?.call1((
                        m.global("repr")?.call1((&text,))?,
                        "->",
                        err.getattr("msg")?,
                        err.getattr("pos")?,
                        err.getattr("lineno")?,
                        err.getattr("colno")?,
                    ))?;
                }
                Err(err2) => return Err(err2),
            }
            py.check_signals()?;
            frame.line(81);
        }
        frame.line(87);
        match (|| {
            m.global("json")?
                .getattr("dumps")?
                .call1(([("p", &m.global("object")?.call0()?)].into_py_dict(py)?,))
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 88, || m.global("TypeError"))? => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(89);
                m.global("print")?
                    .call1(("TypeError:", &err, err.getattr("__notes__")?))?;
            }
            Err(err2) => return Err(err2),
        }
        frame.line(91);
        match (|| {
            rt::call!(
                m.global("json")?.getattr("dumps")?,
                PyList::new(py, [&m.global("float")?.call1(("nan",))?])?,
                allow_nan = false
            )
        })() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 92, || m.global("ValueError"))? => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(93);
                m.global("print")?.call1(("ValueError:", &err))?;
            }
            Err(err2) => return Err(err2),
        }
        let circular = PyList::empty(py).into_any();
        frame.line(95);
        circular.call_method1("append", (&circular,))?;
        frame.line(97);
        match (|| m.global("json")?.call_method1("dumps", (&circular,)))() {
            Ok(_) => {}
            Err(err) if frame.matches(&err, 98, || m.global("ValueError"))? => {
                let err = frame.catch(err);
                let _handling = rt::Handling::new(&err);
                frame.line(99);
                m.global("print")?.call1(("ValueError:", &err))?;
            }
            Err(err2) => return Err(err2),
        }
        frame.line(100);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("dumps")?,
            PyList::new(py, [&m.global("Point")?.call1((1, 2))?])?,
            cls = m.global("PointEncoder")?
        )?,))?;
        frame.line(101);
        m.global("print")?.call1((rt::call!(
            m.global("json")?.getattr("dumps")?,
            m.global("Point")?.call1((3, 4))?,
            default = m.global("vars")?,
            indent = 0
        )?,))?;
        Ok(rt::none(py))
    })
}

// app.py:104
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "main", 104, |frame| {
        frame.line(105);
        m.global("print")?
            .call1((m.global("accelerated")?.call0()?,))?;
        frame.line(106);
        m.global("roundtrips")?.call0()?;
        frame.line(107);
        m.global("pure_python")?.call0()?;
        frame.line(108);
        m.global("errors")?.call0()?;
        frame.line(109);
        m.global("print")?.call1((
            m.global("json")?.getattr("decoder")?.getattr("__name__")?,
            m.global("json")?
                .getattr("decoder")?
                .getattr("__package__")?,
            m.global("json")?
                .getattr("scanner")?
                .getattr("__spec__")?
                .getattr("name")?,
        ))?;
        Ok(rt::none(m.py()))
    })
}

pub mod point {
    use super::*;

    // app.py:67
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 67, |frame| {
            frame.line(68);
            ns.set(
                "__init__",
                rt::function!(
                    ns,
                    __init__(slf as "self", x, y),
                    qualname = "Point.__init__"
                )?,
            )?;
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
        rt::frame!(m, "Point.__init__", 68, |frame| {
            frame.line(69);
            slf.setattr("x", x)?;
            frame.line(70);
            slf.setattr("y", y)?;
            Ok(rt::none(m.py()))
        })
    }
}

pub mod point_encoder {
    use super::*;

    // app.py:73
    /// The body of `class PointEncoder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "PointEncoder", 73, |frame| {
            let __class__ = ns.class_cell();
            frame.line(74);
            ns.set(
                "default",
                rt::function!(
                    ns,
                    default[&__class__](slf as "self", o),
                    qualname = "PointEncoder.default"
                )?,
            )?;
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
        rt::frame!(m, "PointEncoder.default", 74, |frame| {
            frame.line(75);
            if m.global("isinstance")?
                .call1((o, m.global("Point")?))?
                .is_truthy()?
            {
                frame.line(76);
                return Ok([("x", &o.getattr("x")?), ("y", &o.getattr("y")?)]
                    .into_py_dict(m.py())?
                    .into_any());
            }
            frame.line(77);
            rt::super_(__class__, slf)?.call_method1("default", (o,))
        })
    }
}

/// Executes the body of `app.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Drives the `json` package next to it, a copy of the standard library's.\n\nThe copy shadows the standard library's `json` (it is in `sys.path[0]`), so\nthe transpiled program runs the package's transpiled modules. The package\npicks the C accelerator `_json` at import time where it can; the pure-Python\nscanner, string functions and encoder are exercised explicitly as well.\n\n    python app.py            exercises the package\n    python app.py tool ARGS  runs `json.tool.main()` with ARGS\n    python app.py crash      ends with an uncaught `JSONDecodeError`\n")?;
        frame.line(13);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(14);
        let collections = rt::import_names(m, "collections", &["OrderedDict"])?;
        m.set_global("OrderedDict", rt::import_from(&collections, "OrderedDict")?)?;
        frame.line(15);
        let decimal = rt::import_names(m, "decimal", &["Decimal"])?;
        m.set_global("Decimal", rt::import_from(&decimal, "Decimal")?)?;
        frame.line(17);
        m.set_global("json", rt::import(m, "json")?)?;
        frame.line(18);
        let json = rt::import_names(m, "json", &["decoder", "encoder", "scanner"])?;
        m.set_global("decoder", rt::import_from(&json, "decoder")?)?;
        m.set_global("encoder", rt::import_from(&json, "encoder")?)?;
        m.set_global("scanner", rt::import_from(&json, "scanner")?)?;
        frame.line(20);
        m.set_global("DOC", "{\"b\": [1, 2.5, \"caf\\u00e9\", null, true], \"a\": {\"nested\": \"\\ud83d\\ude00\"}, \"n\": -0.0}")?;
        rt::def!(m, accelerated(), doc = "Which functions came from `_json`.")?;
        rt::def!(m, roundtrips())?;
        rt::def!(
            m,
            pure_python(),
            doc = "The fallbacks the package uses without `_json`."
        )?;
        frame.line(67);
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
        frame.line(73);
        m.set_global(
            "PointEncoder",
            rt::class!(
                m,
                "PointEncoder"(m.global("json")?.getattr("JSONEncoder")?),
                point_encoder::body,
                line = 73
            )?,
        )?;
        rt::def!(m, errors())?;
        rt::def!(m, main())?;
        frame.line(112);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(113);
            if m.global("sys")?
                .getattr("argv")?
                .get_item(rt::slice(py, 1, 2)?)?
                .eq(PyList::new(py, ["tool"])?)?
            {
                frame.line(114);
                m.set_global("json", rt::import(m, "json.tool")?)?;
                frame.line(116);
                let value = PyList::empty(py);
                m.global("sys")?
                    .getattr("argv")?
                    .set_item(rt::slice(py, 1, 2)?, value)?;
                frame.line(117);
                m.global("json")?.getattr("tool")?.call_method0("main")?;
            } else {
                frame.line(118);
                if m.global("sys")?
                    .getattr("argv")?
                    .get_item(rt::slice(py, 1, 2)?)?
                    .eq(PyList::new(py, ["crash"])?)?
                {
                    frame.line(119);
                    m.global("json")?
                        .call_method1("loads", ("{\"ok\": [1, 2, ]}",))?;
                } else {
                    frame.line(121);
                    m.global("main")?.call0()?;
                }
            }
        }
        Ok(())
    })
}
