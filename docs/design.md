# Steelix 设计文档

## 1. 目标

把 Python 源码转换成**可读的** Rust 代码。生成的代码通过 PyO3 继续调用 libpython，所以标准库和第三方 Python 库照常可用。

**以应用（app）场景为主**，库场景其次。典型迁移路径如下。

1. 用 steelix 把一个 Python 应用（入口脚本加上它自己的模块）整体转成 Rust crate。
2. 生成的 crate 编译成嵌入 libpython 的可执行程序，`./app` 的行为与 `python app.py` 一致。第三方依赖仍然从 venv 里 import。
3. 开发者逐个函数把 `Bound<PyAny>` 改写成原生 Rust 类型和逻辑，逐步减少对 Python 的依赖，最终摆脱它。

库场景（生成 PyO3 扩展模块替换原 Python 模块）保留为 `--mode ext`，共用同一套 lowering，但优先级较低。

所以**可读性优先于性能**。生成的代码应该像一个熟悉 PyO3 的人手写出来的样子。

### 非目标（至少在 PoC 阶段）

- 性能优化，类型推导出原生类型（后续可基于类型注解做可选的 typed lowering）
- 100% 语义兼容。允许有文档化的差异（见第 12 节）
- generator、async/await、match、metaclass 自定义、`exec`/`eval` 等复杂特性

## 2. 总体流水线

```
.py 源码
  │  frontend/parse.py     ast.parse + tokenize 收集注释
  ▼
Python AST + 注释
  │  frontend/scope.py     symtable 分析作用域（local/global/nonlocal/cell/free）
  │  frontend/bindings.py  自行遍历 AST，统计模块级名字的绑定次数，
  │                        局部变量的定值分析（definite assignment）
  ▼
带作用域信息的 AST
  │  lower/*               逐节点降级，按 Python 求值顺序生成临时变量，
  │                        遇到不支持的语法抛 UnsupportedError
  ▼
Rust AST（rustast/nodes.py）
  │  rustast/printer.py    渲染成源码，再强制过一遍 rustfmt
  ▼
.rs 源码
  │  emit/*                生成 Cargo.toml、lib.rs、每个 Python 模块一个 Rust mod
  ▼
Cargo crate
```

选择先构建一个小型 Rust AST 再打印，而不是直接拼字符串。这样 lowering 和格式化解耦，也方便做局部的可读性改写（例如去掉多余的 clone、合并 `?`）。Rust AST 只覆盖生成代码用得到的部分，不追求完整。

## 3. 目录结构

```
steelix/
├── pyproject.toml             uv 项目，Python 3.14
├── Cargo.toml                 Rust workspace，成员为 crates/*
├── .cargo/config.toml         仓库内构建时 PYO3_PYTHON 指向 .venv/bin/python
├── CLAUDE.md                  给后续会话的约定与命令速查
├── docs/
│   ├── design.md              本文档
│   └── roadmap.md             里程碑与任务拆解
├── src/steelix/
│   ├── cli.py                 命令行入口 `steelix transpile`
│   ├── diagnostics.py         诊断信息、源码位置、UnsupportedError
│   ├── frontend/              解析、作用域分析、绑定分析
│   ├── lower/                 Python AST → Rust AST
│   ├── rustast/               Rust AST 定义、打印、rustfmt
│   └── emit/                  生成完整 crate
├── crates/
│   └── steelix-runtime/       生成代码依赖的运行时（Function、异常 guard 等）
└── tests/
    ├── cases/                 端到端用例（*.py），同时作为 golden 输入
    ├── golden/                期望生成的 .rs（快照测试）
    └── test_*.py
```

## 4. 生成代码的形态

输入

```python
def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


print(fib(10))
```

生成的输出（`tests/golden/fib.rs`）

```rust
use pyo3::prelude::*;
use steelix_runtime as rt;

// fib.py:1
pub fn fib<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(py)?;
    if n.lt(2)? {
        return Ok(n.clone());
    }
    fib(py, &n.sub(1)?)?.add(fib(py, &n.sub(2)?)?)
}

/// Executes the body of `fib.py` in the module `m`.
pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let globals = &m.dict();
    rt::def!(globals, fib(n))?;
    rt::builtins::print(py, (fib(py, &rt::int(py, 10))?,))?;
    Ok(())
}
```

要点

- 所有 Python 值默认都是 `Bound<'py, PyAny>`，运算直接用 PyO3 的 `add`、`lt`、`getattr`、`call` 等方法
- 函数体是普通的 Rust fn，显式接收 `py: Python<'py>`，参数用 `&Bound`。需要动态查找全局变量的函数（以及直接调用了这类函数的函数）额外接收 `globals: &Bound<'py, PyDict>`，放在 `py` 之后
- 模块体生成 `exec_module(m)`，只依赖传入的模块对象，与输出模式无关
- 模块级变量的读写走 globals 字典：`rt::global(globals, "LIMIT")?` 先查 globals 再查 builtins，找不到抛 `NameError`；写入用 `globals.set_item("LIMIT", 20)?`。模块里没有绑定的名字视为 builtin，`print` 用 `rt::builtins::print`，`len(x)` 等少数几个直接写成 PyO3 方法（见第 8 节），其余用 `rt::builtin(py, "max")?`
- 比较在条件里用 `lt` 等返回 bool 的方法，需要值时用 `rich_compare(.., CompareOp::Lt)`
- 函数末尾的 `return` 写成尾表达式，返回值本身是 `PyResult` 时不再包一层 `Ok(..?)`
- 对 Python 可见的函数对象由 runtime 的 `Function` 包装（见第 6 节），注册只占一行：`rt::def!(globals, fib(n))?`，签名的写法和 Python 一样（`greet(name, greeting = "Hello", *, punct = "!")`）。需要 globals 的写成 `rt::def!(globals, scale(globals, x))`，Rust 名字与 Python 名字不同时加 `name = "..."`，有 docstring 时加 `doc = "..."`
- 模块级函数若只绑定一次，并且能在转换时绑定参数，调用时直接调用 Rust 函数（见第 6、7 节）
- 嵌套函数和 lambda 是写在外层函数体里的 Rust fn，被捕获并会改变的变量是 `rt::Cell`（见第 6 节）
- 会递归的函数（直接调用构成的调用图里有环）开头加 `let _guard = rt::RecursionGuard::enter(py)?;`，否则栈溢出会直接 SIGSEGV 而不是抛 `RecursionError`。经过 Python 对象的调用（`PyObject_Call`）CPython 自己会检查，不需要 guard
  - 3.14 的 `Py_EnterRecursiveCall` 按 C 栈剩余空间判断，不看 `sys.getrecursionlimit()`，报错信息是 `Stack overflow (used N kB)`（见第 12 节）
  - 已验证的坑：栈快耗尽时 `PyErr::fetch` 第一次创建 PyO3 的 `PanicException` 类型会失败并在 PyO3 的懒初始化锁上死锁。`RecursionGuard::enter` 每次先确保这个类型已经创建
