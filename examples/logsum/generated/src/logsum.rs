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
    let parser = rt::call!(
        m.global("argparse")?.getattr("ArgumentParser")?,
        prog = "logsum",
        description = m
            .global("__doc__")?
            .call_method0("splitlines")?
            .get_item(0)?
    )?;
    rt::call!(
        parser.getattr("add_argument")?,
        "logfile",
        help = "access log to read"
    )?;
    rt::call!(
        parser.getattr("add_argument")?,
        "--top",
        r#type = rt::builtin(py, "int")?,
        default = m.global("DEFAULT_TOP")?,
        help = "paths to list"
    )?;
    rt::call!(
        parser.getattr("add_argument")?,
        "--slow",
        r#type = rt::builtin(py, "int")?,
        default = m.global("SLOW_MS")?,
        help = "slow request threshold (ms)"
    )?;
    parser.call_method1("parse_args", (argv,))
}

// logsum.py:28
/// Split one log line into a request record, or return None if it is malformed.
pub fn parse_line<'py>(
    m: &rt::Module<'py>,
    line: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let parts = line.call_method0("split")?;
    if parts.len()? < 10
        || !parts
            .get_item(3)?
            .call_method1("startswith", ("[",))?
            .is_truthy()?
    {
        return Ok(rt::none(py));
    }
    let status = parts.get_item(8)?;
    if status.len()? != 3 || !status.call_method0("isdigit")?.is_truthy()? {
        return Ok(rt::none(py));
    }
    let size = parts.get_item(9)?;
    let mut millis = rt::none(py);
    if parts.len()? > 10
        && parts
            .get_item(10)?
            .call_method1("endswith", ("ms",))?
            .is_truthy()?
    {
        millis = rt::builtin(py, "int")?.call1((parts.get_item(10)?.get_item(rt::slice(
            py,
            py.None(),
            -2,
        )?)?,))?;
    }
    Ok([
        ("ip", &parts.get_item(0)?),
        (
            "hour",
            &parts.get_item(3)?.get_item(rt::slice(py, 13, 15)?)?,
        ),
        (
            "method",
            &parts.get_item(5)?.call_method1("lstrip", ("\"",))?,
        ),
        (
            "path",
            &parts
                .get_item(6)?
                .call_method1("split", ("?",))?
                .get_item(0)?,
        ),
        ("status", &rt::builtin(py, "int")?.call1((&status,))?),
        (
            "bytes",
            &(if size.eq("-")? {
                rt::int(py, 0)
            } else {
                rt::builtin(py, "int")?.call1((&size,))?
            }),
        ),
        ("ms", &millis),
    ]
    .into_py_dict(py)?
    .into_any())
}

// logsum.py:51
/// Parse every line of the log, counting the lines that are not requests.
pub fn load<'py>(m: &rt::Module<'py>, path: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut request: Bound<'py, PyAny>;
    let requests = PyList::empty(py).into_any();
    let mut malformed = rt::int(py, 0);
    for line in rt::call!(path.getattr("read_text")?, encoding = "utf-8")?
        .call_method0("splitlines")?
        .try_iter()?
    {
        let line = line?;
        py.check_signals()?;
        if !line.call_method0("strip")?.is_truthy()? {
            continue;
        }
        request = parse_line(m, &line)?;
        if request.is_none() {
            malformed = rt::iadd(&malformed, 1)?;
        } else {
            requests.call_method1("append", (&request,))?;
        }
    }
    Ok(PyTuple::new(py, [&requests, &malformed])?.into_any())
}

// logsum.py:66
/// Nearest-rank percentile of an ascending list of numbers.
pub fn percentile<'py>(
    m: &rt::Module<'py>,
    sorted_values: &Bound<'py, PyAny>,
    pct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if !sorted_values.is_truthy()? {
        return Ok(rt::int(py, 0));
    }
    let rank = pct.mul(sorted_values.len()?)?.add(99)?.floor_div(100)?;
    sorted_values.get_item(rt::builtin(py, "max")?.call1((&rank, 1))?.sub(1)?)
}

// logsum.py:74
pub fn bar<'py>(
    m: &rt::Module<'py>,
    count: &Bound<'py, PyAny>,
    peak: &Bound<'py, PyAny>,
    width: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if peak.eq(0)? {
        return Ok(rt::str(py, ""));
    }
    rt::str(py, "#").mul(rt::builtin(py, "max")?.call1((1, count.mul(width)?.floor_div(peak)?))?)
}

// logsum.py:80
pub fn tally<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
    field: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut key: Bound<'py, PyAny>;
    let counts = PyDict::new(py).into_any();
    for request in requests.try_iter()? {
        let request = request?;
        py.check_signals()?;
        key = request.get_item(field)?;
        counts.set_item(&key, counts.call_method1("get", (&key, 0))?.add(1)?)?;
    }
    Ok(counts)
}

// logsum.py:88
pub fn print_statuses<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut name: Bound<'py, PyAny>;
    let classes = PyDict::new(py).into_any();
    for request in requests.try_iter()? {
        let request = request?;
        py.check_signals()?;
        name = m
            .global("STATUS_CLASSES")?
            .getattr("get")?
            .call1((request.get_item("status")?.floor_div(100)?, "other"))?;
        classes.set_item(&name, classes.call_method1("get", (&name, 0))?.add(1)?)?;
    }
    let peak = rt::call!(
        rt::builtin(py, "max")?,
        classes.call_method0("values")?,
        default = 0
    )?;
    rt::builtins::print(py, ("status",))?;
    // logsum.py:95
    fn lambda<'py>(
        m: &rt::Module<'py>,
        classes: &Bound<'py, PyAny>,
        n: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyTuple::new(m.py(), [&classes.get_item(n)?.neg()?, n])?.into_any())
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
        py.check_signals()?;
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
    }
    Ok(rt::none(py))
}

