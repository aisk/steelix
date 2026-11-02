mod native;

use pyo3::IntoPyObjectExt;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// logsum.py:20
pub fn parse_args<'py>(
    m: &rt::Module<'py>,
    argv: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "parse_args", 20, |frame| {
        frame.line(21);
        let parser = rt::call!(
            m.global("argparse")?.getattr("ArgumentParser")?,
            prog = "logsum",
            description = m
                .global("__doc__")?
                .call_method0("splitlines")?
                .get_item(0)?
        )?;
        frame.line(22);
        rt::call!(
            parser.getattr("add_argument")?,
            "logfile",
            help = "access log to read"
        )?;
        frame.line(23);
        rt::call!(
            parser.getattr("add_argument")?,
            "--top",
            r#type = rt::builtin(py, "int")?,
            default = m.global("DEFAULT_TOP")?,
            help = "paths to list"
        )?;
        frame.line(24);
        rt::call!(
            parser.getattr("add_argument")?,
            "--slow",
            r#type = rt::builtin(py, "int")?,
            default = m.global("SLOW_MS")?,
            help = "slow request threshold (ms)"
        )?;
        frame.line(25);
        parser.call_method1("parse_args", (argv,))
    })
}

// logsum.py:28
/// Split one log line into a request record, or return None if it is malformed.
pub fn parse_line<'py>(
    m: &rt::Module<'py>,
    line: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let request = match native::parse_line(line.extract()?) {
        Ok(Some(request)) => request,
        Ok(None) => return Ok(rt::none(py)),
        // The exception the Python version raises (from `int()`), which
        // `load` catches with `except ValueError`.
        Err(native::NotANumber(text)) => {
            return Err(PyValueError::new_err(format!(
                "invalid literal for int() with base 10: '{text}'"
            )));
        }
    };
    let record = PyDict::new(py);
    record.set_item("ip", request.ip)?;
    record.set_item("hour", request.hour)?;
    record.set_item("method", request.method)?;
    record.set_item("path", request.path)?;
    record.set_item("status", request.status)?;
    record.set_item("bytes", request.bytes)?;
    record.set_item("ms", request.ms)?;
    Ok(record.into_any())
}

// logsum.py:51
/// Parse every line of the log, counting the lines that are not requests.
pub fn load<'py>(m: &rt::Module<'py>, path: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "load", 51, |frame| {
        let mut request: Bound<'py, PyAny>;
        let requests = PyList::empty(py).into_any();
        let mut malformed = rt::int(py, 0);
        frame.line(55);
        for line in rt::call!(path.getattr("read_text")?, encoding = "utf-8")?
            .call_method0("splitlines")?
            .try_iter()?
        {
            let line = line?;
            frame.line(56);
            if !line.call_method0("strip")?.is_truthy()? {
                py.check_signals()?;
                frame.line(55);
                continue;
            }
            frame.line(59);
            match parse_line(m, &line) {
                Ok(value) => request = value,
                Err(err) if err.is_instance_of::<PyValueError>(py) => {
                    // A size or time that is not a number.
                    request = rt::none(py);
                }
                Err(err) => return Err(err),
            }
            if request.is_none() {
                frame.line(64);
                malformed = rt::iadd(&malformed, 1)?;
            } else {
                frame.line(66);
                requests.call_method1("append", (&request,))?;
            }
            py.check_signals()?;
            frame.line(55);
        }
        frame.line(67);
        Ok(PyTuple::new(py, [&requests, &malformed])?.into_any())
    })
}

// logsum.py:70
/// Nearest-rank percentile of an ascending list of numbers.
pub fn percentile<'py>(
    m: &rt::Module<'py>,
    sorted_values: &Bound<'py, PyAny>,
    pct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sorted_values: Vec<u64> = sorted_values.extract()?;
    native::percentile(&sorted_values, pct.extract()?).into_bound_py_any(m.py())
}

// logsum.py:78
pub fn bar<'py>(
    m: &rt::Module<'py>,
    count: &Bound<'py, PyAny>,
    peak: &Bound<'py, PyAny>,
    width: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "bar", 78, |frame| {
        frame.line(79);
        if peak.eq(0)? {
            return Ok(rt::str(py, ""));
        }
        frame.line(81);
        rt::str(py, "#")
            .mul(rt::builtin(py, "max")?.call1((1, count.mul(width)?.floor_div(peak)?))?)
    })
}