- runtime 一律通过 `rt::` 路径引用，不用 glob import，避免被 Python 里叫 `int`、`print` 的局部变量遮蔽
- 字面量规则：作为 `IntoPyObject` 参数时直接写字面量（`n.lt(2)`），超出 i32 范围的加 `i64` 后缀或走 `rt::int`，需要独立的 Python 对象时用 `rt::int(py, 10)`
- 尽量保留 Python 的名字、注释、docstring（转成 `///`），并在函数前标注源位置注释

## 5. 变量模型

Python 局部变量的作用域是整个函数，Rust 的 `let` 是块作用域，不能逐句对应。

- 每个局部变量只声明一次，循环变量、`if`/`try` 内的赋值都是对它的赋值，不再重新 `let`
  - 第一次出现是函数体顶层的赋值语句时，直接在那里写 `let x = ...;`（顶层语句按顺序执行，它支配之后所有使用点），这样直线代码最可读
  - 否则在函数开头声明 `let x: Bound<'py, PyAny>;`，由后面的分支赋值
  - 可能重复赋值（另做一遍"可能已赋值"分析，循环取不动点）时才加 `mut`，与 rustc 的判断一致，不产生 `unused_mut`
  - 被重新赋值的参数在开头遮蔽成 `let mut n = n.clone();`
- 自行做定值分析，规则与 rustc 一致或更保守（条件一律视为不透明，`while True` 没有 `break` 时之后不可达）。能确定已赋值的变量用 `Bound`，不能确定的用 `Option<Bound>`：声明为 `let mut x: Option<Bound<'py, PyAny>> = None;`，赋值写 `x = Some(..)`，读取写 `rt::local(&x, "x")?`，未赋值时抛和 CPython 一样的 `UnboundLocalError`
- 不可达的语句（`return`、`break`、`continue` 之后，没有 `break` 的 `while True` 之后）直接丢弃，函数末尾可达时补 `Ok(rt::none(py))`。模块级代码也做同样的可达性分析，避免 rustc 报 unreachable 警告
- 定值分析覆盖 `for`/`while`（在循环头取不动点）、`break`/`continue`、循环 `else`、解包赋值、增量赋值（先读后写）、`import`（绑定局部变量）和 `del`
- `del x` 让变量变成未赋值状态，所以被 `del` 的局部变量（包括参数）一律是 `Option`。删除时能确定已赋值就写 `x = None;`，否则写 `rt::del_local(&mut x, "x")?;`，未赋值时抛 `UnboundLocalError`。被 `del` 的参数在开头遮蔽成 `let mut a: Option<..> = Some(a.clone());`
- `Option` 变量如果第一次出现是顶层赋值，也在那里声明：`let mut x = Some(..);`
- 循环变量：如果一个名字只被某个 `for` 的目标绑定（可以是嵌套元组），并且只在这个循环体内读取，就在每次迭代开头声明，不提前声明也不需要 `mut`

  ```rust
  for x in xs.try_iter()? {
      let x = x?;
      ..
  }
  for item in pairs.try_iter()? {
      let [key, value] = rt::unpack(&item?)?;
      ..
  }
  ```

  否则（循环后还要读，或者别处也赋值）照常赋值给事先声明的变量：`for item in xs.try_iter()? { x = item?; .. }`
- 解包赋值的目标都是普通局部变量时，第一次出现写成 `let [a, b] = rt::unpack(&value)?;`，之后写成解构赋值 `[a, b] = rt::unpack(&value)?;`。目标里有属性、下标、全局变量或 `Option` 变量时，先解包到临时变量，再按顺序逐个赋值
- 函数内的带注解赋值 `x: int = v` 在分析之前改写成 `x = v`（`frontend/desugar.py`），因为局部变量的注解从不求值。没有值的 `x: int` 只让 `x` 成为局部变量，直接去掉
- 函数顶层第一次出现的 `import` 也直接 `let os = rt::import(globals, "os")?;`
- 被内层函数捕获并且之后会改变的变量（cell）用 `rt::Cell`，不参与上面的处理，见第 6 节

## 6. 函数对象、闭包与默认参数

`wrap_pyfunction!` 得到的是 `builtin_function_or_method`，已验证它有以下问题：`functools.wraps` 无法写 `__name__`，不能设置属性，放进类里不会绑定 `self`。`PyCFunction::new_closure` 也有同样的问题，并且要求 `Send + Sync + 'static`，无法捕获 `Bound`。

