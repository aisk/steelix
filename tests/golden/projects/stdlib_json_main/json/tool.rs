use pyo3::exceptions::PyBrokenPipeError;
use pyo3::exceptions::PySystemExit;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// json/tool.py:37
pub fn _colorize_json<'py>(
    m: &rt::Module<'py>,
    json_str: &Bound<'py, PyAny>,
    theme: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "_colorize_json", 37, |frame| {
        // json/tool.py:38
        fn _replace_match_callback<'py>(
            module: &rt::Module<'py>,
            theme: &Bound<'py, PyAny>,
            r#match: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = module.py();
            rt::frame!(
                module,
                "_colorize_json.<locals>._replace_match_callback",
                38,
                |frame| {
                    let mut m: Bound<'py, PyAny>;
                    frame.line(39);
                    for item in module
                        .global("_group_to_theme_color")?
                        .call_method0("items")?
                        .try_iter()?
                    {
                        let [group, color] = rt::unpack(&item?)?;
                        frame.line(40);
                        m = r#match.call_method1("group", (&group,))?;
                        if m.is_truthy()? {
                            frame.line(41);
                            return rt::fstring(
                                py,
                                (
                                    rt::format(&theme.get_item(&color)?, "")?,
                                    rt::format(&m, "")?,
                                    rt::format(&theme.getattr("reset")?, "")?,
                                ),
                            );
                        }
                        py.check_signals()?;
                        frame.line(39);
                    }
                    frame.line(42);
                    r#match.call_method0("group")
                }
            )
        }
        let _replace_match_callback = rt::function!(
            m,
            _replace_match_callback[theme](r#match),
            qualname = "_colorize_json.<locals>._replace_match_callback"
        )?;
        frame.line(44);
        m.global("re")?.getattr("sub")?.call1((
            m.global("_color_pattern")?,
            &_replace_match_callback,
            json_str,
        ))
    })
}

