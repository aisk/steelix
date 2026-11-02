# logsum: rewriting a transpiled app one function at a time

`logsum.py` is a small CLI that summarizes a web server access log: requests
per status class, the busiest paths, latency percentiles, slow requests and
traffic per hour. It uses `argparse`, `pathlib`, module constants and a few
lambdas.

```sh
python logsum.py access.log --top 3 --slow 100
```

The two directories show the migration path of steelix:

- `generated/src/logsum.rs` is what `steelix transpile logsum.py` produces,
  untouched. The binary behaves exactly like `python logsum.py`.
- `rewritten/src/logsum.rs` is a copy in which three functions were
  rewritten by hand, showing the two steps of section 15 of
  `docs/design.md`:
  - `tally` is a first step: its logic is Rust, but it still works on the
    Python list and dicts it is given and returns a Python dict, through
    typed PyO3 APIs (`cast::<PyDict>()`, `get_item`, iteration), without
    copying anything;
  - `parse_line` and `percentile` went all the way: their logic lives in
    plain Rust in `rewritten/src/logsum/native.rs`, which knows nothing about
    Python and has its own unit tests.

`tests/test_examples.py` checks that `generated/` is still the transpiler's
output and that both versions print exactly what CPython prints for several
command lines, except for one difference the rewrite makes on purpose
(below).

## The convention

The generated function is the Python-facing adapter. Its signature
`(m: &rt::Module<'py>, args: &Bound<'py, PyAny>...) -> PyResult<Bound<'py, PyAny>>`
stays as it is, because two kinds of callers rely on it: the function object
registered by `rt::def!` (used by Python code, key functions and decorators)
and direct calls from other generated functions. Rewriting a function means:

1. write the logic as a new Rust function with native types, in a module that
   does not use PyO3;
2. reduce the generated function to conversions: `extract` the arguments, call
   the native function, convert the result back.

None of the callers change. See section 15 of `docs/design.md` for the full
convention, including functions called through Python and what stops working
once a function is native.

## `parse_line`, before and after

Generated:

```rust
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
        // ... five more fields ...
        ("ms", &millis),
    ]
    .into_py_dict(py)?
    .into_any())
}
```

Rewritten, in `logsum.rs` (the adapter):

```rust
// logsum.py:28
/// Split one log line into a request record, or return None if it is malformed.
pub fn parse_line<'py>(
    m: &rt::Module<'py>,
    line: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let Some(request) = native::parse_line(line.extract()?) else {
        return Ok(rt::none(py));
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
```

and in `logsum/native.rs`:

```rust
/// Splits one log line into a request, or returns `None` if it is malformed.
pub fn parse_line(line: &str) -> Option<Request<'_>> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 || !parts[3].starts_with('[') {
        return None;
    }
    let status = parts[8];
    if status.len() != 3 || !status.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let bytes = match parts[9] {
        "-" => 0,
        size => size.parse().ok()?,
    };
    let ms = match parts.get(10).and_then(|time| time.strip_suffix("ms")) {
        Some(ms) => Some(ms.parse().ok()?),
        None => None,
    };
    let (path, _query) = parts[6].split_once('?').unwrap_or((parts[6], ""));
    Some(Request {
        ip: parts[0],
        hour: parts[3].get(13..15).unwrap_or_default(),
        method: parts[5].trim_start_matches('"'),
        path,
        status: status.parse().ok()?,
        bytes,
        ms,
    })
}
```

`load`, which calls `parse_line(m, &line)?` for every line, did not change.
Python code calling `parse_line` would not notice the difference either,
except for the numbers. The Python version converts them with `int()` and
the Rust one with `str::parse::<u64>`, which accept different strings:

- `int()` also accepts a sign (`-5`), digits grouped with underscores
  (`1_024`) and non-ASCII digits (`٤٢`). The rewritten `parse_line` treats
  such a line as malformed, where the Python version read the number.
- A number neither accepts (`12x`) made the Python version raise
  `ValueError`; the rewritten one counts the line as malformed.

This is deliberate: a log with negative sizes or Arabic-Indic digits is
broken. `access.log` contains one line of each of the first kind, so the
rewritten program prints different totals than CPython for it, and
`tests/test_examples.py` checks exactly that: its output is what CPython
prints for a copy of the log in which those three lines are malformed.

## `tally`, the first step

Rewriting a function into plain Rust types pays off once the data it works
on is Rust too. `tally` returns a dict that `print_paths` and `print_hours`,
still generated, keep using as a dict, so converting it into a `HashMap` would
mean converting it back right away. The first step keeps the data where it is
and only rewrites the logic:

```rust
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
```

The records are still the dicts `parse_line` builds and the result is still
a dict in insertion order, so no caller changes. Once the producer (`load`)
and all the readers of these records are rewritten, they can become a Rust
`struct` and the counts a Rust map in one go.

## Building it yourself

steelix emits a whole crate (`Cargo.toml`, `build.rs`, `src/main.rs`) around
the module file. Those files contain paths of the machine that ran steelix, so
only the module sources are kept here. To build either version:

```sh
uv run steelix transpile examples/logsum/logsum.py -o /tmp/logsum
cp -r examples/logsum/rewritten/src/. /tmp/logsum/src/
cd /tmp/logsum
PYO3_PYTHON=$OLDPWD/.venv/bin/python cargo run -- $OLDPWD/examples/logsum/access.log
PYO3_PYTHON=$OLDPWD/.venv/bin/python cargo test     # the unit tests of native.rs
```
