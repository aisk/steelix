# steelix

steelix transpiles Python into Rust code that calls libpython through [PyO3](https://pyo3.rs).

It is meant to help move Python projects to Rust. You transpile the project once and get a Rust program that behaves like the original. Semantics and error messages stay the same, the standard library and third party packages are still imported from Python, and tracebacks still point at the Python source. You can deploy that program right away, then replace its functions with ordinary Rust one at a time, keeping it working after every step.

It is a bit like [Nuitka](https://nuitka.net), but the output is Rust instead of C, and the goal is readability rather than speed. Nuitka's C is meant for a compiler; steelix's Rust is meant to be read and edited by people, and becomes the starting point of the port. It is plain PyO3 plus a small runtime crate.

## Example

```python
import json


class Item:
    def __init__(self, name, price, qty=1):
        self.name = name
        self.price = price
        self.qty = qty

    def total(self):
        return self.price * self.qty


def load(text):
    try:
        rows = json.loads(text)
    except ValueError as e:
        print(f"bad input: {e}")
        return []
    return [Item(**row) for row in rows if row["qty"] > 0]


items = load('[{"name": "pen", "price": 2, "qty": 3}]')
print(sum(item.total() for item in items))
```

`steelix transpile inventory.py -o out/` writes a Cargo crate. An excerpt of `out/src/inventory.rs`:

```rust
// inventory.py:14
#[rt::frame(m, "load", 14, frame)]
pub fn load<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let rows = match (|| m.global("json")?.method1("loads", (text,)))() {
        Ok(value) => value,
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(
                py,
                (rt::fstring(py, ("bad input: ", rt::format(&e, "")?))?,),
            )?;
            return Ok(PyList::empty(py).into_any());
        }
        Err(err) => return Err(err),
    };
    let list = PyList::empty(py);
    for row in rt::iter(&rows)? {
        let row = row?;
        if row.get_item("qty")?.gt(0)? {
            list.append(rt::call!(m.global("Item")?, **row)?)?;
        }
    }
    Ok(list.into_any())
}

pub mod item {
    use super::*;

    // inventory.py:4
    /// The body of `class Item`, run in its namespace `ns`.
    #[rt::frame(ns, "Item", 4)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", name, price, qty = 1))?;
        rt::def!(ns, total(slf as "self"))?;
        Ok(())
    }

    // inventory.py:5
    #[rt::frame(m, "Item.__init__", 5)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        price: &Bound<'py, PyAny>,
        qty: &Bound<'py, PyAny>,
    ) -> PyResult<()> {
        slf.set_attr("name", name)?;
        slf.set_attr("price", price)?;
        slf.set_attr("qty", qty)?;
        Ok(())
    }

    // inventory.py:10
    #[rt::frame(m, "Item.total", 10)]
    pub fn total<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("price")?.mul(slf.attr("qty")?)
    }
}
```

The module body creates the class from that `body` function, the way Python executes a `class` statement:

```rust
m.set_global(
    "Item",
    rt::class!(
        m,
        "Item"(),
        item::body,
        line = 4,
        attributes = ["name", "price", "qty"]
    )?,
)?;
```

`cargo build` in `out/` produces an executable that embeds libpython and prints `6`, like `python inventory.py`. `Item` is still a real Python class whose methods are native functions, and `load` is a real Python function (`inspect.signature`, pickling and monkeypatching still work).

## Installation

steelix needs Python 3.14 and a Rust toolchain.

```sh
pip install steelix
# or
uv tool install steelix
```

The generated crates depend on the [`steelix-runtime`](https://crates.io/crates/steelix-runtime) crate from crates.io.

## Usage

```sh
steelix transpile app.py -o out/               # a crate for app.py and the project modules it imports
steelix transpile -m pkg --root src/ -o out/   # the same for `python -m pkg`
steelix build app.py                           # transpile, build and assemble a relocatable dist/app/
steelix test out/ -- tests/                    # run the project's pytest suite against the native modules
steelix coverage app.py                        # how much of the code becomes Rust
steelix attrs app.py --class Point             # who reads and writes the attributes of a class
```

When building a generated crate yourself, point PyO3 at the interpreter whose packages the program uses: `PYO3_PYTHON=/path/to/venv/bin/python cargo build` (`\path\to\venv\Scripts\python.exe` on Windows).

`steelix build` bundles the Python runtime and the site-packages of the build environment next to the executable, so the result runs on a machine without Python (Linux, glibc at least as new as the build machine's).

## Rewriting

Each generated function keeps its Python calling convention, so it can be rewritten on its own: keep the signature as an adapter, move the logic into plain Rust, convert the arguments at the boundary. Python containers can stay as they are at first. [`examples/logsum`](examples/logsum) shows a small CLI before and after rewriting some of its functions.

- `steelix test out/ -- tests/` checks each step with the project's own tests, running pytest with the generated executable as the interpreter. It fails if a transpiled module was imported from source.
- `steelix attrs app.py` shows which functions, in which modules, touch the attributes of a class, before it becomes a `#[pyclass]`; `--pyclass NAME` prints a draft.
- `--mode ext` builds a PyO3 extension module instead of an executable, for publishing a transpiled library.

## What is missing

- Async generators, async comprehensions, `match` OR patterns with captures, custom metaclasses and a few rarer constructs are kept as Python: the statement stays Python source inside the generated Rust, in the same module globals, so any program CPython runs can be transpiled. `steelix coverage` reports what was kept and why.
- Relocatable distributions (`steelix build`) only for Linux. On macOS and Windows the generated crates build and run on the machine they were built on.
- No free-threaded Python.

## Status

Early, but tested against CPython: every test case, six command line tools of the standard library, the standard library `json` package and a small Scheme interpreter run with the same output and the same tracebacks as under CPython, and 154 standard library modules transpile into crates that compile without warnings.

## License

BSD 3-Clause, see [LICENSE](LICENSE).
