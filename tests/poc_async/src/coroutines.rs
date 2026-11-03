//! `coroutines.py` with its coroutines and generators written by hand the
//! way steelix would generate them (PoC, design §8 "协程和生成器"). The
//! parts that drive them stay Python. Line markers (`frame.line(N)`) are
//! only written where the test compares traceback lines.

use pyo3::exceptions::{PyGeneratorExit, PyKeyError, PyStopIteration, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// coroutines.py:29
pub async fn add_later<'py>(
    m: &rt::Module<'py>,
    a: &Bound<'py, PyAny>,
    b: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "add_later", 29, async |_| {
        rt::await_(m.global("asyncio")?.method1("sleep", (0,))?).await?;
        a.add(b)
    })
}

// coroutines.py:34
pub async fn ticker<'py>(
    m: &rt::Module<'py>,
    name: &Bound<'py, PyAny>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "ticker", 34, async |_| {
        for i in rt::builtin(py, "range")?.call1((n,))?.try_iter()? {
            let i = i?;
            rt::builtin(py, "print")?.call1(("tick", name, &i))?;
            rt::await_(m.global("asyncio")?.method1("sleep", (0,))?).await?;
            py.check_signals()?;
        }
        name.mul(2)
    })
}

// coroutines.py:41
pub async fn combine<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "combine", 41, async |_| {
        let direct = add_later(m, x, &rt::int(py, 1)).await?;
        let alias = m.global("add_later")?;
        let through_object = rt::await_(alias.call1((x, 2))?).await?;
        let from_python = rt::await_(m.global("py_double")?.call1((x,))?).await?;
        Ok(PyTuple::new(py, [direct, through_object, from_python])?.into_any())
    })
}

// coroutines.py:49
pub async fn guarded<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "guarded", 49, async |frame| {
        let result: PyResult<()> = async {
            log.method1("append", ("started",))?;
            rt::await_(m.global("asyncio")?.method1("sleep", (10,))?).await?;
            log.method1("append", ("not reached",))?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        log.method1("append", ("finally",))?;
        rt::await_(m.global("asyncio")?.method1("sleep", (0,))?).await?;
        let exception = m.global("sys")?.method0("exception")?;
        let name = exception.get_type().attr("__name__")?;
        log.method1(
            "append",
            (rt::str(py, "after await in finally: ").add(name)?,),
        )?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// coroutines.py:60
pub async fn failing<'py>(
    m: &rt::Module<'py>,
    x: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "failing", 60, async |frame| {
        frame.line(61);
        rt::await_(m.global("asyncio")?.method1("sleep", (0,))?).await?;
        frame.line(62);
        let message = rt::str(py, "bad ").add(x.str()?)?;
        Err(frame.raise(PyValueError::new_err((message.unbind(),))))
    })
}

// coroutines.py:65
pub async fn outer_fail<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "outer_fail", 65, async |frame| {
        frame.line(66);
        failing(m, &rt::int(py, 1)).await?;
        Ok(rt::none(py))
    })
}

// coroutines.py:69
pub async fn catcher<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "catcher", 69, async |frame| {
        let print = rt::builtin(py, "print")?;
        match failing(m, &rt::int(py, 2)).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                print.call1(("caught", &e))?;
                rt::await_(m.global("asyncio")?.method1("sleep", (0,))?).await?;
                let exception = m.global("sys")?.method0("exception")?;
                print.call1(("still handling", exception.repr()?))?;
            }
            Err(err) => return Err(err),
        }
        let exception = m.global("sys")?.method0("exception")?;
        print.call1(("after handler", exception.repr()?))?;
        Ok(rt::none(py))
    })
}

// coroutines.py:79
pub async fn echo<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "echo", 79, async |frame| {
        frame.line(80);
        let a = rt::await_(m.global("Suspend")?.call1(("first",))?).await?;
        frame.line(81);
        rt::builtin(m.py(), "print")?.call1(("echo got", &a))?;
        frame.line(82);
        let b = rt::await_(m.global("Suspend")?.call1(("second",))?).await?;
        frame.line(83);
        a.add(b)
    })
}