// logsum.py:99
pub fn print_paths<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
    top: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let paths = tally(m, requests, &rt::str(py, "path"))?;
    // logsum.py:101
    fn lambda<'py>(m: &rt::Module<'py>, item: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyTuple::new(m.py(), [&item.get_item(1)?.neg()?, &item.get_item(0)?])?.into_any())
    }
    let ranked = rt::call!(
        rt::builtin(py, "sorted")?,
        paths.call_method0("items")?,
        key = rt::function!(m, lambda(item), qualname = "print_paths.<locals>.<lambda>")?
    )?;
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
    for item in ranked
        .get_item(rt::slice(py, py.None(), top)?)?
        .try_iter()?
    {
        let [path, count] = rt::unpack(&item?)?;
        py.check_signals()?;
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
    }
    Ok(rt::none(py))
}

// logsum.py:107
pub fn print_latency<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
    slow: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let mut request: Bound<'py, PyAny>;
    let timed = PyList::empty(py).into_any();
    for item in requests.try_iter()? {
        request = item?;
        py.check_signals()?;
        if !request.get_item("ms")?.is_none() {
            timed.getattr("append")?.call1((request.get_item("ms")?,))?;
        }
    }
    timed.call_method0("sort")?;
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
    for pct in m.global("PERCENTILES")?.try_iter()? {
        let pct = pct?;
        py.check_signals()?;
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
    }
    let slow_requests = PyList::empty(py).into_any();
    for item2 in requests.try_iter()? {
        request = item2?;
        py.check_signals()?;
        if !request.get_item("ms")?.is_none() && request.get_item("ms")?.ge(slow)? {
            slow_requests.call_method1("append", (&request,))?;
        }
    }
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
    // logsum.py:121
    fn lambda<'py>(m: &rt::Module<'py>, r: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        r.get_item("ms")?.neg()
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
        py.check_signals()?;
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
    }
    Ok(rt::none(py))
}

// logsum.py:125
pub fn print_hours<'py>(
    m: &rt::Module<'py>,
    requests: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let hours = tally(m, requests, &rt::str(py, "hour"))?;
    let peak = rt::call!(
        rt::builtin(py, "max")?,
        hours.call_method0("values")?,
        default = 0
    )?;
    rt::builtins::print(py, ("requests per hour",))?;
    for hour in rt::builtin(py, "sorted")?.call1((&hours,))?.try_iter()? {
        let hour = hour?;
        py.check_signals()?;
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
    }
    Ok(rt::none(py))
}

// logsum.py:133
pub fn main<'py>(m: &rt::Module<'py>, argv: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let args = parse_args(m, argv)?;
    let path = m.global("Path")?.call1((args.getattr("logfile")?,))?;
    if !path.call_method0("is_file")?.is_truthy()? {
        rt::call!(
            rt::builtin(py, "print")?,
            rt::fstring(py, ("logsum: ", rt::format(&path, "")?, ": no such file"))?,
            file = m.global("sys")?.getattr("stderr")?
        )?;
        return Ok(rt::int(py, 2));
    }
    let [requests, malformed] = rt::unpack(&load(m, &path)?)?;
    let mut total_bytes = rt::int(py, 0);
    let clients = rt::builtin(py, "set")?.call0()?;
    for request in requests.try_iter()? {
        let request = request?;
        py.check_signals()?;
        total_bytes = rt::iadd(&total_bytes, request.get_item("bytes")?)?;
        clients.getattr("add")?.call1((request.get_item("ip")?,))?;
    }
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
    if malformed.is_truthy()? {
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
    rt::builtins::print(py, ())?;
    print_statuses(m, &requests)?;
    rt::builtins::print(py, ())?;
    print_paths(m, &requests, &args.getattr("top")?)?;
    rt::builtins::print(py, ())?;
    print_latency(m, &requests, &args.getattr("slow")?)?;
    rt::builtins::print(py, ())?;
    print_hours(m, &requests)?;
    Ok(rt::int(py, 0))
}

/// Executes the body of `logsum.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global("__doc__", "Summarize a web server access log.\n\nEach line of the log is in the common log format, followed by the time it\ntook to serve the request:\n\n    203.0.113.9 - - [12/Mar/2024:10:15:32 +0000] \"GET /api/users HTTP/1.1\" 200 512 41ms\n")?;
    m.set_global("argparse", rt::import(m, "argparse")?)?;
    m.set_global("sys", rt::import(m, "sys")?)?;
    let pathlib = rt::import_names(m, "pathlib", &["Path"])?;
    m.set_global("Path", rt::import_from(&pathlib, "Path")?)?;
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
    m.set_global("DEFAULT_TOP", 5)?;
    m.set_global("SLOW_MS", 500)?;
    m.set_global("PERCENTILES", PyTuple::new(py, [50, 90, 99])?)?;
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
    rt::def!(m, bar(count, peak, width = m.global("BAR_WIDTH")?))?;
    rt::def!(m, tally(requests, field))?;
    rt::def!(m, print_statuses(requests))?;
    rt::def!(m, print_paths(requests, top))?;
    rt::def!(m, print_latency(requests, slow))?;
    rt::def!(m, print_hours(requests))?;
    rt::def!(m, main(argv))?;
    if m.global("__name__")?.eq("__main__")? {
        m.global("sys")?.getattr("exit")?.call1((main(
            m,
            &m.global("sys")?
                .getattr("argv")?
                .get_item(rt::slice(py, 1, py.None())?)?,
        )?,))?;
    }
    Ok(())
}
