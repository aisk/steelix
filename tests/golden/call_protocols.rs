use pyo3::exceptions::PyRecursionError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// call_protocols.py:9
#[rt::frame(m, "show", 9, frame)]
pub fn show<'py>(
    m: &rt::Module<'py>,
    function: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| rt::builtins::print(py, (rt::call!(function, *args, **kwargs)?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let e = frame.catch(err);
            rt::builtins::print(py, ("TypeError:", &e))?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// call_protocols.py:16
#[rt::frame(m, "pair", 16)]
pub fn pair<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [a, b])?.into_any())
}

// call_protocols.py:20
#[rt::frame(m, "defaults", 20)]
pub fn defaults<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    c: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [a, b, c])?.into_any())
}

// call_protocols.py:24
#[rt::frame(m, "mixed", 24)]
pub fn mixed<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
    args: &Bound<'py, PyTuple>,
    c: &Bound<'py, PyAny>,
    d: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [a, b, args, c, d, kwargs])?.into_any())
}

// call_protocols.py:28
#[rt::frame(m, "posonly_kwargs", 28)]
pub fn posonly_kwargs<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    kwargs: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyTuple::new(m.py(), [a, kwargs])?.into_any())
}

// call_protocols.py:32
pub fn star<'py>(m: &rt::Module<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
    Ok(args.clone().into_any())
}

// Recursion through Python calls ends in `RecursionError`.
// call_protocols.py:136
#[rt::frame(m, "through", 136)]
pub fn through<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
    f: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    f.call1((n.add(1)?, f))
}

// call_protocols.py:153
#[rt::frame(m, "catch", 153, frame)]
pub fn catch<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| -> PyResult<()> {
        Err(frame.raise(PyValueError::new_err((m
            .global("Noisy")?
            .call0()?
            .unbind(),))))
    })() {
        Ok(()) => unreachable!(),
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("caught",))?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(py, ("after",))?;
    Ok(rt::none(py))
}

// Calls with keyword arguments (no `**`) through objects: the arguments are
// evaluated in order, the callee checks the keywords.
// call_protocols.py:168
#[rt::frame(m, "traced", 168)]
pub fn traced<'py>(
    m: &rt::Module<'py>,
    label: &Bound<'py, PyAny>,
    value: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("evaluating", label))?;
    Ok(value.clone())
}

// call_protocols.py:173
#[rt::frame(m, "keywords", 173, frame)]
pub fn keywords<'py>(
    m: &rt::Module<'py>,
    callee: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    match (|| {
        rt::call!(
            callee,
            traced(m, &rt::str(py, "x"), &rt::int(py, 1))?,
            sep = traced(m, &rt::str(py, "sep"), &rt::str(py, "-"))?,
            end = traced(m, &rt::str(py, "end"), &rt::str(py, "!"))?
        )
    })() {
        Ok(value) => Ok(value),
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let err = frame.catch(err);
            rt::fstring(py, ("TypeError: ", rt::format(&err, "")?))
        }
        Err(err2) => Err(err2),
    }
}

// call_protocols.py:180
#[rt::frame(m, "joined", 180)]
pub fn joined<'py>(
    m: &rt::Module<'py>,
    args: &Bound<'py, PyTuple>,
    sep: &Bound<'py, PyAny>,
    end: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    sep.attr("join")?
        .call1((rt::builtins::map(py, (rt::builtin(py, "str")?, args))?,))?
        .add(end)
}

pub mod box_ {
    use super::*;

    // call_protocols.py:81
    /// The body of `class Box`, run in its namespace `ns`.
    #[rt::frame(ns, "Box", 81)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __init__(slf as "self", value, extra = 0))?;
        rt::def!(ns, get(slf as "self"))?;
        rt::def!(ns, add(slf as "self", n, times = 1))?;
        rt::def!(ns, @classmethod make(cls, value))?;
        rt::def!(ns, @staticmethod twice(x))?;
        Ok(())
    }

    // call_protocols.py:82
    #[rt::frame(m, "Box.__init__", 82)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
        extra: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("value", value.add(extra)?)?;
        Ok(rt::none(m.py()))
    }

    // call_protocols.py:85
    #[rt::frame(m, "Box.get", 85)]
    pub fn get<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("value")
    }

    // call_protocols.py:88
    #[rt::frame(m, "Box.add", 88)]
    pub fn add<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        n: &Bound<'py, PyAny>,
        times: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.attr("value")?.add(n.mul(times)?)
    }

    // call_protocols.py:91
    #[rt::frame(m, "Box.make", 91)]
    pub fn make<'py>(
        m: &rt::Module<'py>,
        cls: &Bound<'py, PyAny>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        cls.call1((value,))
    }

    // call_protocols.py:95
    #[rt::frame(m, "Box.twice", 95)]
    pub fn twice<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        rt::int(m.py(), 2).mul(x)
    }
}

