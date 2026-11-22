# Differences from CPython

A transpiled program raises the same exceptions with the same messages and has the same side effects in the same order as under CPython. These are the known, deliberate differences. Most only show in introspection, tooling or object lifetimes, but check the list before deploying. Statements kept as Python (`steelix coverage` lists them) run in CPython itself and have none of the differences that concern native code.

## Tracebacks and warnings

- A native function's traceback frame reports the line of its `def` (line 1 for module code) and shows that source line, not the failing statement. Transpile with `--traceback-lines` to get statement lines; the generated Rust then has a `frame.line(N);` before statements.
- Even with `--traceback-lines`, a statement spanning several lines reports one of them, and native frames have no `^^^` markers.
- `warnings.warn` reports the location of the nearest Python frame, or `<sys>:0` when the whole call chain is native. The default filters therefore differ: a `DeprecationWarning` from native code is not shown, and all `<sys>:0` warnings with the same text count as one location. Warnings have no source line.
- Compile-time `SyntaxWarning`s are not printed: the program never compiles the Python source.
- With `--no-sources` distributions, tracebacks have no source lines.

## Introspection and tools

- Native functions are a runtime type, not `types.FunctionType`: `inspect.isfunction(f)` is False and `type(f).__name__` is `function`. `__code__` only describes the signature. `inspect.signature`, `functools.wraps`, `__closure__` and pickling work.
- Native generators, coroutines and generator expressions are runtime types too: `inspect.isgenerator`/`inspect.iscoroutine` are False, while `collections.abc` checks and `asyncio.iscoroutine` are True. `gi_frame`/`cr_frame`, `gi_code`/`cr_code` and `cr_await` are `None`, so `inspect.getgeneratorstate` reports a fresh generator as closed, `repr(task)` has no "running at" and `Task.get_stack()` is empty.
- Native code runs without interpreter frames. `sys._getframe`, `inspect.stack`, `f_locals`, profilers, coverage tools and debuggers do not see into it; `logging`'s `%(filename)s`, `%(lineno)d` and `%(funcName)s` point at the nearest Python frame (often inside `logging`).
- APIs that guess the module from the caller's frame get it wrong: a class made by three-argument `type()` has no `__module__`, `enum.Enum("C", "A B")` gets `<unknown>`.
- Project modules are loaded by `steelix_runtime.NativeLoader`: `get_source`, `get_data` and `importlib.resources` work, but `get_code` returns a stub, so bytecode tools see nothing.
- `del f.__defaults__` (and other function metadata) raises `AttributeError`; CPython sets it to `None`.

## Calls, globals and monkeypatching

- Calls between the module-level functions of a module are direct Rust calls. Replacing a function from another module (`mock.patch("app.f")`, `app.f = g`) does not affect them; reassigning it inside its own module is recognized. If the replaced function's default arguments are needed, the call raises a `TypeError` that says so.
- Likewise, assigning a function's `__defaults__` from another module is not seen by direct calls that omit those arguments.
- Specialized builtins (`len`, `isinstance`, `str`, `repr`, `bool`, `abs`, `type`, `iter`) call the C API directly, so `builtins.len = ...` does not affect them. Rebinding the name in the module (`len = ...`, `def len`) is recognized. Replacing `builtins.__build_class__` only affects classes kept as Python.
- Other builtins are looked up after their arguments are evaluated (Python looks them up first).
- In a function body, a direct call does not check that the callee's `def` has already run; at module level it does (`NameError`).

## Object lifetimes

These show with `__del__`, weakref callbacks and `finally` in generators, not otherwise.

- Temporaries live until the end of their statement. Variables used only inside a `for`, `try`, `with` or `match` case are released when that block ends, CPython releases them when the function returns. The release order of locals at function return can differ.
- When an exception leaves a native function, its locals are released immediately; CPython keeps them alive with the traceback's frame.
- The garbage collector cannot see what a suspended native generator or coroutine holds in its locals, temporaries, `except ... as e` or `with` targets. Reference cycles through those are never collected and their `finally` blocks do not run until the cycle is broken some other way.
- Literals create a new object each time they are evaluated, where CPython shares constants; only identity comparisons of strings and large integers notice.

## Recursion

- Direct recursion and deep `await`/`yield from` chains are limited by the C stack, not by `sys.getrecursionlimit()`. Too deep raises `RecursionError` with a `Stack overflow (used N kB)` message.

## Generators and coroutines

- A native generator or coroutine suspended in an `except` block restores, at the end of that block, the exception that was being handled when it started, not the one its current caller is handling.
- In `yield from`/`await` delegation, the inner generator sees the outer one's handled exception (`sys.exception()`, `__context__`), not its caller's.
- `throw()` checks its arguments up front: `g.throw(42)` raises `TypeError` to the caller instead of inside the delegate.
- `coroutine.__await__()` on a suspended coroutine raises `RuntimeError` immediately.

## Classes

- Decorators that recreate the class, such as `dataclass(slots=True)`, only update the `__class__` cell of Python functions. Zero-argument `super()` in a native method may then refer to the old class and raise `TypeError`, unless a property in the class also uses `super()` or `__class__`.
- With a metaclass whose `__prepare__` returns a custom mapping, statements kept as Python in the class body cause extra reads and writes of `__module__`, `__qualname__` and similar keys.

## Running a program

- `sys.argv[0]` and `__file__` are the absolute path of the script, as with `python /abs/path/app.py`.
- `sys.executable` is a Python interpreter, not the program. multiprocessing children are started through the program; your own `multiprocessing.set_executable` overrides that.
- A program named `python`, `python3` or `python3.N` behaves as the Python interpreter.
- Distributions (`steelix build`) ignore `PYTHON*` environment variables and the user site-packages, like `python -E -s`; set `STEELIX_PYTHON_ENV=1` to restore them. The project directory is always `sys.path[0]`, also with `PYTHONSAFEPATH`.
- In a distribution the project lives in `dist/app/src/`; code that prints or depends on its original directory name sees the new one. Data files outside the project's packages need `--add-data`.
- Project modules are found by a finder in front of `PathFinder`, so they win over a same-named module that the program puts earlier on `sys.path`.
- A Rust panic (from a rewritten function, or a steelix bug) is reported as an uncaught `pyo3_runtime.PanicException` with exit code 1.

## Extension modules (`--mode ext`)

- Each extension module carries its own copy of the runtime, so `type(a.f) is type(b.f)` is False for functions from two separately built modules. Executables do not have this difference.
- Tracebacks of statements kept as Python show the extension module's path and no source lines.

## Threads

- Native loops, recursive calls and resumed generators let other threads take the GIL when one has waited for `sys.getswitchinterval()`, like the interpreter does, and loops run pending signal handlers (Ctrl-C raises `KeyboardInterrupt`).
- Pending calls (`Py_AddPendingCall`) and exceptions set with `PyThreadState_SetAsyncExc` are only handled once native code returns to the interpreter.
- A generator expression consumed entirely by a C function (`any`, `sum`) lets other threads run on every item but does not check signals.
- No free-threaded Python.