// json/tool.py:47
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 47, |frame| {
        let description = rt::str(
            py,
            "A simple command line interface for json module to validate and pretty-print JSON objects.",
        );
        frame.line(50);
        let parser = rt::call!(
            m.global("argparse")?.getattr("ArgumentParser")?,
            description = &description,
            color = true
        )?;
        frame.line(51);
        rt::call!(
            parser.getattr("add_argument")?,
            "infile",
            nargs = "?",
            help = "a JSON file to be validated or pretty-printed",
            default = "-"
        )?;
        frame.line(54);
        rt::call!(
            parser.getattr("add_argument")?,
            "outfile",
            nargs = "?",
            help = "write the output of infile to outfile",
            default = py.None()
        )?;
        frame.line(57);
        rt::call!(
            parser.getattr("add_argument")?,
            "--sort-keys",
            action = "store_true",
            default = false,
            help = "sort the output of dictionaries alphabetically by key"
        )?;
        frame.line(59);
        rt::call!(
            parser.getattr("add_argument")?,
            "--no-ensure-ascii",
            dest = "ensure_ascii",
            action = "store_false",
            help = "disable escaping of non-ASCII characters"
        )?;
        frame.line(61);
        rt::call!(
            parser.getattr("add_argument")?,
            "--json-lines",
            action = "store_true",
            default = false,
            help = "parse input using the JSON Lines format. Use with --no-indent or --compact to produce valid JSON Lines output."
        )?;
        frame.line(64);
        let group = parser.call_method0("add_mutually_exclusive_group")?;
        frame.line(65);
        rt::call!(
            group.getattr("add_argument")?,
            "--indent",
            default = 4,
            r#type = rt::builtin(py, "int")?,
            help = "separate items with newlines and use this number of spaces for indentation"
        )?;
        frame.line(68);
        rt::call!(
            group.getattr("add_argument")?,
            "--tab",
            action = "store_const",
            dest = "indent",
            r#const = "\t",
            help = "separate items with newlines and use tabs for indentation"
        )?;
        frame.line(71);
        rt::call!(
            group.getattr("add_argument")?,
            "--no-indent",
            action = "store_const",
            dest = "indent",
            r#const = py.None(),
            help = "separate items with spaces rather than newlines"
        )?;
        frame.line(74);
        rt::call!(
            group.getattr("add_argument")?,
            "--compact",
            action = "store_true",
            help = "suppress all whitespace separation (most compact)"
        )?;
        frame.line(76);
        let options = parser.call_method0("parse_args")?;
        frame.line(78);
        let dump_args = [
            ("sort_keys", &options.getattr("sort_keys")?),
            ("indent", &options.getattr("indent")?),
            ("ensure_ascii", &options.getattr("ensure_ascii")?),
        ]
        .into_py_dict(py)?
        .into_any();
        frame.line(83);
        if options.getattr("compact")?.is_truthy()? {
            frame.line(84);
            dump_args.set_item("indent", py.None())?;
            frame.line(85);
            dump_args.set_item("separators", PyTuple::new(py, [",", ":"])?)?;
        }
        match (|| -> PyResult<()> {
            let infile: Bound<'py, PyAny>;
            let mut objs: Option<Bound<'py, PyAny>> = None;
            let outfile: Bound<'py, PyAny>;
            frame.line(88);
            if options.getattr("infile")?.eq("-")? {
                frame.line(89);
                infile = m.global("sys")?.getattr("stdin")?;
            } else {
                frame.line(91);
                infile = rt::call!(
                    rt::builtin(py, "open")?,
                    options.getattr("infile")?,
                    encoding = "utf-8"
                )?;
            }
            let result = (|| -> PyResult<()> {
                let lines: Bound<'py, PyAny>;
                frame.line(93);
                if options.getattr("json_lines")?.is_truthy()? {
                    frame.line(94);
                    lines = infile.call_method0("readlines")?;
                    // json/tool.py:95
                    fn genexpr<'py>(
                        m: &rt::Module<'py>,
                        lines: &Bound<'py, PyIterator>,
                    ) -> PyResult<Option<Bound<'py, PyAny>>> {
                        rt::frame!(m, "main.<locals>.<genexpr>", 95, |_| {
                            if let Some(line) = rt::next(lines)? {
                                return Ok(Some(
                                    m.global("json")?.call_method1("loads", (&line,))?,
                                ));
                            }
                            Ok(None)
                        })
                    }
                    frame.line(95);
                    objs = Some(rt::genexpr!(
                        m,
                        genexpr(lines.try_iter()?),
                        qualname = "main.<locals>.<genexpr>"
                    )?);
                } else {
                    frame.line(97);
                    objs = Some(
                        PyTuple::new(py, [&m.global("json")?.call_method1("load", (&infile,))?])?
                            .into_any(),
                    );
                }
                Ok(())
            })();
            let finally = frame.finally(result);
            frame.line(99);
            if !infile.is(m.global("sys")?.getattr("stdin")?) {
                frame.line(100);
                infile.call_method0("close")?;
            }
            finally.resume()?;
            frame.line(102);
            if options.getattr("outfile")?.is_none() {
                frame.line(103);
                outfile = m.global("sys")?.getattr("stdout")?;
            } else {
                frame.line(105);
                outfile = rt::call!(
                    rt::builtin(py, "open")?,
                    options.getattr("outfile")?,
                    "w",
                    encoding = "utf-8"
                )?;
            }
            frame.with(&outfile, |_| {
                let t: Bound<'py, PyAny>;
                let mut obj: Bound<'py, PyAny>;
                let mut json_str: Bound<'py, PyAny>;
                frame.line(107);
                if rt::call!(m.global("can_colorize")?, file = &outfile)?.is_truthy()? {
                    frame.line(108);
                    t = rt::call!(m.global("get_theme")?, tty_file = &outfile)?
                        .getattr("syntax")?;
                    frame.line(109);
                    for item in rt::local(&objs, "objs")?.try_iter()? {
                        obj = item?;
                        frame.line(110);
                        json_str =
                            rt::call!(m.global("json")?.getattr("dumps")?, &obj, **dump_args)?;
                        frame.line(111);
                        outfile
                            .getattr("write")?
                            .call1((_colorize_json(m, &json_str, &t)?,))?;
                        frame.line(112);
                        outfile.call_method1("write", ("\n",))?;
                        py.check_signals()?;
                        frame.line(109);
                    }
                } else {
                    frame.line(114);
                    for item2 in rt::local(&objs, "objs")?.try_iter()? {
                        obj = item2?;
                        frame.line(115);
                        rt::call!(
                            m.global("json")?.getattr("dump")?,
                            &obj,
                            &outfile,
                            **dump_args
                        )?;
                        frame.line(116);
                        outfile.call_method1("write", ("\n",))?;
                        py.check_signals()?;
                        frame.line(114);
                    }
                }
                Ok(())
            })?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                frame.line(118);
                return Err(frame.raise(PySystemExit::new_err((e.clone().unbind(),))));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

/// Executes the body of `json/tool.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Command-line tool to validate and pretty-print JSON\n\nSee `json.__main__` for a usage example (invocation as\n`python -m json.tool` is supported for backwards compatibility).\n")?;
        // Copied unmodified (below this header) from Lib/json/tool.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        frame.line(9);
        m.set_global("argparse", rt::import(m, "argparse")?)?;
        frame.line(10);
        m.set_global("json", rt::import(m, "json")?)?;
        frame.line(11);
        m.set_global("re", rt::import(m, "re")?)?;
        frame.line(12);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(13);
        let _colorize = rt::import_names(m, "_colorize", &["get_theme", "can_colorize"])?;
        m.set_global("get_theme", rt::import_from(&_colorize, "get_theme")?)?;
        m.set_global("can_colorize", rt::import_from(&_colorize, "can_colorize")?)?;
        // The string we are colorizing is valid JSON,
        // so we can use a looser but simpler regex to match
        // the various parts, most notably strings and numbers,
        // where the regex given by the spec is much more complex.
        frame.line(20);
        m.set_global("_color_pattern", m.global("re")?.getattr("compile")?.call1(("\n    (?P<key>\"(\\\\.|[^\"\\\\])*\")(?=:)           |\n    (?P<string>\"(\\\\.|[^\"\\\\])*\")             |\n    (?P<number>NaN|-?Infinity|[0-9\\-+.Ee]+) |\n    (?P<boolean>true|false)                 |\n    (?P<null>null)\n", m.global("re")?.getattr("VERBOSE")?))?)?;
        frame.line(28);
        m.set_global(
            "_group_to_theme_color",
            [
                ("key", "definition"),
                ("string", "string"),
                ("number", "number"),
                ("boolean", "keyword"),
                ("null", "keyword"),
            ]
            .into_py_dict(py)?,
        )?;
        rt::def!(m, _colorize_json(json_str, theme))?;
        rt::def!(m, main())?;
        frame.line(121);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(123);
            match main(m) {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyBrokenPipeError>(py) => {
                    let exception = frame.catch(err);
                    m.set_global("exc", &exception)?;
                    let _handling = rt::Handling::new(&exception);
                    frame.line(125);
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