因此 runtime 提供 `#[pyclass(frozen, dict, weakref)] Function`

- 实现 `__call__`（按存储的签名描述绑定参数）和 `__get__`（像 Python 函数一样绑定方法，返回 `types.MethodType`），`repr` 是 `<function qualname at 0x..>`
- 持有指向生成的 Rust fn 的指针（统一签名的适配闭包，由 `rt::function!` 生成）、所属模块的 globals、闭包的 cell。通过 `__traverse__` 让 GC 看到它们，模块 dict 和函数之间的循环引用可以回收
- 元信息（`__name__`、`__qualname__`、`__doc__`、`__module__`、`__defaults__`、`__kwdefaults__`、`__annotations__`、`__annotate__`、`__type_params__`）放在 `Mutex` 里，都是可写的真实属性，赋值时做和 CPython 一样的类型检查（`__defaults__` 只接受 tuple 或 None 等）。M1 把它们放在实例 `__dict__` 里，但那样 `__defaults__` 无法做类型检查，调用时也要查 dict。`__dict__` 照常可写
- `__module__` 取自 globals 的 `__name__`，与 CPython 一致；只读的 `__globals__`、`__closure__`（与 CPython 一样是 cell 的 tuple，没有闭包时是 None）
- 生态工具需要的内省（M3）
  - `__signature__` 由存储的签名、当前的默认值和注解构造 `inspect.Signature`，`inspect.signature`、`help()` 都能用。`__dict__` 里有 `__wrapped__`（`functools.wraps` 的结果）时不提供，让 `inspect` 去追踪被包装的函数
  - `__reduce__` 返回 `__qualname__`，模块级函数按引用 pickle，`pickle.loads(pickle.dumps(f)) is f`，与 CPython 一样
- 嵌套函数没有 `__code__`，`isinstance(f, types.FunctionType)` 不成立，见第 12 节

### 签名与参数绑定

`Function` 的 `__call__` 按 CPython `initialize_locals` 的算法绑定参数：位置参数、关键字参数、`*args`、`**kwargs`、keyword-only、positional-only，缺省的参数从 `__defaults__`/`__kwdefaults__` 取。所有 `TypeError` 的文字与 CPython 一致，包括 `missing 2 required positional arguments: 'c' and 'd'`、`got some positional-only arguments passed as keyword arguments`、`takes 2 positional arguments but 3 positional arguments (and 1 keyword-only argument) were given`，以及 `Did you mean 'b'?` 的拼写建议（调用 CPython 自己的实现）。

注册和创建函数对象用两个宏，签名的写法和 Python 一样

```rust
rt::def!(globals, greet(name, greeting = "Hello", *, punct = "!"))?;
rt::def!(globals, span(start, stop = py.None(), /, step = 1))?;
rt::def!(globals, collect(first, *rest, sep = "-", **options))?;
let increment = rt::function!(
    globals,
    increment[&count](step = 1),
    qualname = "make_counter.<locals>.increment"
)?;
```

- `rt::function!` 返回函数对象，`rt::def!` 再把它绑定到 globals 里的 `__name__`
- 签名里 `/`、`*`、`*args`、`**kwargs` 与 Python 相同。默认值是 Rust 表达式，在宏展开处（也就是 `def` 执行时）按 Python 顺序求值一次。Rust 名字与 Python 名字不同的参数写成 `py_ as "py"`，报错信息和 `inspect.signature` 里显示 Python 名字
- 注解写成 `x: "int"`、`-> "str"`，是 Python 源码文本（见下面的注解一节）
- 方括号里是闭包捕获的变量，`&count` 是 cell，不带 `&` 的是值（见闭包一节）
- 可选的 `name = ".."`、`qualname = ".."`、`doc = ".."`。qualname 默认就是名字，嵌套函数是 `outer.<locals>.inner`，lambda 是 `<lambda>`
- 生成的 Rust fn 参数顺序：`py`、需要时的 `globals`、捕获的变量、Python 参数。`*args` 的类型是 `&Bound<'py, PyTuple>`，`**kwargs` 是 `&Bound<'py, PyDict>`，可以直接用 tuple/dict 的方法

### 直接调用与默认参数

模块级函数能在转换时绑定参数时直接调用 Rust fn（条件见第 7 节）：调用处没有 `*`/`**`，关键字参数都能对应到参数，并且不会因为参数不对抛 `TypeError`。否则通过函数对象调用，错误由 `Function` 抛出，文字和 CPython 完全一样。

```rust
// greet("Bob")
greet(
    py,
    &rt::str(py, "Bob"),
    &rt::default(globals, "greet", "greeting")?,
    &rt::default(globals, "greet", "punct")?,
)?
```

- 关键字参数按 Python 顺序求值，再按参数顺序传入，需要时先放进临时变量（第 8 节）
- 省略的参数用 `rt::default(globals, "greet", "greeting")?` 在调用时从函数对象读取，所以给 `__defaults__`/`__kwdefaults__` 赋值后直接调用也能看到新值，可变默认参数在直接调用和经过 Python 的调用之间共享
- 多出来的位置参数和关键字参数在调用处构造成 `PyTuple`/`PyDict` 传给 `*args`/`**kwargs`
- 需要读默认值的函数要接收 `globals` 参数

### 嵌套函数与 lambda

嵌套函数和 lambda 提升为具名 Rust fn，作为 item 写在外层函数体里 Python 定义的位置，紧接着用 `rt::function!` 创建函数对象并绑定局部变量。读代码时函数体和 Python 里的位置一致。

