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
pub fn load<'py>(m: &rt::Module<'py>, text: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "load", 14, |frame| {
        let rows = match (|| m.global("json")?.method1("loads", (text,)))() {
            Ok(value) => value,
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                rt::builtins::print(
                    py,
                    (rt::fstring(py, ("bad input: ", rt::format(&e, "")?))?,),
                )?;
                return Ok(PyList::empty(py).into_any());
            }
            Err(err) => return Err(err),
        };
        let list = PyList::empty(py);
        for row in rows.try_iter()? {
            let row = row?;
            if row.get_item("qty")?.gt(0)? {
                list.append(rt::call!(m.global("Item")?, **row)?)?;
            }
            py.check_signals()?;
        }
        Ok(list.into_any())
    })
}

pub mod item {
    use super::*;

    // inventory.py:5
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        name: &Bound<'py, PyAny>,
        price: &Bound<'py, PyAny>,
        qty: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.__init__", 5, |_| {
            slf.set_attr("name", name)?;
            slf.set_attr("price", price)?;
            slf.set_attr("qty", qty)?;
            Ok(rt::none(m.py()))
        })
    }

    // inventory.py:10
    pub fn total<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Item.total", 10, |_| {
            slf.attr("price")?.mul(slf.attr("qty")?)
        })
    }
}
```

`cargo build` in `out/` produces an executable that embeds libpython and prints `6`, like `python inventory.py`. The class is a real Python class, `load` is a real Python function (`inspect.signature`, pickling and monkeypatching still work), and the generator expression is evaluated lazily as in CPython.

## Usage

steelix needs Python 3.14 and a Rust toolchain. With [uv](https://docs.astral.sh/uv/):

```sh
uv run steelix transpile app.py -o out/        # a crate for app.py and the project modules it imports
uv run steelix transpile -m pkg --root src/ -o out/   # the same for `python -m pkg`
uv run steelix build app.py                    # transpile, build and assemble a relocatable dist/app/
uv run steelix coverage app.py                 # how much of the code becomes Rust
```

When building a generated crate yourself, point PyO3 at the interpreter whose packages the program uses: `PYO3_PYTHON=/path/to/venv/bin/python cargo build`.

`steelix build` bundles the Python runtime and the site-packages of the build environment next to the executable, so the result runs on a machine without Python (Linux, glibc at least as new as the build machine's).

## Rewriting

Each generated function keeps its Python calling convention, so it can be rewritten on its own: keep the signature as an adapter, move the logic into plain Rust, convert the arguments at the boundary. The rest of the program, including Python code that calls the function, does not notice. [`examples/logsum`](examples/logsum) shows a small CLI before and after rewriting some of its functions.

Python containers can stay as they are at first (a `PyDict` is fine in Rust code), so the first step only changes the logic, not the data model.

## What is missing

- **async/await.** A coroutine would have to become a hand written state machine, which is the opposite of readable. Async functions are kept as Python.
- **Generators** (`yield`) are kept as Python for the same reason. Generator expressions are supported.
- **`match`** statements and a few rarer constructs (custom metaclasses, `exec`/`eval` without a namespace, ...) are kept as Python.
- **Executables for Windows and macOS** are not supported yet, only Linux.
- **Free-threaded Python** is not supported.

"Kept as Python" means the statement or function stays Python source inside the generated Rust and runs in the same module globals, so any program CPython runs can be transpiled. `steelix transpile` and `steelix coverage` report what was kept and why.

## Status

Early, but tested against CPython: every test case, six command line tools of the standard library, the standard library `json` package and a small Scheme interpreter run with the same output and the same tracebacks as under CPython, and 154 standard library modules transpile into crates that compile without warnings.

## License

BSD 3-Clause, see [LICENSE](LICENSE).
