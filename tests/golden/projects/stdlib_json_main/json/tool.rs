use pyo3::exceptions::PyBrokenPipeError;
use pyo3::exceptions::PySystemExit;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyIterator;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// json/tool.py:37
#[rt::frame(m, "_colorize_json", 37, frame)]
pub fn _colorize_json<'py>(
    m: &rt::Module<'py>,
    json_str: &Bound<'py, PyAny>,
    theme: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // json/tool.py:38
    #[rt::frame(module, "_colorize_json.<locals>._replace_match_callback", 38)]
    fn _replace_match_callback<'py>(
        module: &rt::Module<'py>,
        theme: &Bound<'py, PyAny>,
        r#match: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut m: Bound<'py, PyAny>;
        for item in rt::iter(&module.global("_group_to_theme_color")?.method0("items")?)? {
            let [group, color] = rt::unpack(&item?)?;
            m = r#match.method1("group", (&group,))?;
            if m.is_truthy()? {
                return rt::fstring(
                    module.py(),
                    (
                        rt::format(&theme.get_item(&color)?, "")?,
                        rt::format(&m, "")?,
                        rt::format(&theme.attr("reset")?, "")?,
                    ),
                );
            }
        }
        r#match.method0("group")
    }
    let _replace_match_callback = rt::function!(frame, _replace_match_callback[theme](r#match))?;
    m.global("re")?.attr("sub")?.call1((
        m.global("_color_pattern")?,
        &_replace_match_callback,
        json_str,
    ))
}

// json/tool.py:47
#[rt::frame(m, "main", 47, frame)]
pub fn main<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let description = rt::str(
        py,
        "A simple command line interface for json module to validate and pretty-print JSON objects.",
    );
    let parser = rt::call!(
        m.global("argparse")?.attr("ArgumentParser")?,
        description = &description,
        color = true
    )?;
    rt::call!(
        parser.attr("add_argument")?,
        "infile",
        nargs = "?",
        help = "a JSON file to be validated or pretty-printed",
        default = "-"
    )?;
    rt::call!(
        parser.attr("add_argument")?,
        "outfile",
        nargs = "?",
        help = "write the output of infile to outfile",
        default = py.None()
    )?;
    rt::call!(
        parser.attr("add_argument")?,
        "--sort-keys",
        action = "store_true",
        default = false,
        help = "sort the output of dictionaries alphabetically by key"
    )?;
    rt::call!(
        parser.attr("add_argument")?,
        "--no-ensure-ascii",
        dest = "ensure_ascii",
        action = "store_false",
        help = "disable escaping of non-ASCII characters"
    )?;
    rt::call!(
        parser.attr("add_argument")?,
        "--json-lines",
        action = "store_true",
        default = false,
        help = "parse input using the JSON Lines format. Use with --no-indent or --compact to produce valid JSON Lines output."
    )?;
    let group = parser.method0("add_mutually_exclusive_group")?;
    rt::call!(
        group.attr("add_argument")?,
        "--indent",
        default = 4,
        r#type = rt::builtin(py, "int")?,
        help = "separate items with newlines and use this number of spaces for indentation"
    )?;
    rt::call!(
        group.attr("add_argument")?,
        "--tab",
        action = "store_const",
        dest = "indent",
        r#const = "\t",
        help = "separate items with newlines and use tabs for indentation"
    )?;
    rt::call!(
        group.attr("add_argument")?,
        "--no-indent",
        action = "store_const",
        dest = "indent",
        r#const = py.None(),
        help = "separate items with spaces rather than newlines"
    )?;
    rt::call!(
        group.attr("add_argument")?,
        "--compact",
        action = "store_true",
        help = "suppress all whitespace separation (most compact)"
    )?;
    let options = parser.method0("parse_args")?;
    let dump_args = [
        ("sort_keys", &options.attr("sort_keys")?),
        ("indent", &options.attr("indent")?),
        ("ensure_ascii", &options.attr("ensure_ascii")?),
    ]
    .into_py_dict(py)?;
    if options.attr("compact")?.is_truthy()? {
        dump_args.set_item("indent", py.None())?;
        dump_args.set_item("separators", PyTuple::new(py, [",", ":"])?)?;
    }
    match (|| -> PyResult<()> {
        let infile = if options.attr("infile")?.eq("-")? {
            m.global("sys")?.attr("stdin")?
        } else {
            rt::call!(
                rt::builtin(py, "open")?,
                options.attr("infile")?,
                encoding = "utf-8"
            )?
        };
        let mut objs: Option<Bound<'py, PyAny>> = None;
        let result = (|| -> PyResult<()> {
            let lines: Bound<'py, PyAny>;
            if options.attr("json_lines")?.is_truthy()? {
                lines = infile.method0("readlines")?;
                // json/tool.py:95
                #[rt::frame(m, "main.<locals>.<genexpr>", 95)]
                fn genexpr<'py>(
                    m: &rt::Module<'py>,
                    lines: &Bound<'py, PyIterator>,
                ) -> PyResult<Option<Bound<'py, PyAny>>> {
                    if let Some(line) = rt::next(lines)? {
                        return Ok(Some(m.global("json")?.method1("loads", (&line,))?));
                    }
                    Ok(None)
                }
                objs = Some(rt::genexpr!(frame, genexpr(lines.try_iter()?))?);
            } else {
                objs = Some(
                    PyTuple::new(py, [&m.global("json")?.method1("load", (&infile,))?])?.into_any(),
                );
            }
            Ok(())
        })();
        let finally = frame.finally(result);
        if !infile.is(m.global("sys")?.attr("stdin")?) {
            infile.method0("close")?;
        }
        finally.resume()?;
        let outfile = if options.attr("outfile")?.is_none() {
            m.global("sys")?.attr("stdout")?
        } else {
            rt::call!(
                rt::builtin(py, "open")?,
                options.attr("outfile")?,
                "w",
                encoding = "utf-8"
            )?
        };
        frame.with(&outfile, |_| {
            let t: Bound<'py, PyAny>;
            let mut obj: Bound<'py, PyAny>;
            let mut json_str: Bound<'py, PyAny>;
            if rt::call!(m.global("can_colorize")?, file = &outfile)?.is_truthy()? {
                t = rt::call!(m.global("get_theme")?, tty_file = &outfile)?.attr("syntax")?;
                for item in rt::iter(rt::local(&objs, "objs")?)? {
                    obj = item?;
                    json_str = rt::call!(m.global("json")?.attr("dumps")?, &obj, **dump_args)?;
                    outfile
                        .attr("write")?
                        .call1((_colorize_json(m, &json_str, &t)?,))?;
                    outfile.method1("write", ("\n",))?;
                }
            } else {
                for item2 in rt::iter(rt::local(&objs, "objs")?)? {
                    obj = item2?;
                    rt::call!(m.global("json")?.attr("dump")?, &obj, &outfile, **dump_args)?;
                    outfile.method1("write", ("\n",))?;
                }
            }
            Ok(())
        })?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            return Err(frame.raise(PySystemExit::new_err((e.clone().unbind(),))));
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

/// Executes the body of `json/tool.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Command-line tool to validate and pretty-print JSON\n\
            \n\
            See `json.__main__` for a usage example (invocation as\n\
            `python -m json.tool` is supported for backwards compatibility).\n",
    )?;
    // Copied unmodified (below this header) from Lib/json/tool.py of CPython 3.14.9,
    // licensed under the PSF License Agreement. A steelix test project:
    // tests/test_projects.py runs it transpiled and with CPython and compares.
    m.set_global("argparse", rt::import(m, "argparse")?)?;
    m.set_global("json", rt::import(m, "json")?)?;
    m.set_global("re", rt::import(m, "re")?)?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    let _colorize = rt::import_names(m, "_colorize", &["get_theme", "can_colorize"])?;
    m.set_global("get_theme", rt::import_from(&_colorize, "get_theme")?)?;
    m.set_global("can_colorize", rt::import_from(&_colorize, "can_colorize")?)?;
    // The string we are colorizing is valid JSON,
    // so we can use a looser but simpler regex to match
    // the various parts, most notably strings and numbers,
    // where the regex given by the spec is much more complex.
    m.set_global("_color_pattern", m.global("re")?.attr("compile")?.call1(("\n    (?P<key>\"(\\\\.|[^\"\\\\])*\")(?=:)           |\n    (?P<string>\"(\\\\.|[^\"\\\\])*\")             |\n    (?P<number>NaN|-?Infinity|[0-9\\-+.Ee]+) |\n    (?P<boolean>true|false)                 |\n    (?P<null>null)\n", m.global("re")?.attr("VERBOSE")?))?)?;
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
    if m.global("__name__")?.eq("__main__")? {
        match main(m) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyBrokenPipeError>(py) => {
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