pub mod child {
    use super::*;

    // call_protocols.py:100
    /// The body of `class Child`, run in its namespace `ns`.
    #[rt::frame(ns, "Child", 100)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let __class__ = ns.class_cell();
        rt::def!(ns, get[&__class__](slf as "self"))?;
        Ok(())
    }

    // call_protocols.py:101
    #[rt::frame(m, "Child.get", 101)]
    pub fn get<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::super_(__class__, slf)?.method0("get")?.add(100)
    }
}

pub mod with_partial {
    use super::*;

    // call_protocols.py:127
    /// The body of `class WithPartial`, run in its namespace `ns`.
    #[rt::frame(ns, "WithPartial", 127)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("value", 10)?;
        ns.set(
            "add",
            rt::call!(
                ns.get("functools")?.attr("partialmethod")?,
                ns.get("Box")?.attr("add")?,
                times = 3
            )?,
        )?;
        Ok(())
    }
}

pub mod noisy {
    use super::*;

    // call_protocols.py:148
    /// The body of `class Noisy`, run in its namespace `ns`.
    #[rt::frame(ns, "Noisy", 148)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __del__(slf as "self"))?;
        Ok(())
    }

    // call_protocols.py:149
    #[rt::frame(m, "Noisy.__del__", 149)]
    pub fn __del__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::builtins::print(py, ("deleted",))?;
        Ok(rt::none(py))
    }
}