// logsum.py:84
/// Rewritten in the first step of design §15: the loop is Rust, but the
/// records and the counts stay Python objects, read and written in place
/// through typed PyO3 APIs. `print_paths` and `print_hours`, which are still
/// generated, read the dict this returns, so it stays a dict until they are
/// rewritten too.
pub fn tally<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
    field: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let counts = PyDict::new(m.py());
    for request in requests.cast::<PyList>()? {
        let Some(key) = request.cast::<PyDict>()?.get_item(field)? else {
            // `request[field]` raised `KeyError`.
            return Err(PyKeyError::new_err(field.clone().unbind()));
        };
        let count = match counts.get_item(&key)? {
            Some(count) => count.extract::<u64>()? + 1,
            None => 1,
        };
        counts.set_item(key, count)?;
    }
    Ok(counts.into_any())
}

// logsum.py:92
pub fn print_statuses<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "print_statuses", 92, |frame| {
        let mut name: Bound<'py, PyAny>;
        let classes = PyDict::new(py).into_any();
        frame.line(94);
        for request in requests.try_iter()? {
            let request = request?;
            frame.line(95);
            name = m
                .global("STATUS_CLASSES")?
                .getattr("get")?
                .call1((request.get_item("status")?.floor_div(100)?, "other"))?;
            frame.line(96);
            classes.set_item(&name, classes.call_method1("get", (&name, 0))?.add(1)?)?;
            py.check_signals()?;
            frame.line(94);
        }
        frame.line(97);
        let peak = rt::call!(
            rt::builtin(py, "max")?,
            classes.call_method0("values")?,
            default = 0
        )?;
        frame.line(98);
        rt::builtins::print(py, ("status",))?;
        frame.line(99);
        // logsum.py:99
        fn lambda<'py>(
            m: &rt::Module<'py>,
            classes: &Bound<'py, PyAny>,
            n: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "print_statuses.<locals>.<lambda>", 99, |_| Ok(
                PyTuple::new(m.py(), [&classes.get_item(n)?.neg()?, n])?.into_any()
            ))
        }
        for item in rt::call!(
            rt::builtin(py, "sorted")?,
            &classes,
            key = rt::function!(
                m,
                lambda[classes](n),
                qualname = "print_statuses.<locals>.<lambda>"
            )?
        )?
        .try_iter()?
        {
            name = item?;
            frame.line(100);
            rt::builtins::print(
                py,
                (rt::fstring(
                    py,
                    (
                        "  ",
                        rt::format(&name, "<13")?,
                        " ",
                        rt::format(&classes.get_item(&name)?, ">5")?,
                        "  ",
                        rt::format(
                            &bar(
                                m,
                                &classes.get_item(&name)?,
                                &peak,
                                &m.default("bar", "width")?,
                            )?,
                            "",
                        )?,
                    ),
                )?,),
            )?;
            py.check_signals()?;
            frame.line(99);
        }
        Ok(rt::none(py))
    })
}

// logsum.py:103
pub fn print_paths<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
    top: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "print_paths", 103, |frame| {
        frame.line(104);
        let paths = tally(m, requests, &rt::str(py, "path"))?;
        // logsum.py:105
        fn lambda<'py>(
            m: &rt::Module<'py>,
            item: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "print_paths.<locals>.<lambda>", 105, |_| Ok(
                PyTuple::new(m.py(), [&item.get_item(1)?.neg()?, &item.get_item(0)?])?.into_any()
            ))
        }
        frame.line(105);
        let ranked = rt::call!(
            rt::builtin(py, "sorted")?,
            paths.call_method0("items")?,
            key = rt::function!(m, lambda(item), qualname = "print_paths.<locals>.<lambda>")?
        )?;
        frame.line(106);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    "top ",
                    rt::format(&rt::builtin(py, "min")?.call1((top, ranked.len()?))?, "")?,
                    " of ",
                    rt::format(&rt::int(py, ranked.len()?), "")?,
                    " paths",
                ),
            )?,),
        )?;
        frame.line(107);
        for item in ranked
            .get_item(rt::slice(py, py.None(), top)?)?
            .try_iter()?
        {
            let [path, count] = rt::unpack(&item?)?;
            frame.line(108);
            rt::builtins::print(
                py,
                (rt::fstring(
                    py,
                    (
                        "  ",
                        rt::format(&count, ">5")?,
                        "  ",
                        rt::format(&path, "")?,
                    ),
                )?,),
            )?;
            py.check_signals()?;
            frame.line(107);
        }
        Ok(rt::none(py))
    })
}

