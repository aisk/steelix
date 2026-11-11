use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// match_mappings.py:12
#[rt::frame(m, "handle", 12)]
pub fn handle<'py>(
    m: &rt::Module<'py>,
    message: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if let Some([type_, pos]) = rt::match_map(message, ["type", "pos"])?
        && type_.eq("click")?
        && let Some([x, y]) = rt::match_seq(&pos)?
        && let rest = rt::match_rest(message, ["type", "pos"])?
    {
        rt::fstring(
            py,
            (
                "click at ",
                rt::format(&x, "")?,
                ",",
                rt::format(&y, "")?,
                " extra=",
                rt::format(&rest, "")?,
            ),
        )
    } else if let Some([type_, key2]) = rt::match_map(message, ["type", "key"])?
        && type_.eq("key")?
        && let Some([key]) = rt::match_class(&key2, &rt::builtin(py, "str")?, 1, [])?
        && key.len()? == 1
    {
        rt::fstring(py, ("key ", rt::format(&key, "")?))
    } else if let Some([type_, key]) = rt::match_map(message, ["type", "key"])?
        && type_.eq("key")?
    {
        rt::fstring(py, ("special key ", key.repr()?))
    } else if let Some([type_, size]) = rt::match_map(message, ["type", "size"])?
        && type_.eq("resize")?
        && let Some([w, h]) = rt::match_map(&size, ["w", "h"])?
    {
        rt::fstring(
            py,
            ("resize ", rt::format(&w, "")?, "x", rt::format(&h, "")?),
        )
    } else if rt::match_mapping(message, 1)?
        && let key2 = m.global("Key")?.attr("ID")?
        && let Some([ident]) = rt::match_keys(message, (&key2,))?
        && let rest = rt::match_rest(message, (&key2,))?
        && rest.is_truthy()?
    {
        rt::fstring(
            py,
            (
                "id ",
                rt::format(&ident, "")?,
                " with ",
                rt::format(&rt::builtins::sorted(py, (&rest,))?, "")?,
            ),
        )
    } else if rt::match_mapping(message, 1)?
        && let Some([ident]) = rt::match_keys(message, (m.global("Key")?.attr("ID")?,))?
    {
        rt::fstring(py, ("just id ", rt::format(&ident, "")?))
    } else if let Some([kind]) = rt::match_map(message, ["type"])?
        && let others = rt::match_rest(message, ["type"])?
    {
        rt::fstring(
            py,
            (
                "unknown ",
                rt::format(&kind, "")?,
                " (",
                rt::format(&rt::int(py, others.len()?), "")?,
                " more)",
            ),
        )
    } else if let Some([one, _, nothing, raw]) =
        rt::match_map(message, (1, 2.5, py.None(), rt::bytes(py, b"raw")))?
    {
        rt::fstring(
            py,
            (
                "odd keys ",
                rt::format(&one, "")?,
                " ",
                rt::format(&nothing, "")?,
                " ",
                rt::format(&raw, "")?,
            ),
        )
    } else if rt::match_mapping(message, 0)? {
        Ok(rt::str(py, "some other mapping"))
    } else {
        Ok(rt::str(py, "not a mapping"))
    }
}

// match_mappings.py:57
#[rt::frame(m, "counts", 57)]
pub fn counts<'py>(
    m: &rt::Module<'py>,
    mapping: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    if let Some([a]) = rt::match_map(mapping, ["a"])?
        && a.eq(1)?
        && let rest = rt::match_rest(mapping, ["a"])?
    {
        rt::builtins::print(
            py,
            (
                "a is 1, rest",
                rt::builtins::dict(py, (rt::builtins::sorted(py, (rest.method0("items")?,))?,))?,
                rest.get_type().attr("__name__")?,
            ),
        )?;
    } else if rt::match_mapping(mapping, 0)?
        && let everything = rt::match_rest(mapping, ())?
    {
        rt::builtins::print(
            py,
            (
                "everything",
                rt::builtins::dict(
                    py,
                    (rt::builtins::sorted(py, (everything.method0("items")?,))?,),
                )?,
                everything.is(mapping),
            ),
        )?;
    }
    Ok(rt::none(py))
}

pub mod key {
    use super::*;

    // match_mappings.py:8
    /// The body of `class Key`, run in its namespace `ns`.
    #[rt::frame(ns, "Key", 8)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("ID", "id")?;
        Ok(())
    }
}