/// Executes the body of `call_protocols.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Calls of transpiled functions through every CPython call path (vectorcall,\n\
            `tp_call`, bound methods, `functools.partial`), with and without the fast\n\
            paths for plain positional calls.",
    )?;
    m.set_global("functools", rt::import(m, "functools")?)?;
    m.set_global("threading", rt::import(m, "threading")?)?;
    rt::def!(m, show(function, /, *args, **kwargs))?;
    rt::def!(m, pair(a, b))?;
    rt::def!(m, defaults(a, b = 2, c = 3))?;
    rt::def!(m, mixed(a, /, b, *args, c, d = 4, **kwargs))?;
    rt::def!(m, posonly_kwargs(a, /, **kwargs))?;
    rt::def!(m, star(*args))?;
    // Plain positional calls, which need no binding.
    show(
        m,
        &m.global("pair")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("pair")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("pair")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("pair")?,
        &PyTuple::empty(py),
        &[("b", 2), ("a", 1)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("pair")?,
        &PyTuple::new(py, [1])?,
        &[("a", 2)].into_py_dict(py)?,
    )?;
    // Missing arguments filled from `__defaults__`, read at call time.
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1, 5])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1, 5, 6])?,
        &PyDict::new(py),
    )?;
    let value = PyTuple::new(py, [7])?;
    m.global("defaults")?.set_attr("__defaults__", value)?;
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    let value = PyTuple::new(py, [8, 9, 10, 11])?;
    m.global("defaults")?.set_attr("__defaults__", value)?;
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    m.global("defaults")?.set_attr("__defaults__", py.None())?;
    show(
        m,
        &m.global("defaults")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    let value = PyTuple::new(py, [2, 3])?;
    m.global("defaults")?.set_attr("__defaults__", value)?;
    // Keywords, `*args` and `**kwargs`.
    show(
        m,
        &m.global("mixed")?,
        &PyTuple::new(py, [1, 2])?,
        &[("c", 3)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("mixed")?,
        &PyTuple::new(py, [1, 2, 3, 4])?,
        &[("c", 5), ("d", 6), ("e", 7), ("f", 8)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("mixed")?,
        &PyTuple::new(py, [1])?,
        &[("b", 2), ("c", 3), ("z", 0)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("mixed")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("mixed")?,
        &PyTuple::empty(py),
        &[("a", 1), ("b", 2), ("c", 3)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("posonly_kwargs")?,
        &PyTuple::new(py, [1])?,
        &[("a", 2)].into_py_dict(py)?,
    )?;
    // call_protocols.py:63
    #[rt::frame(m, "<lambda>", 63)]
    fn lambda<'py>(
        m: &rt::Module<'py>,
        a: &Bound<'py, PyTuple>,
        k: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(PyTuple::new(m.py(), [a.as_any(), k.as_any()])?.into_any())
    }
    show(
        m,
        &rt::function!(m, lambda(*a, **k), qualname = "<lambda>")?,
        &PyTuple::new(py, [1])?,
        &[("x", 2)].into_py_dict(py)?,
    )?;
    m.set_global("t", PyTuple::new(py, [1, 2])?)?;
    rt::builtins::print(
        py,
        (
            rt::call!(m.global("star")?, *m.global("t")?)?
                .rich_compare(m.global("t")?, CompareOp::Eq)?,
            rt::call!(m.global("star")?, *m.global("t")?)?.is(m.global("t")?),
            star(m, &PyTuple::empty(py))?.rich_compare(PyTuple::empty(py), CompareOp::Eq)?,
        ),
    )?;
    rt::call!(
        m.global("show")?,
        m.global("pair")?,
        **[("a", 1), ("b", 2)].into_py_dict(py)?
    )?;
    match (|| rt::call!(m.global("pair")?, 1, **[(2, 3)].into_py_dict(py)?))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exception = frame.catch(err);
            m.set_global("e", &exception)?;
            rt::builtins::print(py, ("TypeError:", m.global("e")?))?;
            m.unbind_global("e")?;
        }
        Err(err) => return Err(err),
    }
    // `tp_call`: `f.__call__` and `type(f).__call__` bind the same way.
    show(
        m,
        &m.global("pair")?.attr("__call__")?,
        &PyTuple::new(py, [1, 2])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("pair")?.attr("__call__")?,
        &PyTuple::new(py, [1])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("pair")?.get_type().attr("__call__")?,
        &PyTuple::new(py, [&m.global("pair")?, &rt::int(py, 1)])?,
        &[("b", 2)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("defaults")?.attr("__call__")?,
        &PyTuple::new(py, [1])?,
        &[("c", 5)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("mixed")?.attr("__call__")?,
        &PyTuple::new(py, [1, 2])?,
        &[("c", 3), ("e", 4)].into_py_dict(py)?,
    )?;
    show(
        m,
        &m.global("mixed")?.attr("__call__")?,
        &PyTuple::new(py, [1, 2])?,
        &[("b", 3), ("c", 4)].into_py_dict(py)?,
    )?;
    m.set_global(
        "Box",
        rt::class!(m, "Box"(), box_::body, line = 81, attributes = ["value"])?,
    )?;
    m.set_global(
        "Child",
        rt::class!(m, "Child"(m.global("Box")?), child::body, line = 100)?,
    )?;
    m.set_global("box", m.global("Box")?.call1((1,))?)?;
    rt::builtins::print(
        py,
        (
            m.global("box")?.method0("get")?,
            m.global("box")?.method1("add", (2,))?,
            m.global("box")?.method1("add", (2, 3))?,
            rt::call!(m.global("box")?.attr("add")?, n = 2, times = 4)?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Box")?.attr("get")?.call1((m.global("box")?,))?,
            m.global("Box")?.attr("add")?.call1((m.global("box")?, 5))?,
            rt::call!(m.global("Box")?, 1, extra = 2)?.method0("get")?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("box")?.attr("get")?.get_type().attr("__name__")?,
            m.global("box")?
                .attr("get")?
                .attr("__self__")?
                .is(m.global("box")?),
            m.global("box")?
                .attr("get")?
                .attr("__func__")?
                .is(m.global("Box")?.attr("__dict__")?.get_item("get")?),
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("box")?
                .attr("get")?
                .rich_compare(m.global("box")?.attr("get")?, CompareOp::Eq)?,
            m.global("box")?
                .attr("get")?
                .is(m.global("box")?.attr("get")?),
            m.global("Box")?
                .attr("get")?
                .is(m.global("Box")?.attr("__dict__")?.get_item("get")?),
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Box")?.method1("make", (5,))?.method0("get")?,
            m.global("box")?.method1("make", (6,))?.method0("get")?,
            m.global("Box")?.method1("twice", (4,))?,
            m.global("box")?.method1("twice", (5,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("Child")?.call1((1,))?.method0("get")?,
            m.global("Child")?.method1("make", (2,))?.method0("get")?,
        ),
    )?;
    show(
        m,
        &m.global("box")?.attr("add")?,
        &PyTuple::empty(py),
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("box")?.attr("add")?,
        &PyTuple::new(py, [1, 2, 3])?,
        &PyDict::new(py),
    )?;
    show(
        m,
        &m.global("Box")?.attr("get")?,
        &PyTuple::empty(py),
        &PyDict::new(py),
    )?;
    m.set_global("getter", m.global("box")?.attr("get")?)?;
    m.global("box")?.set_attr("value", 42)?;
    rt::builtins::print(
        py,
        (
            m.global("getter")?.call0()?,
            rt::builtins::getattr(py, (m.global("box")?, "add"))?.call1((1,))?,
        ),
    )?;
    // A function stored on the instance is not bound.
    let pair2 = m.global("pair")?;
    m.global("box")?.set_attr("pair", pair2)?;
    rt::builtins::print(py, (m.global("box")?.method1("pair", (1, 2))?,))?;
    // `functools.partial` and `partialmethod`.
    rt::builtins::print(
        py,
        (
            m.global("functools")?
                .attr("partial")?
                .call1((m.global("pair")?, 1))?
                .call1((2,))?,
            rt::call!(
                m.global("functools")?.attr("partial")?,
                m.global("defaults")?,
                c = 0
            )?
            .call1((1,))?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            m.global("functools")?
                .attr("partial")?
                .call1((m.global("Box")?.attr("add")?, m.global("box")?))?
                .call1((1,))?,
            rt::call!(
                m.global("functools")?.attr("partial")?,
                m.global("box")?.attr("add")?,
                times = 2
            )?
            .call1((1,))?,
        ),
    )?;
    m.set_global(
        "WithPartial",
        rt::class!(m, "WithPartial"(), with_partial::body, line = 127)?,
    )?;
    rt::builtins::print(
        py,
        (m.global("WithPartial")?.call0()?.method1("add", (1,))?,),
    )?;
    rt::def!(m, through(n, f))?;
    match (|| through(m, &rt::int(py, 0), &m.global("through")?))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyRecursionError>(py) => {
            let _caught = frame.catch(err);
            rt::builtins::print(py, ("RecursionError",))?;
        }
        Err(err) => return Err(err),
    }
    // An exception caught in a function called from a new thread is released
    // right away, like in CPython.
    m.set_global("Noisy", rt::class!(m, "Noisy"(), noisy::body, line = 148)?)?;
    rt::def!(m, catch())?;
    m.set_global(
        "worker",
        rt::call!(
            m.global("threading")?.attr("Thread")?,
            target = m.global("catch")?
        )?,
    )?;
    m.global("worker")?.method0("start")?;
    m.global("worker")?.method0("join")?;
    rt::def!(m, traced(label, value))?;
    rt::def!(m, keywords(callee))?;
    rt::def!(m, joined(*args, sep, end))?;
    rt::builtins::print(py, (keywords(m, &m.global("joined")?)?,))?;
    rt::builtins::print(py, (keywords(m, &rt::builtin(py, "print")?)?,))?;
    rt::builtins::print(py, (keywords(m, &rt::builtin(py, "int")?)?,))?;
    rt::builtins::print(py, (keywords(m, &rt::int(py, 42))?,))?;
    // call_protocols.py:188
    #[rt::frame(m, "<lambda>", 188)]
    fn lambda2<'py>(
        m: &rt::Module<'py>,
        x: &Bound<'py, PyAny>,
        kw: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtins::sorted(m.py(), (kw.method0("items")?,))
    }
    rt::builtins::print(
        py,
        (keywords(
            m,
            &rt::function!(m, lambda2(x, **kw), qualname = "<lambda>")?,
        )?,),
    )?;
    rt::builtins::print(
        py,
        (keywords(
            m,
            &m.global("functools")?
                .attr("partial")?
                .call1((m.global("joined")?, 0))?,
        )?,),
    )?;
    rt::builtins::print(
        py,
        (keywords(
            m,
            &rt::call!(
                m.global("functools")?.attr("partial")?,
                m.global("joined")?,
                sep = "+"
            )?,
        )?,),
    )?;
    Ok(())
}