// logsum.py:111
pub fn print_latency<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
    slow: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "print_latency", 111, |frame| {
        let mut request: Bound<'py, PyAny>;
        let timed = PyList::empty(py).into_any();
        frame.line(113);
        for item in requests.try_iter()? {
            request = item?;
            frame.line(114);
            if !request.get_item("ms")?.is_none() {
                frame.line(115);
                timed.getattr("append")?.call1((request.get_item("ms")?,))?;
            }
            py.check_signals()?;
            frame.line(113);
        }
        frame.line(116);
        timed.call_method0("sort")?;
        frame.line(117);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    "latency (",
                    rt::format(&rt::int(py, timed.len()?), "")?,
                    " timed requests)",
                ),
            )?,),
        )?;
        frame.line(118);
        for pct in m.global("PERCENTILES")?.try_iter()? {
            let pct = pct?;
            frame.line(119);
            rt::builtins::print(
                py,
                (rt::fstring(
                    py,
                    (
                        "  p",
                        rt::format(&pct, "<3")?,
                        " ",
                        rt::format(&percentile(m, &timed, &pct)?, ">6")?,
                        " ms",
                    ),
                )?,),
            )?;
            py.check_signals()?;
            frame.line(118);
        }
        let slow_requests = PyList::empty(py).into_any();
        frame.line(121);
        for item2 in requests.try_iter()? {
            request = item2?;
            frame.line(122);
            if !request.get_item("ms")?.is_none() && request.get_item("ms")?.ge(slow)? {
                frame.line(123);
                slow_requests.call_method1("append", (&request,))?;
            }
            py.check_signals()?;
            frame.line(121);
        }
        frame.line(124);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    "  ",
                    rt::format(&rt::int(py, slow_requests.len()?), "")?,
                    " slower than ",
                    rt::format(slow, "")?,
                    " ms",
                ),
            )?,),
        )?;
        frame.line(125);
        // logsum.py:125
        fn lambda<'py>(m: &rt::Module<'py>, r: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "print_latency.<locals>.<lambda>", 125, |_| r
                .get_item("ms")?
                .neg())
        }
        for item3 in rt::call!(
            rt::builtin(py, "sorted")?,
            &slow_requests,
            key = rt::function!(m, lambda(r), qualname = "print_latency.<locals>.<lambda>")?
        )?
        .get_item(rt::slice(py, py.None(), 3)?)?
        .try_iter()?
        {
            request = item3?;
            frame.line(126);
            rt::builtins::print(
                py,
                (rt::fstring(
                    py,
                    (
                        "    ",
                        rt::format(&request.get_item("ms")?, ">6")?,
                        " ms  ",
                        rt::format(&request.get_item("method")?, "")?,
                        " ",
                        rt::format(&request.get_item("path")?, "")?,
                    ),
                )?,),
            )?;
            py.check_signals()?;
            frame.line(125);
        }
        Ok(rt::none(py))
    })
}

// logsum.py:129
pub fn print_hours<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "print_hours", 129, |frame| {
        frame.line(130);
        let hours = tally(m, requests, &rt::str(py, "hour"))?;
        frame.line(131);
        let peak = rt::call!(
            rt::builtin(py, "max")?,
            hours.call_method0("values")?,
            default = 0
        )?;
        frame.line(132);
        rt::builtins::print(py, ("requests per hour",))?;
        frame.line(133);
        for hour in rt::builtin(py, "sorted")?.call1((&hours,))?.try_iter()? {
            let hour = hour?;
            frame.line(134);
            rt::builtins::print(
                py,
                (rt::fstring(
                    py,
                    (
                        "  ",
                        rt::format(&hour, "")?,
                        ":00 ",
                        rt::format(&hours.get_item(&hour)?, ">4")?,
                        "  ",
                        rt::format(
                            &bar(m, &hours.get_item(&hour)?, &peak, &rt::int(py, 20))?,
                            "",
                        )?,
                    ),
                )?,),
            )?;
            py.check_signals()?;
            frame.line(133);
        }
        Ok(rt::none(py))
    })
}

