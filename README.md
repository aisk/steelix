# steelix

Transpile Python into readable Rust code built on [PyO3](https://pyo3.rs).

The generated code still calls into libpython, so the standard library and third-party Python packages keep working. The main target is applications: `steelix transpile app.py -o out/` produces a Cargo crate that builds into an executable embedding libpython and behaves like `python app.py`. From there the generated code can be rewritten into idiomatic Rust function by function.

Status: early proof of concept. See [docs/design.md](docs/design.md) and [docs/roadmap.md](docs/roadmap.md).