// coroutines.py:86
pub async fn cleanup<'py>(
    m: &rt::Module<'py>,
    tag: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "cleanup", 86, async |frame| {
        let result: PyResult<()> = async {
            rt::await_(m.global("Suspend")?.call1((tag,))?).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        rt::builtin(py, "print")?.call1(("cleanup finally", tag))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// coroutines.py:93
pub async fn stubborn<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "stubborn", 93, async |frame| {
        match rt::await_(m.global("Suspend")?.call1((1,))?).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                rt::builtin(py, "print")?.call1(("ignoring GeneratorExit",))?;
                rt::await_(m.global("Suspend")?.call1((2,))?).await?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:101
pub async fn reenter<'py>(
    m: &rt::Module<'py>,
    r#box: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "reenter", 101, async |frame| {
        match r#box.get_item(0)?.method1("send", (py.None(),)) {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyValueError>(py) => {
                let e = frame.catch(err);
                let _handling = rt::Handling::new(&e);
                rt::builtin(py, "print")?.call1(("reenter:", &e))?;
            }
            Err(err) => return Err(err),
        }
        Ok(rt::str(py, "reentered"))
    })
}

// coroutines.py:109
pub async fn stop_inside<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "stop_inside", 109, async |frame| {
        Err(frame.raise(PyStopIteration::new_err((5,))))
    })
}

// coroutines.py:113
pub async fn await_int<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "await_int", 113, async |_| {
        rt::await_(rt::int(py, 1)).await?;
        Ok(rt::none(py))
    })
}

// coroutines.py:117
pub async fn compute<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "compute", 117, async |_| {
        let doubled = rt::await_(m.global("Request")?.call1(("double", n))?).await?;
        rt::await_(m.global("Request")?.call1(("add", doubled, 1))?).await
    })
}

// coroutines.py:122
pub async fn handler_state<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "handler_state", 122, async |frame| {
        let print = rt::builtin(py, "print")?;
        let result: PyResult<()> = Err(frame.raise(PyKeyError::new_err((1,))));
        match result {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                let exception = m.global("sys")?.method0("exception")?;
                print.call1(("in handler", exception.repr()?))?;
                rt::await_(m.global("Suspend")?.call1(("in handler",))?).await?;
                let exception = m.global("sys")?.method0("exception")?;
                print.call1(("after resume", exception.repr()?))?;
            }
            Err(err) => return Err(err),
        }
        let exception = m.global("sys")?.method0("exception")?;
        print.call1(("after the handler", exception.repr()?))?;
        Ok(rt::none(py))
    })
}

// coroutines.py:132
pub async fn nested_guard<'py>(
    m: &rt::Module<'py>,
    log: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "nested_guard", 132, async |_| {
        guarded(m, log).await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:136
pub async fn countdown<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "countdown", 136, async |_| {
        if n.eq(0)? {
            return Ok(rt::int(m.py(), 0));
        }
        // A recursive `async fn` awaits itself through a `Box`.
        n.add(Box::pin(countdown(m, &n.sub(1)?)).await?)
    })
}

// coroutines.py:145
pub async fn count_up<'py>(
    m: &rt::Module<'py>,
    n: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "count_up", 145, async |_| {
        let mut i = rt::int(py, 0);
        while i.lt(n)? {
            rt::yield_(&i).await?;
            i = rt::iadd(&i, 1)?;
            py.check_signals()?;
        }
        Ok(rt::none(py))
    })
}

// coroutines.py:152
pub async fn accumulate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "accumulate", 152, async |_| {
        let mut total = rt::int(py, 0);
        loop {
            let x = rt::yield_(&total).await?;
            if x.is_none() {
                return Ok(total);
            }
            total = rt::iadd(&total, &x)?;
            py.check_signals()?;
        }
    })
}