- Rust 名字就是 Python 名字，与外层函数的局部变量、临时变量或模块函数冲突时加 `_` 后缀（fn item 的名字和局部变量在同一个命名空间，`let show = rt::function!(globals, show_[&i]())?;`）。lambda 叫 `lambda`、`lambda2`
- 嵌套函数一律通过函数对象调用（`go.get()?.call1((..))`），不做直接调用。嵌套函数的名字常常在循环里重新绑定，直接调用的分析不值得
- 嵌套函数可以递归：函数名本身是外层函数的 cell，嵌套函数通过 cell 读到自己
- lambda 的函数体是一个尾表达式
- 嵌套函数的 docstring 同样生成 `///` 和 `__doc__`

### 闭包与 cell

作用域分析用标准库的 `symtable`，与 CPython 的分类完全一致（第 2 节的 scope 分析）。被内层函数捕获的局部变量（cell）分两种

- 捕获之后可能改变的变量用 `rt::Cell`：被 `nonlocal` 重新绑定的、捕获之后还会赋值的（例如循环变量）、捕获时可能还没赋值的。它包装 Python 的 cell 对象（`PyCell_New`），`__closure__` 看到的就是它。读写写成 `count.get()?`、`count.set(v)?`、`count.delete()?`，未赋值时抛和 CPython 一样的 `UnboundLocalError`（自己的局部变量）或 `NameError: cannot access free variable ..`（捕获的变量）。内层函数收到 `&rt::Cell<'py>` 参数
- 其余的（从不重新绑定的参数，以及只在捕获之前的顶层语句里赋值一次的变量）在外层函数里仍然是普通的 `Bound`，捕获时把当前值放进一个新的 cell，内层函数直接收到 `&Bound`。这类变量占大多数，生成代码和普通变量一样可读，语义也完全相同，因为捕获之后它的值不会再变

```rust
pub fn make_counter<'py>(
    py: Python<'py>,
    globals: &Bound<'py, PyDict>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let count = rt::Cell::new(py, "count", start)?;
    // closures.py:7
    fn increment<'py>(
        py: Python<'py>,
        count: &rt::Cell<'py>,
        step: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        count.set(rt::iadd(&count.get()?, step)?)?;
        count.get()
    }
    let increment = rt::function!(
        globals,
        increment[&count](step = 1),
        qualname = "make_counter.<locals>.increment"
    )?;
    ..
}
```

- 作为 cell 的变量不参与第 5 节的 `Option`/`mut` 处理，未赋值的状态由 cell 本身表示
- 循环里定义的闭包共享同一个 cell（late binding），`lambda i=i: ..` 用默认参数捕获当前值，都和 Python 一样

### 装饰器

按 Python 的顺序执行：从上到下求值装饰器表达式，再求值默认参数、创建函数对象，然后从下到上应用。装饰器先放进临时变量，每次应用一条语句，读起来和 Python 的顺序一致

```rust
let decorator = trace(py, globals, &rt::str(py, "outer"))?;
let decorator2 = trace(py, globals, &rt::str(py, "inner"))?;
let function = rt::function!(globals, greet(name, punct = ..), doc = "Say hello.")?;
let function = decorator2.call1((&function,))?;
globals.set_item("greet", decorator.call1((&function,))?)?;
```

被装饰的名字绑定的是装饰器的返回值，所以对它的调用一律走 globals 查找，不做直接调用。已验证 `functools.wraps`、`functools.lru_cache` 和用户自己的装饰器。

### 注解（PEP 649）

注解不影响生成的 Rust 类型（所有值仍是 `Bound<'py, PyAny>`），只是保存下来供运行时内省。3.14 的注解是惰性求值的，可以引用之后才定义的名字，所以注解以 Python 源码文本的形式写进签名（`width: "float"`），由 runtime 的 `__annotate__` 对象在需要时求值

- `__annotate__(1)`（VALUE）在模块 globals 里 `eval` 每个注解，得到和 CPython 一样的 `__annotations__`，结果缓存
- `__annotate__(3)`（FORWARDREF）对 `NameError` 的注解返回 `annotationlib.ForwardRef`，`__annotate__(4)`（STRING）返回源码文本，字符串字面量的注解返回它的值
- `from __future__ import annotations` 时注解就是字符串，与 CPython 一样
- 注解求值时只看得到模块 globals，引用外层函数局部变量的注解报 `UnsupportedError`

### 注释与源位置

Python 注释保留为 `//` 注释，放在它所属的语句前面：单独一行的注释属于后面的第一条语句，行尾注释属于它所在的语句（移到语句上方），复合语句头部和第一条子语句之间的注释属于那条子语句。模块末尾的注释放在 `exec_module` 末尾。每个函数（包括嵌套函数）前面保留 `// file.py:LINE` 源位置注释。

## 7. 模块、全局变量与 import

- 一个 Python 项目生成一个 crate，每个 Python 模块对应一个 Rust `mod`，模块间可以循环引用
- PyO3 0.30 使用多阶段初始化，执行顶层代码时模块已在 `sys.modules` 里，循环 import 能正常工作
- 包只导出一个 `PyInit_*`。包初始化时安装一个 meta path finder，子模块在被 import 时才执行，保持 Python 的惰性 import 顺序和副作用
- **全局变量不用进程级 static 保存模块对象**，因为 `importlib.reload` 或删除 `sys.modules` 后重新 import 会产生新的模块对象。globals 由 `Function` 持有并显式传给需要它的函数体
- 可以缓存 import 得到的标准库模块对象（`PyOnceLock`），但不能缓存它们的属性，否则 monkeypatch 失效
- **直接调用的条件**：名字在模块内只绑定一次（`def` 算一次），没有装饰器，模块里没有 `from x import *`、`globals()`、对自身模块的 `setattr`，并且调用的参数能在转换时绑定（第 6 节）。否则走 globals 字典动态查找。调用名为 `globals`、`vars`、`locals` 的函数时保守地按 `globals()` 处理，即使这个名字是模块自己定义的函数
- 直接调用无法感知其他模块的 `mock.patch("mod.fn")`，这是有意保留的语义差异（第 12 节）。应用场景里别的模块很少 patch 应用自身的函数，所以影响不大。以后如有需要再提供 `--dynamic-globals` 选项让所有全局调用都走动态查找
- 应用场景下 `importlib.reload` 也很少见，不用 static 保存模块对象主要是为了让同一份生成代码既能作为 `__main__` 运行，也能被别的模块按真实名字 import
- 函数里用 `global` 声明的名字照常走 globals 字典：读 `rt::global(globals, "x")?`，写 `globals.set_item("x", ..)?`，删 `rt::del_global(globals, "x")?`（不存在时抛 `NameError`）。这类函数需要 `globals` 参数

