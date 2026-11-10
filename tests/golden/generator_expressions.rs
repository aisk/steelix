use pyo3::exceptions::PyNameError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyZeroDivisionError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::PyIterator;
use pyo3::types::PyList;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// generator_expressions.py:11
#[rt::frame(m, "show", 11)]
pub fn show<'py>(m: &rt::Module<'py>, exc: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, (exc.get_type().attr("__name__")?, exc))?;
    for frame_ in rt::iter(
        &m.global("traceback")?
            .attr("extract_tb")?
            .call1((exc.attr("__traceback__")?,))?,
    )? {
        let frame_ = frame_?;
        rt::builtins::print(
            py,
            (
                " ",
                frame_.attr("name")?,
                frame_.attr("lineno")?,
                frame_.attr("line")?,
            ),
        )?;
    }
    Ok(rt::none(py))
}

// generator_expressions.py:17
#[rt::frame(m, "noisy", 17)]
pub fn noisy<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::builtins::print(m.py(), ("check", x))?;
    x.rich_compare(2, CompareOp::Gt)
}

// generator_expressions.py:22
#[rt::frame(m, "short_circuit", 22, frame)]
pub fn short_circuit<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // generator_expressions.py:23
    #[rt::frame(m, "short_circuit.<locals>.<genexpr>", 23)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        xs: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(xs)? {
            return Ok(Some(noisy(m, &x)?));
        }
        Ok(None)
    }
    rt::builtin(m.py(), "any")?.call1((rt::genexpr!(frame, genexpr(xs.try_iter()?))?,))
}

// generator_expressions.py:26
#[rt::frame(m, "infinite", 26, frame)]
pub fn infinite<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // generator_expressions.py:27
    #[rt::frame(m, "infinite.<locals>.<genexpr>", 27)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(n) = rt::next(iter)? {
            return Ok(Some(n.mul(&n)?));
        }
        Ok(None)
    }
    let r#gen = rt::genexpr!(
        frame,
        genexpr(m.global("itertools")?.method0("count")?.try_iter()?)
    )?;
    let first = rt::builtin(py, "next")?.call1((&r#gen,))?;
    let rest = rt::builtin(py, "list")?
        .call1((m.global("itertools")?.method1("islice", (&r#gen, 3))?,))?;
    Ok(PyTuple::new(py, [&first, &rest])?.into_any())
}

// generator_expressions.py:33
#[rt::frame(m, "late_binding", 33, frame)]
pub fn late_binding<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let factor = rt::Cell::new(py, "factor", 1)?;
    // generator_expressions.py:35
    #[rt::frame(m, "late_binding.<locals>.<genexpr>", 35)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        factor: &rt::Cell<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(iter)? {
            return Ok(Some(x.mul(factor.get()?)?));
        }
        Ok(None)
    }
    let r#gen = rt::genexpr!(
        frame,
        genexpr[&factor](rt::builtin(py, "range")?.call1((3,))?.try_iter()?)
    )?;
    factor.set(10)?;
    rt::builtin(py, "list")?.call1((&r#gen,))
}

// generator_expressions.py:40
#[rt::frame(m, "first_iterable_now", 40, frame)]
pub fn first_iterable_now<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("creating",))?;
    // generator_expressions.py:42
    #[rt::frame(m, "first_iterable_now.<locals>.<genexpr>", 42)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(iter)? {
            return Ok(Some(x));
        }
        Ok(None)
    }
    let r#gen = rt::genexpr!(frame, genexpr(noisy_iterable(m)?.try_iter()?))?;
    rt::builtins::print(py, ("created",))?;
    rt::builtin(py, "list")?.call1((&r#gen,))
}

// generator_expressions.py:47
#[rt::frame(m, "noisy_iterable", 47)]
pub fn noisy_iterable<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::builtins::print(py, ("first iterable evaluated",))?;
    Ok(PyList::new(py, [1, 2])?.into_any())
}