/// Executes the body of `match_mappings.py` in the module `m`.
#[rt::frame(m, "<module>", 1)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    // Mapping patterns (design §8): JSON-style messages, `**rest`, value
    // patterns as keys, and mappings that are not dicts.
    m.set_global("collections", rt::import(m, "collections")?)?;
    m.set_global("enum", rt::import(m, "enum")?)?;
    m.set_global(
        "Key",
        rt::class!(
            m,
            "Key"(m.global("enum")?.attr("Enum")?),
            key::body,
            line = 8
        )?,
    )?;
    rt::def!(m, handle(message))?;
    m.set_global(
        "messages",
        PyList::new(
            py,
            [
                [
                    ("type", &rt::str(py, "click")),
                    ("pos", PyList::new(py, [1, 2])?.as_any()),
                ]
                .into_py_dict(py)?
                .as_any(),
                [
                    ("type", &rt::str(py, "click")),
                    ("pos", PyTuple::new(py, [3, 4])?.as_any()),
                    ("button", &rt::str(py, "left")),
                    ("count", &rt::int(py, 2)),
                ]
                .into_py_dict(py)?
                .as_any(),
                [("type", "click"), ("pos", "xy")]
                    .into_py_dict(py)?
                    .as_any(),
                [("type", "key"), ("key", "a")].into_py_dict(py)?.as_any(),
                [("type", "key"), ("key", "Enter")]
                    .into_py_dict(py)?
                    .as_any(),
                [("type", &rt::str(py, "key")), ("key", &rt::int(py, 13))]
                    .into_py_dict(py)?
                    .as_any(),
                [
                    ("type", &rt::str(py, "resize")),
                    (
                        "size",
                        [
                            ("w", &rt::int(py, 80)),
                            ("h", &rt::int(py, 24)),
                            ("unit", &rt::str(py, "chars")),
                        ]
                        .into_py_dict(py)?
                        .as_any(),
                    ),
                ]
                .into_py_dict(py)?
                .as_any(),
                [
                    (&m.global("Key")?.attr("ID")?, &rt::int(py, 7)),
                    (&rt::str(py, "name"), &rt::str(py, "x")),
                    (&rt::str(py, "kind"), &rt::str(py, "y")),
                ]
                .into_py_dict(py)?
                .as_any(),
                [(&m.global("Key")?.attr("ID")?, 8)]
                    .into_py_dict(py)?
                    .as_any(),
                [("type", &rt::str(py, "scroll")), ("dy", &rt::int(py, 3))]
                    .into_py_dict(py)?
                    .as_any(),
                [
                    (&rt::int(py, 1), "one"),
                    (&rt::float(py, 2.5), "two"),
                    (&rt::none(py), "none"),
                    (&rt::bytes(py, b"raw"), "raw"),
                ]
                .into_py_dict(py)?
                .as_any(),
                [("other", 1)].into_py_dict(py)?.as_any(),
                PyDict::new(py).as_any(),
                PyList::new(py, ["type", "click"])?.as_any(),
                &rt::str(py, "text"),
            ],
        )?,
    )?;
    for item in rt::iter(&m.global("messages")?)? {
        m.set_global("message", item?)?;
        rt::builtins::print(py, (handle(m, &m.global("message")?)?,))?;
    }
    rt::def!(m, counts(mapping))?;
    counts(m, [("a", 1), ("b", 2)].into_py_dict(py)?.as_any())?;
    counts(m, [("a", 2), ("b", 3)].into_py_dict(py)?.as_any())?;
    counts(
        m,
        &rt::call!(m.global("collections")?.attr("OrderedDict")?, z = 1, y = 2)?,
    )?;
    // A `defaultdict` is looked up with `.get()`: no key is created.
    m.set_global(
        "d",
        m.global("collections")?
            .attr("defaultdict")?
            .call1((rt::builtin(py, "list")?,))?,
    )?;
    {
        let subject = m.global("d")?;
        if let Some([value]) = rt::match_map(&subject, ["missing"])? {
            m.set_global("value", &value)?;
            rt::builtins::print(py, ("found", m.global("value")?))?;
        } else {
            rt::builtins::print(
                py,
                (
                    "not found, keys",
                    rt::builtins::list(py, (m.global("d")?,))?,
                ),
            )?;
        }
    }
    // `**rest` at module level binds a global.
    {
        let subject = [
            ("name", &rt::str(py, "steelix")),
            ("version", &rt::int(py, 1)),
            ("license", &rt::str(py, "MIT")),
        ]
        .into_py_dict(py)?
        .into_any();
        if let Some([name]) = rt::match_map(&subject, ["name"])?
            && let meta = rt::match_rest(&subject, ["name"])?
        {
            m.set_global("name", &name)?;
            m.set_global("meta", &meta)?;
            rt::builtins::print(
                py,
                (
                    m.global("name")?,
                    rt::builtins::sorted(py, (m.global("meta")?,))?,
                ),
            )?;
        }
    }
    rt::builtins::print(py, (m.global("meta")?.get_item("license")?,))?;
    Ok(())
}