### import 语句（M2）

M2 支持标准库和第三方库的绝对 import，语义与字节码 `IMPORT_NAME`/`IMPORT_FROM` 一致：通过 `builtins.__import__` 导入并传入模块的 globals，所以替换了 `__import__` 的 hook 也能工作。生成代码如下

```rust
// import os.path
globals.set_item("os", rt::import(globals, "os.path")?)?;
// import collections.abc as cabc
globals.set_item("cabc", rt::import_module(globals, "collections.abc")?)?;
// from json import dumps, loads as parse
let json = rt::import_names(globals, "json", &["dumps", "loads"])?;
globals.set_item("dumps", rt::import_from(&json, "dumps")?)?;
globals.set_item("parse", rt::import_from(&json, "loads")?)?;
```

- 没有用 `py.import`：`PyImport_Import` 返回最内层的子模块而不是顶层包，也没有 `fromlist` 语义（`from pkg import submodule` 需要它导入子模块）
- `rt::import_from` 与 `IMPORT_FROM` 一样先 `getattr`，失败时回退到 `sys.modules["pkg.name"]`（循环 import），再失败时抛文字一致的 `ImportError`，并设置 `name`、`path`、`name_from`
- 函数内的 import 绑定局部变量，同样需要 `globals` 参数
- 相对 import 和 `from x import *` 暂不支持（M6）

## 8. 表达式与求值顺序

Rust 方法调用的形状会改变求值顺序，例如 `x in y` 写成 `y.contains(x)`、`o.m(f())` 写成 `o.call_method1("m", (f(),))`、`a[i] = f()` 写成 `a.set_item(i, f())`。规则如下

- 多个子表达式中有可能产生副作用的，按 Python 顺序先求值到临时变量。纯名字和字面量不需要临时变量，以保持可读
- 赋值语句先求值右边，再求值目标（`a[i] = f()` 是先 `f()`，再 `a`、`i`）
- 方法调用先 `getattr` 再求值参数（`o.missing(f())` 不应调用 `f`）
- 比较：在 `if`/`while` 条件里用 `lt` 等返回 bool 的方法；需要值时（`m = a < b`，numpy 之类会返回非 bool）用 `rich_compare`。链式比较中间操作数只求值一次
- 增量赋值：PyO3 没有 in-place 运算方法，用 runtime 的 `rt::iadd` 等（基于 `PyNumber_InPlaceAdd`），`a[i] += v`、`o.x += v` 中的 `a`、`i`、`o` 只求值一次
- `and`/`or`：值语义的 block 表达式，在条件里优化成 `&&`/`||`

### 实现（M2）

lowering 给每个子表达式记录它在 Rust 里的形态（`lower/expr.py` 的 `Value`）：拥有的 `Bound`、`&Bound` 参数、局部变量、可以直接作为 `IntoPyObject` 的字面量、代表 Python bool 的 Rust `bool`（`in`、`is`、`not`、`isinstance`），以及代表 int 的 Rust 整数（`len`）。每个使用点选最可读的写法，例如条件里直接用 Rust `bool`，`len(xs) > 3` 直接写 `xs.len()? > 3`。

临时变量是语句前面的 `let`（prelude）。只有 Rust 的求值顺序与 Python 不同、或者一个值要用两次时才引入：

- 只在操作数"不纯"时需要。纯的是局部变量、参数和字面量，它们没有副作用也不会失败。全局变量读取不纯（可能抛 `NameError`，也可能被前面的调用改掉）
- 某个操作数需要 prelude 时，Python 顺序里排在它前面的不纯操作数也一起提升到它的 prelude 之前（`operands`），否则 prelude 会抢先执行
- 临时变量尽量按表达式命名（`let item = ..`、`let index = ..`、全局变量或属性的名字），只使用一次的临时变量直接 move，不 clone
- 只在条件成立时执行的代码有自己的 prelude，放进对应的 block：`and`/`or` 的右侧、条件表达式的分支、链式比较后面的部分。`while` 条件需要 prelude 时生成 `loop { prelude; if !cond { break; } .. }`，`elif` 条件需要 prelude 时生成 `else { prelude; if cond { .. } }`

```rust
// data[trace("i", 1)] = trace("v", 5)
let value = trace(py, &rt::str(py, "v"), &rt::int(py, 5))?;
data.set_item(trace(py, &rt::str(py, "i"), &rt::int(py, 1))?, &value)?;
// g() in h()
let item = g.call0()?;
h.call0()?.contains(&item)?
```

具体写法