// generator_expressions.py:52
#[rt::frame(m, "failing", 52, frame)]
pub fn failing<'py>(m: &rt::Module<'py>, xs: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // generator_expressions.py:53
    #[rt::frame(m, "failing.<locals>.<genexpr>", 53)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        xs: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(xs)? {
            return Ok(Some(rt::int(m.py(), 10).floor_div(&x)?));
        }
        Ok(None)
    }
    let r#gen = rt::genexpr!(frame, genexpr(xs.try_iter()?))?;
    rt::builtins::print(py, ("created",))?;
    let out = PyList::empty(py).into_any();
    for value in rt::iter(&r#gen)? {
        let value = value?;
        out.method1("append", (&value,))?;
    }
    Ok(out)
}

// generator_expressions.py:69
#[rt::frame(m, "failing_iteration", 69, frame)]
pub fn failing_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    // generator_expressions.py:70
    #[rt::frame(m, "failing_iteration.<locals>.<genexpr>", 70)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(iter)? {
            return Ok(Some(x));
        }
        Ok(None)
    }
    let r#gen = {
        let iter = m.global("Bad")?.call0()?.try_iter()?;
        rt::genexpr!(frame, genexpr(iter))?
    };
    rt::builtin(m.py(), "sum")?.call1((&r#gen,))
}

// generator_expressions.py:75
#[rt::frame(m, "failing_condition", 75, frame)]
pub fn failing_condition<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // generator_expressions.py:76
    #[rt::frame(m, "failing_condition.<locals>.<genexpr>", 76)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        xs: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        for x in rt::iter(xs)? {
            let x = x?;
            if rt::int(m.py(), 10).floor_div(&x)?.is_truthy()? {
                return Ok(Some(x));
            }
        }
        Ok(None)
    }
    rt::builtin(m.py(), "list")?.call1((rt::genexpr!(frame, genexpr(xs.try_iter()?))?,))
}

// generator_expressions.py:79
#[rt::frame(m, "closed_early", 79, frame)]
pub fn closed_early<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // generator_expressions.py:80
    #[rt::frame(m, "closed_early.<locals>.source", 80, frame)]
    async fn source<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let result: PyResult<()> = async {
            rt::yield_(rt::int(py, 1)).await?;
            rt::yield_(rt::int(py, 2)).await?;
            rt::yield_(rt::int(py, 3)).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        rt::builtins::print(py, ("source closed",))?;
        finally.resume()?;
        Ok(rt::none(py))
    }
    let source = rt::function!(frame, gen source())?;
    // generator_expressions.py:88
    #[rt::frame(m, "closed_early.<locals>.<genexpr>", 88)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(iter)? {
            return Ok(Some(x.mul(2)?));
        }
        Ok(None)
    }
    let r#gen = rt::genexpr!(frame, genexpr(source.call0()?.try_iter()?))?;
    for value in rt::iter(&r#gen)? {
        let value = value?;
        rt::builtins::print(py, ("got", &value))?;
        break;
    }
    r#gen.method0("close")?;
    rt::builtins::print(
        py,
        ("after close", rt::builtin(py, "list")?.call1((&r#gen,))?),
    )?;
    // generator_expressions.py:94
    #[rt::frame(m, "closed_early.<locals>.<genexpr>", 94)]
    fn genexpr2<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(iter)? {
            return Ok(Some(x));
        }
        Ok(None)
    }
    let mut other = Some(rt::genexpr!(frame, genexpr2(source.call0()?.try_iter()?))?);
    rt::builtins::print(
        py,
        (rt::builtin(py, "next")?.call1((rt::local(&other, "other")?,))?,),
    )?;
    other = None;
    rt::builtins::print(py, ("deleted",))?;
    Ok(rt::none(py))
}

// generator_expressions.py:100
#[rt::frame(m, "stop_iteration", 100, frame)]
pub fn stop_iteration<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // generator_expressions.py:101
    #[rt::frame(m, "stop_iteration.<locals>.boom", 101, frame)]
    fn boom<'py>(m: &rt::Module<'py>, v: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        if v.eq(2)? {
            return Err(frame.raise(PyStopIteration::new_err("inner")));
        }
        Ok(v.clone())
    }
    let boom = rt::function!(frame, boom(v))?;
    // generator_expressions.py:107
    #[rt::frame(m, "stop_iteration.<locals>.<genexpr>", 107)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        boom: &Bound<'py, PyAny>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(v) = rt::next(iter)? {
            return Ok(Some(boom.call1((&v,))?));
        }
        Ok(None)
    }
    match (|| -> PyResult<Bound<'py, PyAny>> {
        let list = rt::builtin(py, "list")?;
        let iter = rt::builtin(py, "range")?.call1((4,))?.try_iter()?;
        rt::builtins::print(
            py,
            (list.call1((rt::genexpr!(frame, genexpr[boom](iter))?,))?,),
        )
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyRuntimeError>(py) => {
            let exc = frame.catch(err);
            rt::builtins::print(
                py,
                (
                    exc.get_type().attr("__name__")?,
                    &exc,
                    exc.attr("__cause__")?.repr()?,
                    exc.attr("__suppress_context__")?,
                ),
            )?;
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

// generator_expressions.py:112
#[rt::frame(m, "protocol", 112, frame)]
pub fn protocol<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // generator_expressions.py:113
    #[rt::frame(m, "protocol.<locals>.<genexpr>", 113)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(i) = rt::next(iter)? {
            return Ok(Some(i));
        }
        Ok(None)
    }
    let g = rt::genexpr!(
        frame,
        genexpr(rt::builtin(py, "range")?.call1((3,))?.try_iter()?)
    )?;
    rt::builtins::print(
        py,
        (
            g.get_type().attr("__name__")?,
            g.repr()?.method1(
                "startswith",
                ("<generator object protocol.<locals>.<genexpr> at ",),
            )?,
        ),
    )?;
    rt::builtins::print(
        py,
        (
            g.attr("__name__")?,
            g.attr("__qualname__")?,
            g.try_iter()?.is(&g),
            g.attr("gi_running")?,
        ),
    )?;
    match g.method1("send", (1,)) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyTypeError>(py) => {
            let exc = frame.catch(err);
            rt::builtins::print(py, ("TypeError", &exc))?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(
        py,
        (
            g.method1("send", (py.None(),))?,
            rt::builtin(py, "next")?.call1((&g,))?,
        ),
    )?;
    match (|| {
        g.attr("throw")?
            .call1((rt::builtin(py, "ValueError")?.call1(("thrown",))?,))
    })() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let exc = frame.catch(err);
            rt::builtins::print(py, ("thrown back", &exc))?;
        }
        Err(err) => return Err(err),
    }
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "list")?.call1((&g,))?,
            rt::builtin(py, "next")?.call1((&g, "exhausted"))?,
        ),
    )?;
    g.set_attr("__name__", "renamed")?;
    rt::builtins::print(py, (g.attr("__name__")?,))?;
    Ok(rt::none(py))
}