// coroutines.py:161
pub async fn resilient<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "resilient", 161, async |frame| {
        loop {
            match rt::yield_(rt::str(py, "ready")).await {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                    let e = frame.catch(err);
                    let _handling = rt::Handling::new(&e);
                    rt::builtin(py, "print")?.call1(("handled", e.repr()?))?;
                }
                Err(err) => return Err(err),
            }
            py.check_signals()?;
        }
    })
}

// coroutines.py:169
pub async fn with_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "with_finally", 169, async |frame| {
        let result: PyResult<()> = async {
            rt::yield_(rt::int(py, 1)).await?;
            rt::yield_(rt::int(py, 2)).await?;
            Ok(())
        }
        .await;
        let finally = frame.finally(result);
        rt::builtin(py, "print")?.call1(("generator finally",))?;
        finally.resume()?;
        Ok(rt::none(py))
    })
}

// coroutines.py:177
pub async fn delegate<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "delegate", 177, async |_| {
        let result = rt::yield_from(m.global("py_inner")?.call0()?).await?;
        rt::builtin(py, "print")?.call1(("delegate got", &result))?;
        rt::yield_(&result).await?;
        Ok(rt::none(py))
    })
}

// coroutines.py:183
pub async fn in_with<'py>(
    m: &rt::Module<'py>,
    manager: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "in_with", 183, async |frame| {
        frame
            .with_async(manager, async |value| {
                rt::yield_(value).await?;
                rt::yield_(value.add(1)?).await?;
                Ok(())
            })
            .await?;
        Ok(rt::none(m.py()))
    })
}

// coroutines.py:189
pub async fn pep479<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "pep479", 189, async |_| {
        rt::yield_(rt::int(py, 1)).await?;
        rt::builtin(py, "next")?.call1((PyList::empty(py).try_iter()?,))?;
        Ok(rt::none(py))
    })
}

// coroutines.py:194
pub async fn bad_finally<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "bad_finally", 194, async |frame| {
        let result: PyResult<()> = async {
            rt::yield_(rt::int(py, 1)).await?;
            Ok(())
        }
        .await;
        let _finally = frame.finally(result);
        Err(frame.raise(PyKeyError::new_err(("in finally",))))
    })
}

// coroutines.py:201
pub async fn closing_value<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "closing_value", 201, async |frame| {
        match rt::yield_(rt::int(py, 1)).await {
            Ok(_) => {}
            Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                return Ok(rt::str(py, "closed value"));
            }
            Err(err) => return Err(err),
        }
        Ok(rt::none(py))
    })
}