- 方法调用：参数都是纯的时写 `o.call_method1("m", (x, 1))?`，否则写 `o.getattr("m")?.call1((f()?,))?`，保证先查找方法
- `in`/`not in` 用 `contains`，`is` 用 `Bound::is`，与 `None` 比较用 `is_none()`。它们的结果是 Rust `bool`，需要对象时才 `rt::bool(py, ..)`
- `and`/`or` 在条件里是 `&&`/`||`，右侧有 prelude 时写成 block。需要值时生成 `if` 表达式，左操作数先放进临时变量，决定结果时直接 move 出来；左操作数是 Rust `bool` 时不需要临时变量。左操作数是常量时在转换时就决定。作为语句的 `a or f()` 生成 `if !a.is_truthy()? { f()?; }`
- 链式比较：`a < b < c` 在条件里是 `a.lt(&b)? && b.lt(c)?`；需要值时第一个比较结果放进临时变量，`if result.is_truthy()? { b.rich_compare(c, ..)? } else { result }`
- 一元运算用 PyO3 的 `neg`/`pos`/`bitnot`，`not` 是 `!cond`
- 增量赋值的 runtime 函数按 `operator` 模块命名：`rt::iadd`、`rt::itruediv`、`rt::ior` 等。`x += v` 写成 `x = rt::iadd(&x, v)?;`
- 容器字面量用 PyO3 的构造函数：`PyList::new(py, [1, 2, 3])?`、`PyTuple::new(..)`、`PySet::new(..)`、`[("a", 1)].into_py_dict(py)?`，空的用 `PyList::empty(py)`、`PyDict::new(py)`。元素都是同类型的字面量时直接写，否则统一写成 `&Bound`。所有元素先求值再构造，与 `BUILD_LIST`/`BUILD_MAP` 一致
- 下标 `get_item`/`set_item`/`del_item`，属性 `getattr`/`setattr`/`delattr`。切片用 `rt::slice(py, start, stop)?` 和 `rt::slice_step(..)`，省略的部分是 `py.None()`，因为 `PySlice::new` 只接受整数，而 Python 的切片对象保留 `None`
- 解包用 `rt::unpack`（固定个数，返回数组）和 `rt::unpack_star`（返回 `([前], 列表, [后])`），按 CPython 的 `UNPACK_SEQUENCE` 逐步迭代，错误信息一致
- f-string：每个字段是 `rt::format(&value, "spec")?`（`PyObject_Format`，没有 spec 时传 `""`），`!r`/`!s`/`!a` 是 `repr()?`/`str()?`/`rt::ascii(..)?`，嵌套的 spec 本身也是 f-string。多段时用 `rt::fstring(py, (a, " = ", b))?` 拼接（`"".join`），超过 12 段时嵌套
- `assert` 写成 `if !cond { return Err(PyAssertionError::new_err(msg)); }`，消息只在失败时求值
- 少数 builtin 有逐字等价的 PyO3 写法，直接使用：`len`、`isinstance`、`str`、`repr`、`bool`、`abs`、`type`（单参数）、`iter`（单参数），以及 M1 的 `print`。只在名字只可能是 builtin（模块里没有绑定它，也没有 `import *`/`globals()`）时生效，其余 builtin 走 `rt::builtin(py, "max")?` 动态查找。`range` 的特化留到后面

### 函数调用（M3）

CPython 先求值被调用的对象，再按源码顺序求值位置参数（包括 `*x`），然后按源码顺序求值关键字参数（包括 `**m`）。所以 `f(x, y=g(), *h())` 里 `h()` 在 `g()` 之前。lowering 按这个顺序排列参数

- 参数都是普通的位置参数时写 `f.call1((a, b))?`
- 有关键字参数或展开时用 `rt::call!`，写法接近 Python，宏按书写顺序求值

  ```rust
  rt::call!(r#fn, *args, **kwargs)?
  rt::call!(rt::global(globals, "k")?, 1, 2, "self" = 3, r#type = 4)?
  ```

  关键字是 Rust 标识符时直接写，否则写成字符串字面量。`*x` 与 `LIST_EXTEND` 一样迭代，不可迭代时抛 `Value after * must be an iterable, not int`；`**m` 与 `DICT_MERGE` 一样合并，重复的关键字抛 `got multiple values for keyword argument`，不是 mapping 时抛 `argument after ** must be a mapping`，文字都与 CPython 一致
- 直接调用的参数见第 6 节

## 9. 异常

- `try/except/finally` 用闭包返回 `PyResult<Flow>`，`Flow` 枚举传递块内的 return/break/continue
- **进入 handler 时必须设置"当前正在处理的异常"**，否则 handler 里 `sys.exception()` 为 None，新抛出的异常没有 `__context__`，`logging.exception()`、`traceback.print_exc()`、辅助函数里的裸 `raise` 都会出错（已验证）。runtime 提供 RAII guard，用 `PyErr_GetHandledException`/`PyErr_SetHandledException` 保存、设置、恢复
- handler 内的裸 `raise` 重新抛出绑定的异常
- `with` 显式调用 `__enter__`/`__exit__`，结构同 try/finally

## 10. 类

默认用动态建类，语义等同 `__build_class__`（`types.new_class`：metaclass、`__prepare__`、`__set_name__`、`__init_subclass__`），方法是第 6 节的 `Function`。

为了可读，每个类生成一个 Rust mod 放方法，加一个简短的建类函数

```rust
pub mod point {
    pub fn __init__<'py>(py: Python<'py>, slf: &Bound<'py, PyAny>, x: &Bound<'py, PyAny>) -> PyResult<()> { ... }
    pub fn norm<'py>(py: Python<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> { ... }
}
```

- 零参数 `super()` 降级为 `super(Cls, self)`，Cls 取建类后保存的类对象
- Python 3.14（PEP 649）下需要在命名空间里提供 `__annotations__`/`__annotate__`，dataclass 才能工作

不选 `#[pyclass]` 作为默认方案的原因（已验证）：不能继承 Python 定义的类（TestCase、Enum、NamedTuple、Thread 等），不支持 pickle/copy，和 Exception 混合继承会出现 layout conflict，类体内的逻辑、条件定义的类、局部类都无法表达。`#[pyclass]` 作为 M8 的可选升级路径，适用于只继承 object 或原生类型、且有类型注解的类，它更接近最终的 Rust 形态。