// generator_expressions.py:130
#[rt::frame(m, "consumers", 130, frame)]
pub fn consumers<'py>(
    m: &rt::Module<'py>,
    words: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // generator_expressions.py:131
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 131)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(rt::int(m.py(), w.len()?)));
        }
        Ok(None)
    }
    // generator_expressions.py:131
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 131)]
    fn genexpr2<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(w.method0("upper")?));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "sum")?.call1((rt::genexpr!(frame, genexpr(words.try_iter()?))?,))?,
            rt::str(py, "-")
                .attr("join")?
                .call1((rt::genexpr!(frame, genexpr2(words.try_iter()?))?,))?,
        ),
    )?;
    // generator_expressions.py:132
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 132)]
    fn genexpr3<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(w));
        }
        Ok(None)
    }
    // generator_expressions.py:132
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 132)]
    fn genexpr4<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(w));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            rt::call!(
                rt::builtin(py, "sorted")?,
                rt::genexpr!(frame, genexpr3(words.try_iter()?))?,
                key = rt::builtin(py, "len")?
            )?,
            rt::call!(
                rt::builtin(py, "max")?,
                rt::genexpr!(frame, genexpr4(words.try_iter()?))?,
                key = rt::builtin(py, "len")?
            )?,
        ),
    )?;
    // generator_expressions.py:133
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 133)]
    fn genexpr5<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(w.get_item(0)?));
        }
        Ok(None)
    }
    // generator_expressions.py:133
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 133)]
    fn genexpr6<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let py = m.py();
        if let Some(w) = rt::next(words)? {
            return Ok(Some(
                PyTuple::new(py, [&w, &rt::int(py, w.len()?)])?.into_any(),
            ));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "tuple")?
                .call1((rt::genexpr!(frame, genexpr5(words.try_iter()?))?,))?,
            rt::builtin(py, "dict")?.call1((rt::genexpr!(frame, genexpr6(words.try_iter()?))?,))?,
        ),
    )?;
    // generator_expressions.py:134
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 134)]
    fn genexpr7<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(w));
        }
        Ok(None)
    }
    // generator_expressions.py:134
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 134)]
    fn genexpr8<'py>(
        m: &rt::Module<'py>,
        words: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(w) = rt::next(words)? {
            return Ok(Some(w.get_item(-1)?));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "all")?.call1((rt::genexpr!(frame, genexpr7(words.try_iter()?))?,))?,
            rt::builtin(py, "set")?
                .call1((rt::genexpr!(frame, genexpr8(words.try_iter()?))?,))?
                .rich_compare(PySet::new(py, ["a", "c"])?, CompareOp::Eq)?,
        ),
    )?;
    // generator_expressions.py:135
    #[rt::frame(m, "consumers.<locals>.<genexpr>", 135)]
    fn genexpr9<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(item) = rt::next(iter)? {
            let [a, b] = rt::unpack(&item)?;
            return Ok(Some(PyTuple::new(m.py(), [&a, &b])?.into_any()));
        }
        Ok(None)
    }
    rt::builtins::print(
        py,
        (rt::builtin(py, "list")?.call1((rt::genexpr!(
            frame,
            genexpr9(
                rt::builtin(py, "zip")?
                    .call1((words, rt::builtin(py, "range")?.call1((3,))?))?
                    .try_iter()?
            )
        )?,))?,),
    )?;
    Ok(rt::none(py))
}

