# steelix

Transpile Python into readable Rust code built on [PyO3](https://pyo3.rs).

The generated code still calls into libpython, so the standard library and third-party Python packages keep working. The main target is applications: `steelix transpile app.py -o out/` produces a Cargo crate that builds into an executable embedding libpython and behaves like `python app.py`. Constructs steelix can not lower yet (classes, exceptions, generators, ...) stay Python source inside the generated code and run in the module's globals, so any program CPython runs can be transpiled; `steelix transpile` prints how much of it became native Rust. From there the generated code can be rewritten into idiomatic Rust function by function.

Status: early proof of concept. See [docs/design.md](docs/design.md), [docs/roadmap.md](docs/roadmap.md) and [docs/coverage.md](docs/coverage.md).