// logsum.py:137
pub fn main<'py>(m: &rt::Module<'py>, argv: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "main", 137, |frame| {
        frame.line(138);
        let args = parse_args(m, argv)?;
        frame.line(139);
        let path = m.global("Path")?.call1((args.getattr("logfile")?,))?;
        frame.line(140);
        if !path.call_method0("is_file")?.is_truthy()? {
            frame.line(141);
            rt::call!(
                rt::builtin(py, "print")?,
                rt::fstring(py, ("logsum: ", rt::format(&path, "")?, ": no such file"))?,
                file = m.global("sys")?.getattr("stderr")?
            )?;
            return Ok(rt::int(py, 2));
        }
        frame.line(143);
        let [requests, malformed] = rt::unpack(&load(m, &path)?)?;
        let mut total_bytes = rt::int(py, 0);
        frame.line(145);
        let clients = rt::builtin(py, "set")?.call0()?;
        frame.line(146);
        for request in requests.try_iter()? {
            let request = request?;
            frame.line(147);
            total_bytes = rt::iadd(&total_bytes, request.get_item("bytes")?)?;
            frame.line(148);
            clients.getattr("add")?.call1((request.get_item("ip")?,))?;
            py.check_signals()?;
            frame.line(146);
        }
        frame.line(149);
        rt::call!(
            rt::builtin(py, "print")?,
            rt::fstring(
                py,
                (
                    rt::format(&path.getattr("name")?, "")?,
                    ": ",
                    rt::format(&rt::int(py, requests.len()?), "")?,
                    " requests"
                )
            )?,
            end = ""
        )?;
        frame.line(150);
        rt::builtins::print(
            py,
            (rt::fstring(
                py,
                (
                    " from ",
                    rt::format(&rt::int(py, clients.len()?), "")?,
                    " clients, ",
                    rt::format(&total_bytes.div(1024)?, ".1f")?,
                    " KiB",
                ),
            )?,),
        )?;
        frame.line(151);
        if malformed.is_truthy()? {
            frame.line(152);
            rt::builtins::print(
                py,
                (rt::fstring(
                    py,
                    (
                        "skipped ",
                        rt::format(&malformed, "")?,
                        " malformed line",
                        rt::format(
                            &(if malformed.ne(1)? {
                                rt::str(py, "s")
                            } else {
                                rt::str(py, "")
                            }),
                            "",
                        )?,
                    ),
                )?,),
            )?;
        }
        frame.line(153);
        rt::builtins::print(py, ())?;
        frame.line(154);
        print_statuses(m, &requests)?;
        frame.line(155);
        rt::builtins::print(py, ())?;
        frame.line(156);
        print_paths(m, &requests, &args.getattr("top")?)?;
        frame.line(157);
        rt::builtins::print(py, ())?;
        frame.line(158);
        print_latency(m, &requests, &args.getattr("slow")?)?;
        frame.line(159);
        rt::builtins::print(py, ())?;
        frame.line(160);
        print_hours(m, &requests)?;
        Ok(rt::int(py, 0))
    })
}

/// Executes the body of `logsum.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Summarize a web server access log.\n\nEach line of the log is in the common log format, followed by the time it\ntook to serve the request:\n\n    203.0.113.9 - - [12/Mar/2024:10:15:32 +0000] \"GET /api/users HTTP/1.1\" 200 512 41ms\n")?;
        frame.line(9);
        m.set_global("argparse", rt::import(m, "argparse")?)?;
        frame.line(10);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(11);
        let pathlib = rt::import_names(m, "pathlib", &["Path"])?;
        m.set_global("Path", rt::import_from(&pathlib, "Path")?)?;
        frame.line(13);
        m.set_global(
            "STATUS_CLASSES",
            [
                (2, "success"),
                (3, "redirect"),
                (4, "client error"),
                (5, "server error"),
            ]
            .into_py_dict(py)?,
        )?;
        frame.line(14);
        m.set_global("DEFAULT_TOP", 5)?;
        frame.line(15);
        m.set_global("SLOW_MS", 500)?;
        frame.line(16);
        m.set_global("PERCENTILES", PyTuple::new(py, [50, 90, 99])?)?;
        frame.line(17);
        m.set_global("BAR_WIDTH", 30)?;
        rt::def!(m, parse_args(argv))?;
        rt::def!(
            m,
            parse_line(line),
            doc = "Split one log line into a request record, or return None if it is malformed."
        )?;
        rt::def!(
            m,
            load(path),
            doc = "Parse every line of the log, counting the lines that are not requests."
        )?;
        rt::def!(
            m,
            percentile(sorted_values, pct),
            doc = "Nearest-rank percentile of an ascending list of numbers."
        )?;
        frame.line(78);
        rt::def!(m, bar(count, peak, width = m.global("BAR_WIDTH")?))?;
        rt::def!(m, tally(requests, field))?;
        rt::def!(m, print_statuses(requests))?;
        rt::def!(m, print_paths(requests, top))?;
        rt::def!(m, print_latency(requests, slow))?;
        rt::def!(m, print_hours(requests))?;
        rt::def!(m, main(argv))?;
        frame.line(164);
        if m.global("__name__")?.eq("__main__")? {
            frame.line(165);
            m.global("sys")?.getattr("exit")?.call1((main(
                m,
                &m.global("sys")?
                    .getattr("argv")?
                    .get_item(rt::slice(py, 1, py.None())?)?,
            )?,))?;
        }
        Ok(())
    })
}