// generator_expressions.py:138
#[rt::frame(m, "captures", 138, frame)]
pub fn captures<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let base = rt::int(m.py(), 10);
    // generator_expressions.py:141
    #[rt::frame(m, "captures.<locals>.inner", 141, frame)]
    fn inner<'py>(m: &rt::Module<'py>, base: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        // generator_expressions.py:142
        #[rt::frame(m, "captures.<locals>.inner.<locals>.<genexpr>", 142)]
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            base: &Bound<'py, PyAny>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            if let Some(i) = rt::next(iter)? {
                return Ok(Some(base.add(&i)?));
            }
            Ok(None)
        }
        rt::builtin(py, "list")?.call1((rt::genexpr!(
            frame,
            genexpr[base](rt::builtin(py, "range")?.call1((2,))?.try_iter()?)
        )?,))
    }
    let inner = rt::function!(frame, inner[base]())?;
    inner.call0()
}

// generator_expressions.py:147
#[rt::frame(m, "comprehension_variable", 147, frame)]
pub fn comprehension_variable<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let x: Bound<'py, PyAny>;
    // Each generator sees the comprehension's variable when it runs.
    // generator_expressions.py:149
    #[rt::frame(m, "comprehension_variable.<locals>.<genexpr>", 149)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        x: &rt::Cell<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(underscore) = rt::next(iter)? {
            return Ok(Some(x.get()?));
        }
        Ok(None)
    }
    let gens = {
        let x_ = rt::Cell::empty(py, "x")?;
        let list = PyList::empty(py);
        for item in rt::iter(&rt::builtin(py, "range")?.call1((3,))?)? {
            x_.set(item?)?;
            list.append(rt::genexpr!(
                frame,
                genexpr[&x_ as "x"](rt::builtin(py, "range")?.call1((1,))?.try_iter()?)
            )?)?;
        }
        list.into_any()
    };
    let list = PyList::empty(py);
    for g in rt::iter(&gens)? {
        let g = g?;
        list.append(rt::builtin(py, "list")?.call1((&g,))?)?;
    }
    Ok(list.into_any())
}