/// Executes the body of `coroutines.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    rt::frame!(m, "<module>", 1, |_| {
        m.set_global(
            "__doc__",
            "Native coroutines and generators (PoC, design §8): every function in the\n\
                first half is hand-written as native Rust in `src/coroutines.rs`, the way\n\
                steelix would generate it; the drivers in the second half stay Python.",
        )?;
        m.set_global("asyncio", rt::import(m, "asyncio")?)?;
        m.set_global("collections", rt::import(m, "collections.abc")?)?;
        m.set_global("gc", rt::import(m, "gc")?)?;
        m.set_global("inspect", rt::import(m, "inspect")?)?;
        m.set_global("re", rt::import(m, "re")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
        m.set_global("traceback", rt::import(m, "traceback")?)?;
        m.set_global("warnings", rt::import(m, "warnings")?)?;
        rt::exec_python!(
            m,
            line = 15,
            r#"
class Suspend:
    """Awaitable that hands its value to whoever drives the coroutine and
    returns what is sent back."""

    def __init__(self, value):
        self.value = value

    def __await__(self):
        return (yield self.value)
"#
        )?;
        rt::def!(m, async add_later(a, b))?;
        rt::def!(m, async ticker(name, n))?;
        rt::def!(m, async combine(x))?;
        rt::def!(m, async guarded(log))?;
        rt::def!(m, async failing(x))?;
        rt::def!(m, async outer_fail())?;
        rt::def!(m, async catcher())?;
        rt::def!(m, async echo())?;
        rt::def!(m, async cleanup(tag))?;
        rt::def!(m, async stubborn())?;
        rt::def!(m, async reenter(r#box))?;
        rt::def!(m, async stop_inside())?;
        rt::def!(m, async await_int())?;
        rt::def!(m, async compute(n))?;
        rt::def!(m, async handler_state())?;
        rt::def!(m, async nested_guard(log))?;
        rt::def!(m, async countdown(n))?;
        rt::def!(m, gen count_up(n))?;
        rt::def!(m, gen accumulate())?;
        rt::def!(m, gen resilient())?;
        rt::def!(m, gen with_finally())?;
        rt::def!(m, gen delegate())?;
        rt::def!(m, gen in_with(manager))?;
        rt::def!(m, gen pep479())?;
        rt::def!(m, gen bad_finally())?;
        rt::def!(m, gen closing_value())?;
        rt::exec_python!(
            m,
            line = 208,
            r#"
# --- kept as Python ---


async def py_awaits(awaitable):
    return await awaitable


async def py_double(x):
    await asyncio.sleep(0)
    return x * 2


def py_inner():
    try:
        received = yield "inner 1"
        print("inner received", received)
        try:
            yield "inner 2"
        except KeyError:
            print("inner caught KeyError")
        yield "inner 3"
    finally:
        print("inner finally")
    return "inner result"


class Manager:
    def __enter__(self):
        print("enter")
        return 10

    def __exit__(self, typ, value, tb):
        print("exit", typ.__name__ if typ else None)
        return False


class Request:
    def __init__(self, *args):
        self.args = args

    def __await__(self):
        return (yield self)


def frames(e):
    return [(f.name, f.lineno) for f in traceback.extract_tb(e.__traceback__)]


def unraisable(u):
    message = re.sub(r" at 0x[0-9a-f]+", "", u.err_msg or "")
    print("unraisable:", u.exc_type.__name__, u.exc_value, "|", message)


sys.unraisablehook = unraisable


async def main():
    print("combine", await combine(10))
    print("gather", await asyncio.gather(ticker("a", 2), ticker("b", 3)))
    log = []
    task = asyncio.create_task(guarded(log))
    await asyncio.sleep(0)
    task.cancel()
    try:
        await task
    except asyncio.CancelledError:
        print("cancelled", log, task.cancelled())
    for make in (nested_guard, lambda log: py_awaits(guarded(log))):
        log = []
        task = asyncio.create_task(make(log))
        await asyncio.sleep(0)
        task.cancel()
        try:
            await task
        except asyncio.CancelledError as e:
            print("cancelled", log, [name for name, _ in frames(e)])
    try:
        await outer_fail()
    except ValueError as e:
        print("traceback", e, frames(e))
    await catcher()
    print("countdown", await countdown(30))


print("--- asyncio")
asyncio.run(main())

print("--- send, throw, close")
c = echo()
print(c.send(None))
print(c.send(1))
try:
    c.send(2)
except StopIteration as e:
    print("StopIteration", e.value, e.args)
try:
    c.send(None)
except RuntimeError as e:
    print("reuse:", e)
try:
    c.throw(ValueError("late"))
except RuntimeError as e:
    print("reuse throw:", e)
print("close after end", c.close())

c = echo()
c.send(None)
try:
    c.throw(KeyError("thrown"))
except KeyError as e:
    print("throw propagates", repr(e), frames(e)[1:])

c = cleanup("closed")
print(c.send(None))
print("close returns", c.close(), c.cr_suspended)

c = stubborn()
c.send(None)
try:
    c.close()
except RuntimeError as e:
    print(e)
print("still suspended", c.cr_suspended)
del c
print("after del stubborn")

box = []
c = reenter(box)
box.append(c)
try:
    c.send(None)
except StopIteration as e:
    print("reenter returned", e.value)
del box, c

try:
    stop_inside().send(None)
except RuntimeError as e:
    print(e, repr(e.__cause__))

try:
    await_int().send(None)
except TypeError as e:
    print("TypeError:", e)

try:
    echo().send(1)
except TypeError as e:
    print("TypeError:", e)

c = echo()
try:
    c.throw(KeyError("before start"))
except KeyError as e:
    print("thrown before start", repr(e), frames(e)[1:])
try:
    c.send(None)
except RuntimeError as e:
    print("after throw:", e)

c = handler_state()
print(c.send(None))
print("outside", repr(sys.exception()))
try:
    raise ValueError("caller")
except ValueError:
    try:
        c.send(None)
    except StopIteration:
        pass

print("--- finalization")
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    c = add_later(1, 2)
    del c
    gc.collect()
    c = add_later(1, 2)
    c.close()
    del c
    for w in caught:
        print("warning", w.category.__name__, w.message, w.lineno)
with warnings.catch_warnings():
    warnings.simplefilter("error")
    c = add_later(1, 2)
    del c
    print("warning as error reported")

c = cleanup("deleted")
c.send(None)
print("before del")
del c
print("after del")

cycle = [cleanup("in a cycle")]
cycle.append(cycle)
cycle[0].send(None)
del cycle
print("before collect")
gc.collect()
print("after collect")


class Holder:
    def __str__(self):
        return "holder"


holder = Holder()
holder.coroutine = cleanup(holder)
holder.coroutine.send(None)
del holder
gc.collect()
print("after collecting a cycle through the locals")

w = with_finally()
next(w)
print("before del generator")
del w
print("after del generator")
b = bad_finally()
next(b)
del b
print("after bad_finally")

print("--- introspection")
c = add_later(1, 2)
print("iscoroutinefunction", inspect.iscoroutinefunction(add_later))
print("isgeneratorfunction", inspect.isgeneratorfunction(count_up))
print("asyncio.iscoroutine", asyncio.iscoroutine(c))
print("Coroutine abc", isinstance(c, collections.abc.Coroutine))
print("inspect.iscoroutine", inspect.iscoroutine(c))
print("names", c.__name__, c.__qualname__, repr(c).split(" at ")[0])
c.close()
g = count_up(1)
print("Generator abc", isinstance(g, collections.abc.Generator))
print("names", g.__name__, g.__qualname__, repr(g).split(" at ")[0])

print("--- trampoline")


def trampoline(coro):
    value = None
    while True:
        try:
            request = coro.send(value)
        except StopIteration as e:
            return e.value
        op, *args = request.args
        print("request", op, args)
        value = args[0] * 2 if op == "double" else sum(args)


print("trampoline", trampoline(compute(20)))

print("--- generators")
print(list(count_up(4)))
for i in count_up(2):
    print("for", i)

acc = accumulate()
print(next(acc), acc.send(5), acc.send(7))
try:
    acc.send(None)
except StopIteration as e:
    print("accumulated", e.value)

r = resilient()
print(next(r))
print(r.throw(KeyError("k")))
r.close()
print("closed", r.gi_suspended)

w = with_finally()
print(next(w))
w.close()
print("closed with finally")

d = delegate()
print(next(d))
print(d.send("hello"))
print(d.throw(KeyError("x")))
print(next(d))
try:
    next(d)
except StopIteration:
    print("delegate done")
d = delegate()
next(d)
d.close()
print("delegate closed")

m = in_with(Manager())
print(next(m), next(m))
m.close()
m = in_with(Manager())
print(next(m))
try:
    m.throw(ValueError("in with"))
except ValueError as e:
    print("with saw", e)

v = closing_value()
next(v)
print("close returns", v.close())

p = pep479()
print(next(p))
try:
    next(p)
except RuntimeError as e:
    print(e, repr(e.__cause__))
"#
        )?;
        Ok(())
    })
}
