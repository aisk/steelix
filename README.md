# steelix

Transpile Python into readable Rust code built on [PyO3](https://pyo3.rs).

The generated code still calls into libpython, so the standard library and third-party Python packages keep working. The idea is to get a working Rust crate first, then rewrite it into idiomatic Rust function by function.

Status: early proof of concept. See [docs/design.md](docs/design.md) and [docs/roadmap.md](docs/roadmap.md).