// generator_expressions.py:153
#[rt::frame(m, "collected", 153, frame)]
pub fn collected<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    // The generator is in a reference cycle through the cell of `g`.
    let g = rt::Cell::new(py, "g", py.None())?;
    // generator_expressions.py:156
    #[rt::frame(m, "collected.<locals>.<genexpr>", 156)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        g: &rt::Cell<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        for x in rt::iter(iter)? {
            let x = x?;
            if !g.get()?.is_none() {
                return Ok(Some(x));
            }
        }
        Ok(None)
    }
    g.set(rt::genexpr!(
        frame,
        genexpr[&g](rt::builtin(py, "range")?.call1((3,))?.try_iter()?)
    )?)?;
    let r#ref = m.global("weakref")?.attr("ref")?.call1((g.get()?,))?;
    g.delete()?;
    m.global("gc")?.method0("collect")?;
    Ok(rt::bool(py, r#ref.call0()?.is_none()))
}

// generator_expressions.py:163
#[rt::frame(m, "reentrant", 163, frame)]
pub fn reentrant<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let g = rt::Cell::empty(py, "g")?;
    // generator_expressions.py:164
    #[rt::frame(m, "reentrant.<locals>.inner", 164)]
    fn inner<'py>(
        m: &rt::Module<'py>,
        g: &rt::Cell<'py>,
        x: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::builtin(m.py(), "next")?.call1((g.get()?,))
    }
    let inner = rt::function!(frame, inner[&g](x))?;
    // generator_expressions.py:167
    #[rt::frame(m, "reentrant.<locals>.<genexpr>", 167)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        inner: &Bound<'py, PyAny>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(x) = rt::next(iter)? {
            return Ok(Some(inner.call1((&x,))?));
        }
        Ok(None)
    }
    g.set(rt::genexpr!(
        frame,
        genexpr[inner](rt::builtin(py, "range")?.call1((3,))?.try_iter()?)
    )?)?;
    match (|| rt::builtin(py, "next")?.call1((g.get()?,)))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let exc = frame.catch(err);
            return Ok(exc.str()?.into_any());
        }
        Err(err) => return Err(err),
    }
    Ok(rt::none(py))
}

pub mod bad {
    use super::*;

    // generator_expressions.py:61
    /// The body of `class Bad`, run in its namespace `ns`.
    #[rt::frame(ns, "Bad", 61)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::def!(ns, __iter__(slf as "self"))?;
        rt::def!(ns, __next__(slf as "self"))?;
        Ok(())
    }

    // generator_expressions.py:62
    pub fn __iter__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Ok(slf.clone())
    }

    // generator_expressions.py:65
    #[rt::frame(m, "Bad.__next__", 65, frame)]
    pub fn __next__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(frame.raise(PyValueError::new_err("next failed")))
    }
}

pub mod table {
    use super::*;