## 11. 输出模式

两种模式共用生成的 `exec_module(m)`，区别只在 crate 的入口。

- **bin 模式**（默认，主要场景）：嵌入 libpython 的可执行程序，`main.rs` 只调用 `rt::embed::run_main`
  - 不用 `auto-initialize`，用 `PyConfig` 初始化，设置 `executable`，否则 `sys.path` 里没有 `.venv` 的 site-packages。同时传入真实的 `argv`，`parse_argv = 0`
  - 以 `__main__` 身份执行模块。结束时调用 `Py_FinalizeEx`，这样会 flush stdout 并执行 atexit
  - `SystemExit` 和未捕获异常交给 `PyErr_Print` 处理，退出码与 CPython 一致
  - build.rs 设置指向构建时 libpython 的 rpath，避免运行时加载到系统的 libpython
  - M1 把构建时解释器的路径写死在二进制里，只适合在本机运行。分发到别的机器需要单独设计（见下）
  - 与 `python app.py` 一致的脚本信息（M2），在转换时确定并写进 `main.rs` 的 `rt::embed::Script`
    - `sys.argv[0]` 是传给 `steelix transpile` 的脚本路径，保持原样（相对路径仍是相对路径），后面接程序收到的参数。这样 `argparse` 的程序名、按 `argv[0]` 打印的用法说明都和原来一致，相当于在转换时的目录里执行 `python app.py`。二进制自己的真实命令行放在 `sys.orig_argv`
    - `sys.path[0]` 是脚本所在目录（解析符号链接），所以脚本旁边还没转换的 Python 模块照样能 import，这在 M6 之前很有用
    - `__main__` 的 `__file__` 是脚本的绝对路径（3.9 起 CPython 也是绝对路径），`__cached__` 是 `None`
    - 这些路径指向源码位置，和解释器路径一样只适合在本机运行，M7 再处理
  - 已知差异：`__main__` 的 `__loader__` 不是 `SourceFileLoader`
- **ext 模式**（`--mode ext`，次要场景）：cdylib 扩展模块，替换原 Python 模块

### 分发（待设计）

可执行程序在别的机器上需要找到 libpython、标准库和第三方依赖。可能的方案有：随程序附带 python-build-standalone 加上 venv 的 site-packages，运行时按相对于二进制的路径定位；或者静态链接 libpython 并把纯 Python 依赖打包成 zip 放进 `sys.path`。在 M7 做 spike 再定。

## 12. 名字卫生与已知语义差异

名字卫生，由一个确定性的重命名表统一处理

- Rust 关键字用 `r#`（`type`、`match`、`loop`、`ref`、`move`、edition 2024 保留的 `gen` 等），PyO3 会去掉参数名的 `r#`
- `self`、`super`、`crate` 不能做 raw identifier，`self` 改名为 `slf`
- 生成代码自己使用的名字和 Python 名字冲突时，改名 Python 名字，加 `_` 后缀直到不冲突。函数体内保留 `py`、`globals`、`_guard`、`Ok`、`Err`、`Some`、`None`；模块级函数名保留 `exec_module`，以及 `exec_module` 自己的局部变量 `m`、`py`、`globals`（名为 `m` 的 Python 函数生成 `fn m_`）。不需要改名的名字优先保留原拼写，改名不会抢占它们
- 嵌套函数的 Rust fn 和外层函数的局部变量、临时变量在同一个命名空间，冲突时同样加 `_` 后缀
- Python 可见的函数名由 `rt::def!` 去掉 `r#` 得到，改过名的用 `name = "..."` 显式给出。改过名的参数在签名里写成 `globals_ as "globals"`，报错信息、关键字参数和 `inspect.signature` 都用 Python 名字
- 含孤立代理（lone surrogate）的字符串不能写成 Rust `&str` 字面量，用 `rt::str_surrogatepass(py, b"...")?` 从 `surrogatepass` 编码的字节构造
- 字面量的具体写法：i32 范围内的整数直接写，超出的加 `i64`/`i128` 后缀，更大的用 `rt::bigint(py, "...")?`；浮点数用 `repr` 的最短往返表示，`inf` 写 `f64::INFINITY`；`None` 作参数时写 `py.None()`；`bytes` 一律用 `rt::bytes(py, b"...")`，不依赖 PyO3 对字节数组的隐式转换
- docstring 按 3.13+ 编译器的规则去缩进（与 `inspect.cleandoc` 不同，保留首尾空行），同时生成 `///` 和 `__doc__`

已知且有意保留的语义差异