    // generator_expressions.py:174
    /// The body of `class Table`, run in its namespace `ns`.
    #[rt::frame(ns, "Table", 174, frame)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        ns.set("size", 3)?;
        // generator_expressions.py:176
        #[rt::frame(m, "Table.<genexpr>", 176)]
        fn genexpr<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            if let Some(i) = rt::next(iter)? {
                return Ok(Some(PyTuple::new(m.py(), [&i, &i])?.into_any()));
            }
            Ok(None)
        }
        ns.set(
            "rows",
            ns.get("list")?.call1((rt::genexpr!(
                m,
                genexpr(ns.get("range")?.call1((ns.get("size")?,))?.try_iter()?),
                qualname = "Table.<genexpr>"
            )?,))?,
        )?;
        // generator_expressions.py:178
        #[rt::frame(m, "Table.<genexpr>", 178)]
        fn genexpr2<'py>(
            m: &rt::Module<'py>,
            iter: &Bound<'py, PyIterator>,
        ) -> PyResult<Option<Bound<'py, PyAny>>> {
            if let Some(i) = rt::next(iter)? {
                return Ok(Some(m.global("size")?));
            }
            Ok(None)
        }
        match (|| -> PyResult<Bound<'py, PyAny>> {
            let list = ns.get("list")?;
            let iter = ns.get("range")?.call1((2,))?.try_iter()?;
            list.call1((rt::genexpr!(
                m,
                genexpr2(iter),
                qualname = "Table.<genexpr>"
            )?,))
        })() {
            Ok(value) => {
                ns.set("broken", &value)?;
            }
            Err(err) if err.is_instance_of::<PyNameError>(m.py()) => {
                let exception = frame.catch(err);
                ns.set("exc", &exception)?;
                ns.get("print")?.call1(("class body:", ns.get("exc")?))?;
                ns.unbind("exc")?;
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}

/// Executes the body of `generator_expressions.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global(
        "__doc__",
        "Generator expressions are lazy: only the first iterable is evaluated\n\
            when the generator is created, the rest runs as items are requested, in\n\
            a `<genexpr>` frame of its own.",
    )?;
    m.set_global("gc", rt::import(m, "gc")?)?;
    m.set_global("itertools", rt::import(m, "itertools")?)?;
    m.set_global("traceback", rt::import(m, "traceback")?)?;
    m.set_global("weakref", rt::import(m, "weakref")?)?;
    rt::def!(m, show(exc))?;
    rt::def!(m, noisy(x))?;
    rt::def!(m, short_circuit(xs))?;
    rt::def!(m, infinite())?;
    rt::def!(m, late_binding())?;
    rt::def!(m, first_iterable_now())?;
    rt::def!(m, noisy_iterable())?;
    rt::def!(m, failing(xs))?;
    m.set_global("Bad", rt::class!(m, "Bad"(), bad::body, line = 61)?)?;
    rt::def!(m, failing_iteration())?;
    rt::def!(m, failing_condition(xs))?;
    rt::def!(m, closed_early())?;
    rt::def!(m, stop_iteration())?;
    rt::def!(m, protocol())?;
    rt::def!(m, consumers(words))?;
    rt::def!(m, captures())?;
    rt::def!(m, comprehension_variable())?;
    rt::def!(m, collected())?;
    rt::def!(m, reentrant())?;
    m.set_global("Table", rt::class!(m, "Table"(), table::body, line = 174)?)?;
    rt::builtins::print(
        py,
        (
            short_circuit(m, PyList::new(py, [1, 3, 5])?.as_any())?,
            infinite(m)?,
            late_binding(m)?,
        ),
    )?;
    rt::builtins::print(py, (first_iterable_now(m)?,))?;
    match (|| failing(m, PyList::new(py, [1, 0])?.as_any()))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
            let exception = frame.catch(err);
            m.set_global("exc", &exception)?;
            show(m, &m.global("exc")?)?;
            m.unbind_global("exc")?;
        }
        Err(err) => return Err(err),
    }
    match failing_iteration(m) {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyValueError>(py) => {
            let exception = frame.catch(err);
            m.set_global("exc", &exception)?;
            show(m, &m.global("exc")?)?;
            m.unbind_global("exc")?;
        }
        Err(err) => return Err(err),
    }
    match (|| failing_condition(m, PyList::new(py, [1, 0])?.as_any()))() {
        Ok(_) => {}
        Err(err) if err.is_instance_of::<PyZeroDivisionError>(py) => {
            let exception = frame.catch(err);
            m.set_global("exc", &exception)?;
            show(m, &m.global("exc")?)?;
            m.unbind_global("exc")?;
        }
        Err(err) => return Err(err),
    }
    closed_early(m)?;
    stop_iteration(m)?;
    protocol(m)?;
    consumers(m, PyList::new(py, ["a", "bb", "abc"])?.as_any())?;
    rt::builtins::print(
        py,
        (
            captures(m)?,
            comprehension_variable(m)?,
            m.global("Table")?.attr("rows")?,
            collected(m)?,
            reentrant(m)?,
        ),
    )?;
    // generator_expressions.py:202
    #[rt::frame(m, "<genexpr>", 202)]
    fn genexpr<'py>(
        m: &rt::Module<'py>,
        iter: &Bound<'py, PyIterator>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        if let Some(c) = rt::next(iter)? {
            return Ok(Some(c.mul(2)?));
        }
        Ok(None)
    }
    m.set_global(
        "module_gen",
        rt::genexpr!(m, genexpr(rt::str(py, "ab").try_iter()?))?,
    )?;
    rt::builtins::print(
        py,
        (
            rt::builtin(py, "next")?.call1((m.global("module_gen")?,))?,
            rt::builtin(py, "list")?.call1((m.global("module_gen")?,))?,
            m.global("module_gen")?.get_type().attr("__qualname__")?,
        ),
    )?;
    Ok(())
}