- 直接调用的模块函数不受其他模块 monkeypatch 影响（可用 `--dynamic-globals` 关闭）
- traceback 里没有 Python 行号（缓解措施见第 14 节）
- 只有 C API 调用的热循环不经过解释器的检查点，Ctrl-C 需要 `PyErr_CheckSignals` 才能打断，PoC 阶段在循环里插入检查，或者先作为已知问题记录
- 同理，这类循环里 GIL 不会自动切换
- 直接调用的递归受 C 栈大小限制而不是 `sys.getrecursionlimit()`，`RecursionError` 的信息是 `Stack overflow (used N kB) ...` 而不是 `maximum recursion depth exceeded`
- 模块级代码里，`def` 执行之前的调用走 globals 查找（会得到 `NameError`）；但函数体里的直接调用不检查被调函数的 `def` 是否已经执行过
- `builtins.print` 等在参数求值之后才查找（Python 是先查找名字再求值参数），只有参数求值过程中替换了这个 builtin 才看得出区别
- 特化的 builtin（`len`、`isinstance`、`str`、`repr`、`bool`、`abs`、`type`、`iter`）直接调用对应的 C API，不在运行时查找 `builtins` 模块，所以 `builtins.len = ..` 这样的 monkeypatch 对它们无效。模块级重新绑定这些名字（`len = ..`、`def len`）则会被识别，不再特化
- 字面量每次求值都创建新对象，而 Python 的常量在同一段代码里共享。只有比较 `str`、大整数常量的 identity 时才看得出区别，例如 `d["k"] += 1` 里 `__getitem__` 和 `__setitem__` 收到的是两个相等但不同的 `"k"`
- f-string 中带转换但没有 spec 的字段（`{x!r}`）直接使用 `repr()`/`str()`/`ascii()` 的结果，CPython 对非精确 `str` 的结果还会再调用一次 `format`。只有 `__repr__`/`__str__` 返回带自定义 `__format__` 的 `str` 子类时才有区别
- `ImportError` 的文字只覆盖常见情况，CPython 对循环 import 中"partially initialized module"和模块名遮蔽标准库的特殊提示没有复刻。模块级 import 传给 `__import__` 的 `locals` 是 `None`（CPython 传模块的 dict）
- warning（例如 `DeprecationWarning`）的输出里没有源码行
- 函数对象是 `Function` 而不是 `types.FunctionType`：`isinstance(f, types.FunctionType)` 和 `inspect.isfunction(f)` 为 False，没有 `__code__`（依赖字节码的工具无法使用）。`__closure__` 存在，内容是真实的 cell。`inspect.signature`、`functools.wraps`、pickle 能正常使用（第 6 节）
- `del f.__defaults__` 等删除元信息属性的操作抛 `AttributeError`，CPython 是把它设为 None。赋值为 None 的行为一致
- 注解由源码文本 `eval` 得到。STRING 格式对非字符串字面量的注解直接返回源码文本，CPython 由 AST 重新生成，空白等写法可能不同；FORWARDREF 格式创建的 `ForwardRef` 没有 `owner`。`__annotate__` 对 FORWARDREF、STRING 格式直接返回结果，CPython 编译器生成的 `__annotate__` 只支持 VALUE，其余格式由 `annotationlib` 用假 globals 实现，通过 `annotationlib.get_annotations` 使用时没有区别
- 直接调用时省略的参数从 globals 里当前的函数对象读取。如果别的模块把这个名字换成了别的对象（monkeypatch），直接调用照常执行原来的 Rust 函数，读默认值时抛 `TypeError`。默认值被删掉（`__defaults__ = None`）后直接调用缺参数时，`TypeError` 只列出第一个缺少的参数，其余参数也已经求值
- 模块级的条件定义（`if` 里的 `def`）和重复定义暂不支持，报 `UnsupportedError`
- 嵌套函数的调用总是经过函数对象（性能差异，语义相同）

## 13. 不支持特性的处理

- 默认遇到不支持的语法就报错，诊断信息带 `文件:行:列` 和特性名，便于统计覆盖率
- **混合模式**：不支持的函数或类保留原 Python 源码嵌入 crate，在模块执行时编译并绑定到 globals。整个项目可以先跑起来，再逐步扩大可转换的范围。generator、async 适合用这种方式兜底。这是转换真实项目的前提，所以在 M5 前后实现

## 14. 测试与构建

- **单元测试**：rustast 打印、作用域与绑定分析等纯 Python 逻辑
- **Golden 快照**：`tests/cases/*.py` 转换结果与 `tests/golden/*.rs` 对比，`--update-golden` 更新。rustfmt 是必须的，固定 style edition，避免快照抖动
- **`cargo check` 层**：只检查 golden 能编译，比端到端快。同时要求生成代码没有编译警告（crate 根上只放 `non_snake_case`、`unused_variables`、`unused_assignments` 三个 allow。最后一个是因为 Python 代码里常见赋值后不再读取的写法，`del x` 生成的 `x = None;` 也是这种情况），把可读性问题挡在快照阶段
- 所有用例生成到 `target/e2e/` 下同一个 cargo workspace 里一次构建，`CARGO_TARGET_DIR` 共用，PyO3 只编译一次；workspace 复用仓库的 `Cargo.lock`，离线也能解析到同一个 PyO3 版本
- rustfmt 的配置随包附带（`rustast/rustfmt.toml`，edition 与 style edition 都是 2024），用 `--config-path` 显式指定，不受运行目录影响，并复制到生成的 crate 里
- **端到端差分测试**：比较 `python case.py` 与生成的可执行程序的 stdout、退出码，以及 stderr 最后一行（异常类型和消息，不比较 traceback）。ext 模式只保留少量冒烟测试（`python -c "import case"`）。用 pytest marker `e2e` 区分
- **`PYO3_PYTHON` 必须显式设置**。`.cargo/config.toml` 只对仓库内的构建生效，生成到仓库外的 crate 会悄悄用系统 Python 构建
- 生成的 `Cargo.toml` 依赖 `steelix-runtime` 的方式：测试里用 path 依赖，发布后用固定版本
- 源码映射：函数和语句前的 `// file.py:LINE` 注释，异常经过时用 `add_note` 附加 Python 位置。对迁移的人来说行号很重要，优先级不低
- CI：GitHub Actions 跑 ruff、pytest（含 e2e），后续补上

## 15. 待定问题

1. 参数传递用 `&Bound` 还是 `Bound`。初版用 `&Bound`，如果生成的 clone 太多再调整
2. ABI 策略：按版本编译还是 abi3。runtime 里的 ffi 调用尽量限制在 limited API 内，保留 abi3 的可能
3. free-threaded（3.14t）是否支持，暂不考虑
