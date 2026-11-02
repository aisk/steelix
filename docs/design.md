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
- generator、async/await、match、metaclass 自定义、`exec`/`eval` 等复杂特性的原生转换。用到它们的代码通过混合模式保留为 Python 源码照常运行（第 13 节）

## 2. 总体流水线

```
.py 源码
  │  frontend/parse.py     ast.parse + tokenize 收集注释
  ▼
Python AST + 注释
  │  frontend/subset.py    找出不支持的构造，所在的顶层语句保留为 Python（§13）
  │  frontend/scope.py     symtable 分析作用域（local/global/nonlocal/cell/free）
  │  frontend/bindings.py  自行遍历 AST，统计模块级名字的绑定次数，
  │                        局部变量的定值分析（definite assignment）
  ▼
带作用域信息的 AST
  │  lower/*               逐节点降级，按 Python 求值顺序生成临时变量，
  │                        保留的语句生成 rt::exec_python!
  ▼
Rust AST（rustast/nodes.py）
  │  rustast/verify.py     检查作用域、跳转标签、带类型的接收者
  │  rustast/printer.py    渲染成源码，再强制过一遍 rustfmt
  ▼
.rs 源码
  │  emit/*                生成 Cargo.toml、lib.rs、每个 Python 模块一个 Rust mod
  ▼
Cargo crate
```

选择先构建一个小型 Rust AST 再打印，而不是直接拼字符串。这样 lowering 和格式化解耦，也方便做局部的可读性改写（例如去掉多余的 clone、合并 `?`）。Rust AST 只覆盖生成代码用得到的部分，不追求完整。

打印之前先跑一遍检查器（`rustast/verify.py`，M3a），发现问题就报内部错误，而不是写出编译不过或语义错误的代码。它检查三类容易在 lowering 里犯、review 也不容易看出来的错误：

- 每个路径和局部变量都在作用域内。嵌套的 fn item 看不到外层函数的局部变量，块里的 item 在整个块内可见。例如条件里一个 block 声明的临时变量在下一个 block 里使用，或者循环头里的 lambda 对应的 fn item 被放进了循环体
- `break`/`continue` 的标签存在，`continue` 的标签是循环；不带标签的跳转不能跨出带标签的 block（rustc E0695）
- 带具体类型的值（`Bound<PyDict>` 等）不能调用被该类型自己的方法遮蔽的 `PyAnyMethods` 方法（见 §8）

类型只从参数类型和生成代码里少数几种产生具体类型的表达式推出来，其余都当作 `PyAny`。宏参数里不是表达式的部分（签名、`key = value`）用 `Tokens` 节点表示：文本和表达式交错，对局部变量和 item 的引用必须放在表达式部分里，文本部分不检查。Rust AST 里没有任意文本节点。检查器很便宜（所有用例加起来十几毫秒），所以每次转换都会运行。

## 3. 目录结构

```
steelix/
├── pyproject.toml             uv 项目，Python 3.14
├── Cargo.toml                 Rust workspace，成员为 crates/*
├── .cargo/config.toml         仓库内构建时 PYO3_PYTHON 指向 .venv/bin/python
├── CLAUDE.md                  给后续会话的约定与命令速查
├── docs/
│   ├── design.md              本文档
│   ├── roadmap.md             里程碑与任务拆解
│   └── coverage.md            真实应用的转换覆盖率与优先级建议
├── src/steelix/
│   ├── cli.py                 命令行入口 `steelix transpile`
│   ├── coverage.py            转换覆盖率报告
│   ├── diagnostics.py         诊断信息、源码位置、UnsupportedError
│   ├── frontend/              解析、作用域分析、绑定分析
│   ├── lower/                 Python AST → Rust AST
│   ├── rustast/               Rust AST 定义、打印、rustfmt
│   └── emit/                  生成完整 crate
├── crates/
│   └── steelix-runtime/       生成代码依赖的运行时（Function、异常 guard 等）
├── examples/                  示例应用：转换的原样输出和手工改写的版本（第 15 节）
└── tests/
    ├── cases/                 端到端用例（*.py），同时作为 golden 输入
    ├── apps/                  从标准库复制的真实命令行工具（混合模式）
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
pub fn fib<'py>(m: &rt::Module<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _guard = rt::RecursionGuard::enter(m.py())?;
    rt::frame!(m, "fib", 1, |frame| {
        frame.line(2);
        if n.lt(2)? {
            return Ok(n.clone());
        }
        frame.line(4);
        fib(m, &n.sub(1)?)?.add(fib(m, &n.sub(2)?)?)
    })
}

/// Executes the body of `fib.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        rt::def!(m, fib(n))?;
        frame.line(7);
        rt::builtins::print(py, (fib(m, &rt::int(py, 10))?,))?;
        Ok(())
    })
}
```

要点

- 所有 Python 值默认都是 `Bound<'py, PyAny>`，运算直接用 PyO3 的 `add`、`lt`、`getattr`、`call` 等方法
- 函数体和模块体包在 `rt::frame!` 里，可能出错的语句前用 `frame.line(N);` 记下 Python 行号，异常的 traceback 因此有原生函数的帧（见第 9 节）
- 函数体是普通的 Rust fn，第一个参数是模块上下文 `m: &rt::Module<'py>`，Python 参数用 `&Bound`。`m.py()` 给出 `Python<'py>`，函数体里用到不止一次时开头写 `let py = m.py();`，只用一次时就地写 `m.py()`
- 模块体生成 `exec_module(m)`，只依赖传入的模块上下文，与输出模式无关
- 模块级变量的读写走 globals 字典：`m.global("LIMIT")?` 先查 globals 再查 builtins，找不到抛 `NameError`；写入用 `m.set_global("LIMIT", 20)?`。模块里没有绑定的名字视为 builtin，`print` 用 `rt::builtins::print`，`len(x)` 等少数几个直接写成 PyO3 方法（见第 8 节），其余用 `rt::builtin(py, "max")?`
- 比较在条件里用 `lt` 等返回 bool 的方法，需要值时用 `rich_compare(.., CompareOp::Lt)`
- 函数末尾的 `return` 写成尾表达式，返回值本身是 `PyResult` 时不再包一层 `Ok(..?)`
- 带 `else` 且有 `break` 的循环包在带标签的 block 里，`break` 写成 `break 'for_else`，正常结束时执行 `else`。`else` 里的 `break`/`continue` 属于外层循环，要跨出这个 block，rustc 不允许不带标签的跳转跨出带标签的 block（E0695），所以只在这时给外层循环加标签 `'outer: for ..`，写成 `continue 'outer`。其余循环都不带标签
- 对 Python 可见的函数对象由 runtime 的 `Function` 包装（见第 6 节），注册只占一行：`rt::def!(m, fib(n))?`，签名的写法和 Python 一样（`greet(name, greeting = "Hello", *, punct = "!")`）。Rust 名字与 Python 名字不同时加 `name = "..."`，有 docstring 时加 `doc = "..."`
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
- 函数顶层第一次出现的 `import` 也直接 `let os = rt::import(m, "os")?;`
- 被内层函数捕获并且之后会改变的变量（cell）用 `rt::Cell`，不参与上面的处理，见第 6 节
- `try` 体、`finally` 之前的部分和 `with` 体在立即调用的闭包里执行（第 9 节），闭包不能初始化外层 fn 里声明了但还没赋值的变量，所以（`frontend/regions.py`、`frontend/bindings.py`）
  - 只在一个闭包区域内出现的局部变量在闭包里声明，规则同上（区域顶层第一次赋值处 `let`，否则在闭包开头声明），每次执行闭包都从未赋值开始
  - 在闭包里赋值、但在外面也用到的变量，进入闭包时必须已经确定赋值，否则是 `Option`
  - 只有一个表达式的 `try`（`x = int(s)`）不需要闭包，`match` 之后才赋值，`x` 仍然是普通的 `Bound`
  - `except .. as e` 的名字只在 handler 里使用时是 handler 里的 `let`，`with .. as f` 的名字只在这些 `with` 里使用时是闭包参数

## 6. 函数对象、闭包与默认参数

`wrap_pyfunction!` 得到的是 `builtin_function_or_method`，已验证它有以下问题：`functools.wraps` 无法写 `__name__`，不能设置属性，放进类里不会绑定 `self`。`PyCFunction::new_closure` 也有同样的问题，并且要求 `Send + Sync + 'static`，无法捕获 `Bound`。

因此 runtime 提供 `#[pyclass(frozen, dict, weakref)] Function`

- 实现 `__call__`（按存储的签名描述绑定参数）和 `__get__`（像 Python 函数一样绑定方法，返回 `types.MethodType`），`repr` 是 `<function qualname at 0x..>`
- 持有指向生成的 Rust fn 的指针（统一签名的小闭包，由 `rt::function!` 生成，把绑定好的参数逐个传给 Rust fn）、所属模块的 globals、闭包的 cell。通过 `__traverse__` 让 GC 看到它们，模块 dict 和函数之间的循环引用可以回收
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
rt::def!(m, greet(name, greeting = "Hello", *, punct = "!"))?;
rt::def!(m, span(start, stop = py.None(), /, step = 1))?;
rt::def!(m, collect(first, *rest, sep = "-", **options))?;
let increment = rt::function!(
    m,
    increment[&count](step = 1),
    qualname = "make_counter.<locals>.increment"
)?;
```

- `rt::function!` 返回函数对象，`rt::def!` 再把它绑定到 globals 里的 `__name__`。第一个参数是模块上下文，函数对象保存它的 globals，经过 Python 调用时再用它构造 `rt::Module` 传给 Rust fn
- 签名里 `/`、`*`、`*args`、`**kwargs` 与 Python 相同。默认值是 Rust 表达式，在宏展开处（也就是 `def` 执行时）按 Python 顺序求值一次。Rust 名字与 Python 名字不同的参数写成 `py_ as "py"`，报错信息和 `inspect.signature` 里显示 Python 名字
- 注解写成 `x: "int"`、`-> "str"`，是 Python 源码文本（见下面的注解一节）
- 方括号里是闭包捕获的变量，`&count` 是 cell，不带 `&` 的是值（见闭包一节）
- 可选的 `name = ".."`、`qualname = ".."`、`doc = ".."`。qualname 默认就是名字，嵌套函数是 `outer.<locals>.inner`，lambda 是 `<lambda>`
- 生成的 Rust fn 参数顺序：`m`、捕获的变量、Python 参数。`*args` 的类型是 `&Bound<'py, PyTuple>`，`**kwargs` 是 `&Bound<'py, PyDict>`，可以直接用 tuple/dict 的方法

### 直接调用与默认参数

模块级函数能在转换时绑定参数时直接调用 Rust fn（条件见第 7 节）：调用处没有 `*`/`**`，关键字参数都能对应到参数，并且不会因为参数不对抛 `TypeError`。否则通过函数对象调用，错误由 `Function` 抛出，文字和 CPython 完全一样。

```rust
// greet("Bob")，greeting 的默认值是 "Hello"，punct 的默认值是 [] 之类的可变对象
greet(m, &rt::str(py, "Bob"), &rt::str(py, "Hello"), &m.default("greet", "punct")?)?
```

- 关键字参数按 Python 顺序求值，再按参数顺序传入，需要时先放进临时变量（第 8 节）
- 省略的参数如果默认值是不可变的字面量（`None`、bool、数字、字符串、bytes），并且模块里没有给 `__defaults__`/`__kwdefaults__` 赋值（也没有以字符串形式写出这两个名字，例如 `setattr`），直接在调用处写成字面量
- 其余默认值用 `m.default("greet", "punct")?` 在调用时从函数对象读取，所以给 `__defaults__`/`__kwdefaults__` 赋值后直接调用也能看到新值，可变默认参数在直接调用和经过 Python 的调用之间共享
- 多出来的位置参数和关键字参数在调用处构造成 `PyTuple`/`PyDict` 传给 `*args`/`**kwargs`

### 嵌套函数与 lambda

嵌套函数和 lambda 提升为具名 Rust fn，作为 item 写在外层函数体里 Python 定义的位置，紧接着用 `rt::function!` 创建函数对象并绑定局部变量。读代码时函数体和 Python 里的位置一致。

- Rust 名字就是 Python 名字，与外层函数的局部变量、临时变量或模块函数冲突时加 `_` 后缀（fn item 的名字和局部变量在同一个命名空间，`let show = rt::function!(m, show_[&i]())?;`）。lambda 叫 `lambda`、`lambda2`
- 嵌套函数一律通过函数对象调用（`go.get()?.call1((..))`），不做直接调用。嵌套函数的名字常常在循环里重新绑定，直接调用的分析不值得
- 嵌套函数可以递归：函数名本身是外层函数的 cell，嵌套函数通过 cell 读到自己
- lambda 的函数体是一个尾表达式
- `if`、`elif`、`while`、`for` 头部表达式里的 lambda，其 fn item 放在整个语句之前，而不是第一个分支或循环体里，否则头部看不到它
- 嵌套函数的 docstring 同样生成 `///` 和 `__doc__`

### 闭包与 cell

作用域分析用标准库的 `symtable`，与 CPython 的分类完全一致（第 2 节的 scope 分析）。被内层函数捕获的局部变量（cell）分两种

- 捕获之后可能改变的变量用 `rt::Cell`：被 `nonlocal` 重新绑定的、捕获之后还会赋值的（例如循环变量）、捕获时可能还没赋值的。它包装 Python 的 cell 对象（`PyCell_New`），`__closure__` 看到的就是它。读写写成 `count.get()?`、`count.set(v)?`、`count.delete()?`，未赋值时抛和 CPython 一样的 `UnboundLocalError`（自己的局部变量）或 `NameError: cannot access free variable ..`（捕获的变量）。内层函数收到 `&rt::Cell<'py>` 参数
- 其余的（从不重新绑定的参数，以及只在捕获之前的顶层语句里赋值一次的变量）在外层函数里仍然是普通的 `Bound`，捕获时把当前值放进一个新的 cell，内层函数直接收到 `&Bound`。这类变量占大多数，生成代码和普通变量一样可读，语义也完全相同，因为捕获之后它的值不会再变

```rust
pub fn make_counter<'py>(
    m: &rt::Module<'py>,
    start: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let count = rt::Cell::new(py, "count", start)?;
    // closures.py:7
    fn increment<'py>(
        m: &rt::Module<'py>,
        count: &rt::Cell<'py>,
        step: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        count.set(rt::iadd(&count.get()?, step)?)?;
        count.get()
    }
    let increment = rt::function!(
        m,
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
let decorator = trace(m, &rt::str(py, "outer"))?;
let decorator2 = trace(m, &rt::str(py, "inner"))?;
let function = rt::function!(m, greet(name, punct = ..), doc = "Say hello.")?;
let function = decorator2.call1((&function,))?;
m.set_global("greet", decorator.call1((&function,))?)?;
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
- **全局变量不用进程级 static 保存模块对象**，因为 `importlib.reload` 或删除 `sys.modules` 后重新 import 会产生新的模块对象。globals 由 `Function` 持有，包装成模块上下文 `rt::Module` 显式传给每个函数体。以后需要按模块保存的状态（例如混合模式里编译好的 Python 代码）也放在这里
- **只绑定一次的全局名字仍然每次查 globals**（M3b 评估过）。`import os`、`LIMIT = 10` 这类名字在模块里只绑定一次（`ModuleBindings` 能数出来），可以考虑在函数里缓存它们的值，但没有这样做
  - 主要原因是缓存在可读性上没有收益：有了模块上下文，`m.global("LIMIT")?` 已经和 Python 的写法一一对应，缓存只会让生成代码多出缓存槽。函数被改写成原生 Rust 时，常量自然变成 Rust 的 `const`（第 15 节）
  - 动态查找顺带让替换全局变量（`monkeypatch.setattr(app, "LIMIT", 1)`、`mock.patch("app.requests")`）照常生效，函数在名字绑定之前被调用时也照常抛 `NameError`。但这不是完整的 monkeypatch 支持：对模块函数的直接调用本来就看不到替换（下面和第 12 节），所以不能拿它作为不缓存的理由
  - 缓存而又保持语义的做法是用 dict watcher（`PyDict_AddWatcher`）在 globals 或 builtins 变化时让缓存失效，但每个使用点需要一个缓存槽（生成代码里多一个宏或 static），每个解释器最多只有 8 个 watcher，实现和审查的成本都高
  - 性能上，实测一次全局读取 44 ns，CPython 特化后的 `LOAD_GLOBAL` 约 1.5 ns。其中大部分是每次从 `&str` 新建并哈希一个 Python 字符串，所以 `m.global` 和 `rt::builtin` 只接受 `'static` 的名字，按地址缓存驻留字符串，降到 18 ns，语义不变。剩下的差距等专门做性能时再考虑 dict watcher
- **直接调用的条件**：名字在模块内只绑定一次（`def` 算一次），没有装饰器，模块里没有 `from x import *`、`globals()`、对自身模块的 `setattr`、不带命名空间的 `exec`/`eval`，并且调用的参数能在转换时绑定（第 6 节）。否则走 globals 字典动态查找。调用名为 `globals`、`vars`、`locals` 的函数时保守地按 `globals()` 处理，即使这个名字是模块自己定义的函数。带参数的 `vars(obj)`、`locals(..)` 读的是别的命名空间，不算
- **对自身模块的修改**（M3d，`ModuleBindings.patches_itself`）按常见写法保守地识别，原生代码和保留为 Python 的代码都检查：
  - 模块对象是 `sys.modules[__name__]`、`sys.modules["__main__"]`（也包括 `.get(..)`、`from sys import modules` 之后的 `modules[..]`）、`import __main__` 绑定的名字，以及被赋值为这些表达式的名字（不分作用域，传递）
  - 对模块对象 `setattr`/`delattr`、给它的属性赋值或删除属性、读它的 `__dict__`、`vars(模块对象)`
  - 第二个参数是字符串字面量、并且是模块里某个 `def` 的名字的 `setattr`/`delattr`，不管第一个参数是什么
  - 只有一个参数的 `exec`/`eval`（`globals()` 作命名空间时已经按 `globals()` 处理）
  - 识别不到的写法见第 12 节
- 直接调用无法感知其他模块的 `mock.patch("mod.fn")`，这是有意保留的语义差异（第 12 节）。应用场景里别的模块很少 patch 应用自身的函数，所以影响不大。以后如有需要再提供 `--dynamic-globals` 选项让所有全局调用都走动态查找
- 应用场景下 `importlib.reload` 也很少见，不用 static 保存模块对象主要是为了让同一份生成代码既能作为 `__main__` 运行，也能被别的模块按真实名字 import
- 函数里用 `global` 声明的名字照常走 globals 字典：读 `m.global("x")?`，写 `m.set_global("x", ..)?`，删 `m.del_global("x")?`（不存在时抛 `NameError`）
- globals 里没有的名字和 `LOAD_GLOBAL` 一样到 `builtins` 模块的命名空间（它的 dict）里找（`rt::builtin`），不是读 `builtins` 模块对象的属性。M3d 之前用的是 `getattr`，`__annotations__`、`__dict__` 这类每个模块对象都有的属性会被当成 builtin 读到，CPython 是 `NameError`

### import 语句（M2）

M2 支持标准库和第三方库的绝对 import，语义与字节码 `IMPORT_NAME`/`IMPORT_FROM` 一致：通过 `builtins.__import__` 导入并传入模块的 globals，所以替换了 `__import__` 的 hook 也能工作。生成代码如下

```rust
// import os.path
m.set_global("os", rt::import(m, "os.path")?)?;
// import collections.abc as cabc
m.set_global("cabc", rt::import_module(m, "collections.abc")?)?;
// from json import dumps, loads as parse
let json = rt::import_names(m, "json", &["dumps", "loads"])?;
m.set_global("dumps", rt::import_from(&json, "dumps")?)?;
m.set_global("parse", rt::import_from(&json, "loads")?)?;
```

- 没有用 `py.import`：`PyImport_Import` 返回最内层的子模块而不是顶层包，也没有 `fromlist` 语义（`from pkg import submodule` 需要它导入子模块）
- `rt::import_from` 与 `IMPORT_FROM` 一样先 `getattr`，失败时回退到 `sys.modules["pkg.name"]`（循环 import），再失败时抛文字一致的 `ImportError`，并设置 `name`、`path`、`name_from`
- 函数内的 import 绑定局部变量
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

`Value` 还记录值的具体类型（`typed`），例如容器字面量是 `PyList`，`*args` 是 `PyTuple`，`str(x)` 是 `PyString`，`iter(x)` 是 `PyIterator`。这很重要：`Bound<PyDict>` 除了 `PyAnyMethods` 还有 `PyDictMethods`，同名时 Rust 优先选后者，而 `PyDict::get_item` 在 key 不存在时返回 `None`，不抛 `KeyError`。`Bound<PyIterator>` 实现了 `Iterator`，`eq`、`lt` 等会解析成 `Iterator` 的方法。所以生成的方法调用都经过 `Value.method(name, ..)`，它查 `rustast/pyo3.py` 的表，被遮蔽且行为不同的方法先 `.as_any()` 擦除类型。行为完全相同的（精确类型上的 `contains`、`len`，dict 的 `set_item`、`del_item`）保留原写法，其中 `len` 不会失败，写成 `args.len()`，不带 `?`。

临时变量是语句前面的 `let`（prelude）。只有 Rust 的求值顺序与 Python 不同、或者一个值要用两次时才引入：

- 只在操作数"不纯"时需要。纯的是局部变量、参数和字面量，它们没有副作用也不会失败。全局变量读取不纯（可能抛 `NameError`，也可能被前面的调用改掉）
- 某个操作数需要 prelude 时，Python 顺序里排在它前面的不纯操作数也一起提升到它的 prelude 之前（`operands`），否则 prelude 会抢先执行
- 临时变量尽量按表达式命名（`let item = ..`、`let index = ..`、全局变量或属性的名字），只使用一次的临时变量直接 move，不 clone
- 只在条件成立时执行的代码有自己的 prelude，放进对应的 block：`and`/`or` 的右侧、条件表达式的分支、链式比较后面的部分。`while` 条件需要 prelude 时生成 `loop { prelude; if !cond { break; } .. }`，`elif` 条件需要 prelude 时生成 `else { prelude; if cond { .. } }`

```rust
// data[trace("i", 1)] = trace("v", 5)
let value = trace(m, &rt::str(py, "v"), &rt::int(py, 5))?;
data.set_item(trace(m, &rt::str(py, "i"), &rt::int(py, 1))?, &value)?;
// g() in h()
let item = g.call0()?;
h.call0()?.contains(&item)?
```

具体写法

- 方法调用：参数都是纯的时写 `o.call_method1("m", (x, 1))?`，否则写 `o.getattr("m")?.call1((f()?,))?`，保证先查找方法
- `in`/`not in` 用 `contains`，`is` 用 `Bound::is`，与 `None` 比较用 `is_none()`。它们的结果是 Rust `bool`，需要对象时才 `rt::bool(py, ..)`
- `and`/`or` 在条件里是 `&&`/`||`，右侧有 prelude 时写成 block。需要值时生成 `if` 表达式，左操作数先放进临时变量，决定结果时直接 move 出来；左操作数是 Rust `bool` 时不需要临时变量。左操作数是常量时在转换时就决定。作为语句的 `a or f()` 生成 `if !a.is_truthy()? { f()?; }`
- 链式比较：`a < b < c` 在条件里是 `a.lt(&b)? && b.lt(c)?`；中间操作数需要临时变量时，它和之后所有用到它的比较放在同一个 block 里：`a.lt(b)? && { let rhs = f(py)?; b.lt(&rhs)? && rhs.lt(d)? }`。需要值时第一个比较结果放进临时变量，`if result.is_truthy()? { b.rich_compare(c, ..)? } else { result }`
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
  rt::call!(m.global("k")?, 1, 2, "self" = 3, r#type = 4)?
  ```

  关键字是 Rust 标识符时直接写，否则写成字符串字面量。`*x` 与 `LIST_EXTEND` 一样迭代，不可迭代时抛 `Value after * must be an iterable, not int`；`**m` 与 `DICT_MERGE` 一样合并，重复的关键字抛 `got multiple values for keyword argument`，不是 mapping 时抛 `argument after ** must be a mapping`，文字都与 CPython 一致
- 直接调用的参数见第 6 节

## 9. 异常（M4）

目标是让最常见的写法读起来像手写的 Rust，少数几种固定的结构覆盖其余情况。运行时的部分在 `crates/steelix-runtime/src/frame.rs`，生成的部分在 `lower/handling.py`。

### 帧与行号

原生函数运行时没有 Python 帧。和 Cython 一样，异常离开函数时再把这个函数补进 traceback

- 每个函数体包在 `rt::frame!(m, "qualname", def 所在行, |frame| { .. })` 里，模块体用 `<module>`。`RecursionGuard` 在它外面。函数体里不可能出错的（没有 `?`）不包
- 每条可能出错的语句前写 `frame.line(N);`，只是写一个 `Cell<u32>`。复合语句的 marker 写在头部（`if`、`while` 的条件，`for` 的迭代），循环体正常走完时把行号设回循环头，和 CPython 在 `JUMP_BACKWARD` 处的行号一致。不会出错的语句、`break`/`continue`、和上一条行号相同的语句不写
- 异常离开函数（或者到达 handler、`finally`、`with` 的 `__exit__`）时，`Frame` 用 `PyCode_NewEmpty` 和 `code.replace(co_qualname=..)` 创建 code 对象（每个函数只创建一次，`PyOnceLock` 缓存），`PyFrame_New` 创建帧，再用 `types.TracebackType(tb_next, frame, -1, line)` 插到 traceback 前面。同一次调用只创建一个帧对象，异常已经带有这个帧的条目时（在这里 `raise`，或者裸 `raise` 重新抛出）不再重复添加，和 CPython 一样
- 多行语句的行号取值所在的那一行（`return`、赋值、表达式语句取值的第一行），traceback 只显示这一行源码。`tb_lasti` 是 -1，所以原生帧没有 `^^^` 定位，这是和 CPython traceback 唯一的文字差异
- 可读性的取舍：marker 让每条语句多一行，但它是显式的、和 Python 源码一一对应，手工修改 Rust 时也容易维护。示例（`total_size`，第 24 行起）

  ```rust
  rt::frame!(m, "total_size", 24, |frame| {
      let mut text: Bound<'py, PyAny>;
      let mut total = rt::int(py, 0);
      frame.line(26);
      for path in paths.try_iter()? {
          let path = path?;
          frame.line(27);
          text = read_first_line(m, &path)?;
          frame.line(28);
          total = rt::iadd(&total, text.len()?)?;
          py.check_signals()?;
          frame.line(26);
      }
      Ok(total)
  })
  ```

- 否决的方案
  - 由 `#[track_caller]` 或 Rust 源码行号反查 Python 行号：rustfmt 和手工修改都会打乱对应关系
  - proc macro 改写函数体，或者把每个 `?` 改写成带行号的形式：生成的代码不再是普通 Rust，也增加编译时间
  - 为每条语句生成伪造的行号表（`co_linetable`），可以得到多行显示和 `^` 定位，但要在生成代码里放一张静态位置表，噪音太大
  - 在调用期间压入真正的解释器帧：没有公开 API，见下面的 logging
- 每次调用的代价是 `frame!` 里几个字段的初始化和每条语句一次 `Cell::set`，`fib(27)` 和 `for i in range(n): total += i` 在 release 下测不出差别（M4 前后都是 21 ms 和 130 ms）
- logging、warnings 仍然看不到原生帧。它们读的是线程状态里正在执行的解释器帧（`sys._getframe`、`PyEval_GetFrame`），原生函数运行时这个链上没有它，而在调用期间伪造一个解释器帧没有公开 API（`_PyInterpreterFrame` 是内部结构，`f_lineno` 只能在 trace 函数里设置），所以只记录为已知差异（第 12 节）

### raise

- `raise X` 写成 `return Err(frame.raise(X));`，`raise X from Y` 写成 `frame.raise_from(X, Y)`，`from None` 传 `py.None()`。`X` 可以是类（会实例化，构造结果不是 `BaseException` 实例时抛 `TypeError`）、实例、或者 `PyErr`，`__cause__`、`__context__`、`__suppress_context__`、`__traceback__` 都和 CPython 一致（`__context__` 由 `PyErr_SetObject` 设置）
- builtin 异常类加字面量参数直接写成 PyO3 的写法：`frame.raise(PyValueError::new_err("bad value"))`。`assert` 同样写成 `frame.raise(PyAssertionError::new_err(..))`
- 裸 `raise` 写成 `frame.reraise()`，重新抛出当前正在处理的异常（`PyErr_GetHandledException`），所以在 handler 调用的辅助函数里也能用。没有正在处理的异常时抛 `RuntimeError: No active exception to reraise`

### try/except

两种结构。第一种（下面叫 A）是 `try` 里只有一条语句，形如 `x = expr`、`return expr` 或者 `expr`，并且 handler 不需要异常对象：直接对表达式的结果 `match`

```rust
frame.line(3);
let int = rt::builtin(py, "int")?;
match int.call1((s,)) {
    Ok(value) => x = value,
    Err(err) if err.is_instance_of::<PyValueError>(py) => x = rt::int(py, 0),
    Err(err) => return Err(err),
}
```

- 表达式里查找 builtin 或全局名字的 `?` 提到 `match` 前面（`let int = ..;`），这样 scrutinee 里没有 `?`。还有别的 `?` 时写成 `(|| expr)()`
- `else` 的语句放在 `Ok` 分支里
- handler 的类型是 builtin 异常时写成 `err.is_instance_of::<PyX>(py)`，元组写成几个 `||`。其余写 `rt::matches(&err, &handler)?`，handler 不是异常类时和 CPython 一样抛 `TypeError`
- 最后一个分支 `Err(err) => return Err(err)` 把没有匹配的异常传出去

其余的（B）把 `try` 体放进立即调用的闭包，`match` 闭包的结果。闭包里的 `?` 被 `match` 接住，`return`/`break`/`continue` 由闭包返回 `rt::Flow`

```rust
match (|| -> PyResult<()> {
    frame.line(65);
    ..
    Ok(())
})() {
    Ok(()) => {}
    Err(err) if rt::matches(&err, &m.global("ValueError")?)? => {
        let e = frame.catch(err);
        let _handling = rt::Handling::new(&e);
        frame.line(67);
        ..
    }
    Err(err) => return Err(err),
}
```

- 闭包的返回类型按块里的跳转决定：没有跳转是 `PyResult<()>`；只有 `return` 并且结尾不可达时直接返回值，写 `return Ok(..)`；其余是 `PyResult<rt::Flow<'py>>`，`Flow` 有 `Next`、`Break`、`Continue`、`Return(value)`。外面按 `Flow` 分派，只有一种跳转时写成 `if let rt::Flow::Return(value) = ..? { return Ok(value); }`，否则写 `match`
- handler 的 `let e = frame.catch(err);` 把这一帧补进 traceback，得到异常对象。不用 `as` 也要设置正在处理的异常时写 `let _handling = rt::Handling::new(&frame.catch(err));`。handler 只有 `pass`、赋值字面量这类不会调用 Python 代码的语句时省略这两行
- `rt::Handling` 是 RAII guard：创建时用 `PyErr_GetHandledException`/`PyErr_SetHandledException` 保存并设置正在处理的异常，`Drop` 时恢复。所以 handler 里 `sys.exception()`、`logging.exception()`、`traceback.print_exc()`、新异常的 `__context__`、辅助函数里的裸 `raise` 都和 CPython 一样，从 `?`、`return`、`break`、panic 离开时也会恢复
- `as e` 的名字只在 handler 内使用时就是 handler 里的 `let`，离开 handler 自然消失，这就是 CPython 结尾的隐式 `del e`。模块级的名字写入 globals，handler 结束时 `m.unbind_global("e")?`。函数里 handler 之外也用到这个名字的，所在的函数保留为 Python（`except-as name used outside its handler`，标准库里只有 1 处）
- `except*` 保留为 Python（§13），异常组的拆分和重新组合不值得用 Rust 再实现一遍

### finally

```rust
let result = (|| -> PyResult<rt::Flow<'py>> {
    frame.line(12);
    if item.eq(target)? {
        return Ok(rt::Flow::Return(item.clone()));
    }
    Ok(rt::Flow::Next)
})();
let finally = frame.finally(result);
frame.line(15);
rt::builtins::print(py, ("checked", &item))?;
if let rt::Flow::Return(value) = finally.resume()? {
    return Ok(value);
}
```

- `try`（连同 `except`、`else`）的结果先存进 `result`，`frame.finally(result)` 在结果是异常时把这一帧补进 traceback，并在 `finally` 块执行期间把它设为正在处理的异常。`finally` 块是外层的普通代码，它自己的 `return`、`break`、`continue`、`raise` 就是外层的跳转，自然覆盖 `try` 里的 `return` 或者吞掉异常，和 CPython 一样。`finally` 块总是跳走时不会走到 `resume`，变量写成 `_finally`
- `finally.resume()?` 在块正常结束时继续 `try` 的结果：重新抛出异常（不再添加 traceback 条目），或者按 `Flow` 跳转

### with

```rust
frame.line(20);
if let rt::Flow::Return(value) =
    frame.with(&rt::builtin(py, "open")?.call1((path,))?, |f| {
        frame.line(21);
        Ok(rt::Flow::Return(f.call_method0("readline")?))
    })?
{
    return Ok(value);
}
```

- `frame.with(manager, |value| body)` 在类型上查找 `__enter__`、`__exit__`（`_PyObject_LookupSpecial` 的规则，找不到时的报错和 CPython 一样，包括"支持异步上下文管理协议"的提示），调用 `__enter__`，执行 body，最后调用 `__exit__`：正常结束传三个 `None`，异常时传异常并设为正在处理的异常，返回真值时吞掉异常。`__exit__` 在 `with` 所在的行执行
- `as` 的目标是只在这个 `with` 里使用的局部名字时，直接是闭包参数 `|f|`；元组、属性、全局变量等目标在闭包开头赋值。多个 item 写成嵌套的 `frame.with`
- body 的跳转和 `try` 一样用 `Flow`。`async with` 保留为 Python

### 改写成原生代码时的约定

手工改写的函数（第 15 节）对 Python 一侧抛出的异常类型要和原来一样，调用方的 `except` 才能照常工作。原生的错误用 Rust 的 `Result`/`Option` 表达，在 adapter 里转换成原来的 Python 异常，例如 `examples/logsum` 的 `parse_line` 把 `native::NotANumber` 转成和 `int()` 一样的 `ValueError`，生成的 `load` 照常用 `except ValueError` 接住

## 10. 类

默认用动态建类，语义等同 `__build_class__`（`types.new_class`：metaclass、`__prepare__`、`__set_name__`、`__init_subclass__`），方法是第 6 节的 `Function`。

为了可读，每个类生成一个 Rust mod 放方法，加一个简短的建类函数

```rust
pub mod point {
    pub fn __init__<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>, x: &Bound<'py, PyAny>) -> PyResult<()> { ... }
    pub fn norm<'py>(m: &rt::Module<'py>, slf: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> { ... }
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
  - `SystemExit` 和未捕获异常交给 `PyErr_Print` 处理，退出码与 CPython 一致。之前和 CPython 的 `flush_io` 一样先 flush `sys.stderr` 和 `sys.stdout`（忽略错误），否则 stdout 和 stderr 重定向到同一个文件时，缓冲区里的输出会排在 traceback 后面
  - 生成的 `Cargo.toml` 在 dev profile 下把 pyo3、pyo3-ffi 和 runtime 设为 `opt-level = 3`，生成的 crate 本身设为 1。没有优化的 PyO3 让每个 Python 操作慢几倍；不用 `package."*"`，因为它也会优化 proc macro，让干净构建的时间翻倍。实测（M3a）一个递归加循环的基准：全部 `opt-level = 0` 时 1.39s，现在的配置 0.40s，而 CPython 是 0.18s；干净构建从 11.5s 增加到 15.3s，改一行后重新构建从 0.5s 增加到 1.0s。转换出来的代码每个操作仍然经过 C API，没有字节码解释的开销但也没有 CPython 的特化和内联缓存，在函数被改写成原生 Rust 之前不会比 CPython 快
  - 只有 crate 是 workspace 根时 profile 才生效，所以只在独立 crate（默认）里生成
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
- 生成代码自己使用的名字和 Python 名字冲突时，改名 Python 名字，加 `_` 后缀直到不冲突。函数体内保留 `py`、`_guard`、`frame`、`_handling`、`Ok`、`Err`、`Some`、`None`；模块级函数名保留 `exec_module`、`m`、`module`、`py`、`frame`，它们是生成的函数的局部变量（名为 `m` 的 Python 函数生成 `fn m_`）。不需要改名的名字优先保留原拼写，改名不会抢占它们
- 模块上下文参数叫 `m`，和 `#[pymodule]` 函数的模块参数一样。函数里有名为 `m` 的 Python 局部变量（常见的是正则的 match 对象）时，保留局部变量的名字，这个函数的上下文参数改叫 `module`
- 嵌套函数的 Rust fn 和外层函数的局部变量、临时变量在同一个命名空间，冲突时同样加 `_` 后缀
- Python 可见的函数名由 `rt::def!` 去掉 `r#` 得到，改过名的用 `name = "..."` 显式给出。改过名的参数在签名里写成 `py_ as "py"`，报错信息、关键字参数和 `inspect.signature` 都用 Python 名字
- 含孤立代理（lone surrogate）的字符串不能写成 Rust `&str` 字面量，用 `rt::str_surrogatepass(py, b"...")?` 从 `surrogatepass` 编码的字节构造
- 字面量的具体写法：i32 范围内的整数直接写，超出的加 `i64`/`i128` 后缀，更大的用 `rt::bigint(py, "...")?`；浮点数用 `repr` 的最短往返表示，`inf` 写 `f64::INFINITY`；`None` 作参数时写 `py.None()`；`bytes` 一律用 `rt::bytes(py, b"...")`，不依赖 PyO3 对字节数组的隐式转换
- docstring 按 3.13+ 编译器的规则去缩进（与 `inspect.cleandoc` 不同，保留首尾空行），同时生成 `///` 和 `__doc__`

已知且有意保留的语义差异

- 直接调用的模块函数不受其他模块 monkeypatch 影响（可用 `--dynamic-globals` 关闭）
- 原生函数的 traceback 帧是异常离开时合成的（第 9 节）。和 CPython 的区别：原生帧没有 `^^^` 定位（`tb_lasti` 是 -1），多行语句只显示取值所在的一行，`f_locals` 为空。保留为 Python 的代码（§13）的帧和 CPython 完全一样
- 行号的细节：`rt::def!` 执行出错（例如装饰器抛异常）时报告的是上一条语句的行号；`continue` 之前检查信号时的行号是上一条执行的语句；只有一个表达式的 `try`（第 9 节的 A）里查找 builtin 和全局名字提前到 `match` 之前，只有这个查找本身出错（`NameError`）时才看得出顺序不同
- 编译期的 `SyntaxWarning`（例如 3.14 PEP 765 对 `finally` 里 `return` 的警告）由 CPython 在编译时输出，二进制程序没有编译 Python 源码这一步，不输出
- 合成的帧只在异常经过时存在，运行期间解释器的帧链上没有原生函数，所以按调用方的帧工作的标准库功能看到的是离它最近的 Python 帧（`tests/cases/frames_logging.py` 记录了现在的输出）
  - `logging` 的 `%(filename)s`、`%(lineno)d`、`%(funcName)s` 指向 logging 自己的帧，例如 `__init__.py:2191 warning` 而不是 `app.py:8 step`
  - `warnings.warn` 的位置是调用这个原生函数的保留代码所在的行；整条调用链都是原生代码时是 `<sys>:0`
  - 从调用方帧推断模块名的 API：三个参数的 `type(name, bases, dict)` 创建的类没有 `__module__`，`enum.Enum("C", "A B")` 这样的函数式 API 得到 `<unknown>`。`collections.namedtuple` 找不到帧时用 `__main__`，bin 模式的主模块碰巧是对的
  - 要修好需要在原生函数运行期间压入真正的解释器帧，CPython 没有公开 API（第 9 节）
- 原生的 `while`、`for` 循环每次迭代结尾（以及 `continue` 之前）调用 `py.check_signals()?`（`PyErr_CheckSignals`，M3d 加入，M4 从迭代开头移到结尾，traceback 的行号因此和 CPython 一样是最后执行的语句），对应 CPython 在循环回跳处检查信号，所以 Ctrl-C 能打断只调用 C API 的循环，抛出 `KeyboardInterrupt`。它同时会运行已经排上的循环垃圾回收，和解释器的检查点一样。代价实测每次迭代 3 到 5 ns（release，`for i in range(n): total += i` 从 36 ns 到 42 ns，空循环从 6.3 ns 到 9.7 ns），只在几乎什么都不做的循环里明显。bin 模式下未捕获的 `KeyboardInterrupt` 打印 traceback，finalize 之后和 CPython 一样用 `SIGINT` 结束进程（shell 看到的是被中断，`subprocess` 看到的退出码是 `-2`）
- 原生循环里 GIL 不会像解释器那样定期切换，其他 Python 线程要等到原生代码调用会释放 GIL 的 API 时才能运行
- 直接调用的递归受 C 栈大小限制而不是 `sys.getrecursionlimit()`，`RecursionError` 的信息是 `Stack overflow (used N kB) ...` 而不是 `maximum recursion depth exceeded`
- 模块级代码里，`def` 执行之前的调用走 globals 查找（会得到 `NameError`）；但函数体里的直接调用不检查被调函数的 `def` 是否已经执行过
- `builtins.print` 等在参数求值之后才查找（Python 是先查找名字再求值参数），只有参数求值过程中替换了这个 builtin 才看得出区别
- 特化的 builtin（`len`、`isinstance`、`str`、`repr`、`bool`、`abs`、`type`、`iter`）直接调用对应的 C API，不在运行时查找 `builtins` 模块，所以 `builtins.len = ..` 这样的 monkeypatch 对它们无效。模块级重新绑定这些名字（`len = ..`、`def len`）则会被识别，不再特化
- 字面量每次求值都创建新对象，而 Python 的常量在同一段代码里共享。只有比较 `str`、大整数常量的 identity 时才看得出区别，例如 `d["k"] += 1` 里 `__getitem__` 和 `__setitem__` 收到的是两个相等但不同的 `"k"`
- f-string 中带转换但没有 spec 的字段（`{x!r}`）直接使用 `repr()`/`str()`/`ascii()` 的结果，CPython 对非精确 `str` 的结果还会再调用一次 `format`。只有 `__repr__`/`__str__` 返回带自定义 `__format__` 的 `str` 子类时才有区别
- `ImportError` 的文字只覆盖常见情况，CPython 对循环 import 中"partially initialized module"和模块名遮蔽标准库的特殊提示没有复刻。模块级 import 传给 `__import__` 的 `locals` 是 `None`（CPython 传模块的 dict）
- warning（例如 `DeprecationWarning`）的输出里没有源码行
- 对自身模块的修改（第 7 节）只识别常见写法。识别不到的有：`importlib.import_module(__name__)`、`__import__(__name__)`、`getattr(sys, "modules")` 等其他取得模块对象的方式，经过参数、容器、属性传递的模块对象，ext 模式下按模块的真实名字 `import` 自己，`exec`/`eval` 的命名空间是别处得到的模块 dict，以及其他模块里的修改（`mock.patch("app.f")`）。这些情况下直接调用看不到替换
- 函数对象是 `Function` 而不是 `types.FunctionType`：`isinstance(f, types.FunctionType)` 和 `inspect.isfunction(f)` 为 False，没有 `__code__`（依赖字节码的工具无法使用）。`__closure__` 存在，内容是真实的 cell。`inspect.signature`、`functools.wraps`、pickle 能正常使用（第 6 节）
- `del f.__defaults__` 等删除元信息属性的操作抛 `AttributeError`，CPython 是把它设为 None。赋值为 None 的行为一致
- 注解由源码文本 `eval` 得到。STRING 格式对非字符串字面量的注解直接返回源码文本，CPython 由 AST 重新生成，空白等写法可能不同；FORWARDREF 格式创建的 `ForwardRef` 没有 `owner`。`__annotate__` 对 FORWARDREF、STRING 格式直接返回结果，CPython 编译器生成的 `__annotate__` 只支持 VALUE，其余格式由 `annotationlib` 用假 globals 实现，通过 `annotationlib.get_annotations` 使用时没有区别
- 直接调用时省略的参数，不可变字面量的默认值直接写在调用处，所以别的模块给这个函数的 `__defaults__`/`__kwdefaults__` 赋值时直接调用看不到，这和直接调用看不到 monkeypatch 是同一类差异。本模块里的赋值会被识别，这时改为读函数对象（第 6 节）。字面量每次调用都创建新对象，见上面关于常量 identity 的一条
- 其余省略的参数从 globals 里当前的函数对象读取。如果别的模块把这个名字换成了别的对象（monkeypatch，或者 `sys.modules[__name__].f = g`），直接调用照常执行原来的 Rust 函数，读默认值时抛 `TypeError: f() was replaced by <function g at 0x..>, but transpiled code calls the original directly and can not take the default of its parameter 'b' from the replacement`。CPython 会调用替换后的函数，要做到一样，直接调用就得在每次调用前检查 globals，失去了直接调用的意义，所以只保证报错清楚。M3c 之前这里会 panic默认值被删掉（`__defaults__ = None`）后直接调用缺参数时，`TypeError` 只列出第一个缺少的参数，其余参数也已经求值
- 模块级的条件定义（`if` 里的 `def`）和重复定义（第二个及以后的）保留为 Python（§13），条件定义所在的 `if` 本身是原生代码
- 模块级只有一个参数的 `exec(src)`、`eval(src)` 在原生代码里显式传入模块的 globals（`exec(src, globals())`），和 CPython 在模块级的行为一样。函数里的要读写局部变量，所在的函数保留为 Python
- 模块级不带参数的 `globals()`、`locals()`、`vars()` 都是模块的 globals 字典，`dir()` 是排好序的名字列表（`m.dir()?`），和 CPython 一样。函数里不带参数的 `locals()`、`vars()`、`dir()` 要读局部变量，所在的函数保留为 Python
- 混合模式（§13）的差异：跟在 `;` 后面的保留语句，第一行在 traceback 里的列位置会偏；ext 模式下文件名用模块的 `__file__`，也就是扩展模块的路径，traceback 显示不出源码行
- 模块级注解（§13 的"模块注解"）：`__annotate__` 的结果和 CPython 一样，剩下的差异是
  - 一段保留代码执行期间，`__annotate__` 和 `__conditional_annotations__` 是这一段自己的（只有这一段的注解），执行完才换回整个模块的。只有同一段代码里读注解（或者它调用的函数读模块的 `__annotations__`）时才看得出来
  - `__annotate__` 里注解的列位置：每条注解从第 0 列开始，原来缩进的（`if` 里的）或者跟在 `;` 后面的注解求值出错时，traceback 里 `^` 的位置会偏。`__annotate__` 函数对象每段保留代码执行时会重新绑定一次，比较它的 identity 时和 CPython 不同
  - 有 `from __future__ import annotations` 时，CPython 在模块开头创建的 `__conditional_annotations__` 要等第一段带注解的保留代码执行时才有（这种模式下它不被使用）
- Rust 的 panic（手工改写的代码里的 bug，或者 steelix 的 bug）在 bin 模式下和未捕获的 Python 异常一样处理：打印 `pyo3_runtime.PanicException` 的 traceback，退出码是 1，而不是 Rust 默认的 101。生成的代码和 runtime 不应该因为 Python 一侧可见的状态 panic，这类错误都转成 Python 异常
- 嵌套函数的调用总是经过函数对象（性能差异，语义相同）

## 13. 不支持特性的处理：混合模式（M3c）

目标是 CPython 能运行的应用都能转换，并且行为一致。steelix 还不能降级的部分保留为 Python 源码，在模块的 globals 里执行。整个程序先跑起来，再逐步扩大原生转换的范围，覆盖率报告（下面）告诉我们下一步该支持什么。

- 默认就是混合模式。`steelix transpile --no-fallback` 恢复以前的行为：遇到不支持的语法就报错，诊断信息带 `文件:行:列` 和特性名
- 粒度是模块作用域里的语句。模块的作用域就是 globals 字典，所以不支持的模块级语句（类、`try`、`with`、模块级的推导式、条件定义的函数、带注解的赋值等）可以就地从源码执行，求值顺序和 Python 一致。包含不支持语法的顶层 `def`（generator、`try`、推导式、嵌套的类等）整个保留为 Python，嵌套函数一起保留
- 模块级的 `if`、`for`、`while`（包括 `if __name__ == "__main__":`）会拆开（M3d）：只有体里的某些语句不支持时，只保留这些语句，`if`/循环本身和其余语句仍是原生代码。拆分是递归的，体里的 `if`/`for`/`while` 同样处理。不拆的情况：不支持的构造在条件、循环变量或被迭代的表达式里，整个语句保留；保留的语句里有跳出原生循环的 `break`/`continue`（单独编译是 `SyntaxError`），整个循环保留，这是最简单而且正确的做法，保留的循环自己的 `break`/`continue` 不受影响。保留的语句之间不用同步任何东西，它们读写的都是 globals
- 原生函数里的一部分语句保留为 Python（更细的粒度）暂不做：那需要在 Rust 局部变量和 Python 代码之间同步局部变量。记录保留哪些语句的数据结构（`frontend/fallback.py` 的 `FallbackPlan`）按语句节点索引，现在已经放进模块级复合语句里的语句，以后可以直接放进函数里的语句

### 判定

- `frontend/subset.py` 遍历整个模块，记录每个不支持的构造（特性名、位置、所在的最外层函数，即模块级 `def` 或类的方法），按顶层语句分组。遇到不支持的节点不停下，继续检查它的子节点，所以一个函数里所有的阻碍都会被统计。类的方法也照常检查，用来估计支持类之后还有什么挡着
- 少数不支持的情况要到分析或 lowering 时才发现（函数里不带参数的 `locals()`/`vars()`/`dir()`/`exec()`/`eval()`、嵌套函数的注解引用外层局部变量、超过参数个数上限的调用）。这时 lowering 抛出 `UnsupportedError`，按出错的位置找到所在的顶层语句，和它里面已经保留的语句一起按上面的规则重新拆分，然后从头重新分析和 lowering，因为保留一个 `def` 会改变别的函数能否直接调用它。正常情况下最多重试几次
- 同名的模块级 `def` 出现多次时，第一个原生转换，后面的保留为 Python（两个 Rust 函数需要不同的名字）。名字绑定了不止一次，所以调用都走 globals

### 对分析的影响

- 保留为 Python 的 `def` 不是直接调用的目标（`ModuleBindings.direct_function` 不返回它），对它的调用一律走 globals 查找。它的名字照常计入绑定次数，所以它和别的定义同名时，原生函数也不会被直接调用
- 保留的语句里的绑定照常统计。推导式里的 `:=` 绑定外层作用域（PEP 572），在模块级就是全局变量，同样计入，否则原生代码可能把同名的名字当成 builtin
- 作用域分析（symtable）只链接原生语句里的函数，跳过保留的语句（包括复合语句里的），保留的语句里可能有类和推导式，它们的符号表结构不同
- 模块级的可达性分析把保留的语句看作不透明的普通语句：执行完继续往下

### 生成的代码

每个保留的复合语句（`def`、`class`、`try`、`with`、整个保留的循环等）单独一个 `rt::exec_python!` 调用，只有连续的简单语句合在一起（M3d 之前连续的保留语句都合成一段，一个 `exec_python!` 里可能有几百行、好几个函数和类）。源码写成 raw string，原样放在它执行的位置，前面一行注释给出源位置和保留的原因（特性和位置）。下面节选自 `tests/golden/mixed_mode.rs`：

```rust
    rt::def!(m, log(event))?;
    log(m, &rt::str(py, "native statement before the class"))?;
    // mixed_mode.py:17, kept as Python: class definition at 17:1
    rt::exec_python!(
        m,
        line = 17,
        r#"
class Counter:
    """Kept as Python: classes are not lowered yet."""

    instances = log("class body")

    def __init__(self, start):
        self.value = start

    def bump(self):
        # Python calling a native function.
        self.value = double(self.value)
        return self
"#
    )?;
    rt::def!(m, double(x))?;
```

拆开的 `if` 里保留的语句保持原来的缩进：

```rust
    if m.global("FLAG")?.is_truthy()? {
        // mixed_mode.py:62, kept as Python: function definition outside module top level at 62:5
        rt::exec_python!(
            m,
            line = 62,
            r#"
    def pick():
        return "first"
"#
        )?;
    } else {
```

- raw string 从 `r#"` 的下一行开始，`line = 17` 是第一行在 Python 文件里的行号。源码里出现 `"#` 时自动加更多的 `#`
- 语句之间、保留的语句前面的注释仍然生成 Rust 注释；保留的语句内部的注释留在源码里。复合语句和带装饰器的语句总是从行首开始；跟在 `;` 后面的简单语句只截取它自己
- 复合语句体里的语句保留原来的缩进，运行时在它前面一行补一个 `if True:` 再编译，所以列位置（traceback 里的 `^`）也和文件一致
- 模块有 `from __future__ import annotations` 时写成 `rt::exec_python!(m, line = 17, future_annotations, r#"..."#)`，编译时带上对应的 flag
- 生成的模块文件开头有一段模块文档，列出所有保留为 Python 的语句和原因，读代码的人一眼就能看出哪些已经是原生代码
- 检查器（§2）认识这个宏：参数必须是模块上下文、`line = N`、可选的 `future_annotations` 或 `first_annotation = I`（见下面的模块注解）和 raw string，源码必须从新的一行开始，并且按这个行号（缩进的源码按上面的 `if True:`）能用 `compile()` 编译

运行时 `rt::python::exec_python` 在源码前面补 `line - 1` 个空行，用 `Py_CompileStringObject` 以模块的 `__file__` 为文件名编译，再用 `PyEval_EvalCode` 在模块的 globals 里执行（globals 和 locals 是同一个 dict，和模块级代码一样）。所以

- 保留的 `def` 和 `class` 定义的对象和 CPython 完全一样：`__module__` 来自 globals 的 `__name__`，`__qualname__`、`__code__.co_filename`、`co_firstlineno` 都是原来的，`inspect.getsource` 能读到源码
- 异常经过保留的代码时，traceback 里是原来的文件名和行号，并且能显示源码行
- 原生代码和 Python 代码共享同一个 globals：Python 代码调用原生函数（`rt::def!` 绑定在 globals 里），原生代码通过 `m.global(..)` 调用 Python 定义的函数和类，两个方向都可以

### 模块注解

CPython 3.14 把模块里所有的注解编译成一个 `__annotate__` 函数，在模块的第一条语句之前绑定，并用 `__conditional_annotations__` 集合记录哪些带注解的赋值已经执行过（按注解在模块里的序号），`__annotate__` 只返回执行过的。模块级的带注解赋值现在都保留为 Python，分在几段单独编译的代码里，每段都会绑定只有自己注解的 `__annotate__`，序号也从 0 开始。所以（M3d）

- `exec_module` 开头调用 `rt::module_annotate(m, &[(8, "A: int"), (16, "B: list[str]")])?`，把所有注解按原来的行号编译成模块的 `__annotate__`，和 CPython 一次编译整个模块得到的一样，`__conditional_annotations__` 是空集合
- 含有模块级注解的保留代码写成 `rt::exec_python!(m, line = 16, first_annotation = 1, r#"..."#)`，`first_annotation` 是这一段第一个注解在模块里的序号。执行完把这一段记下的序号加上它，并入模块的集合，再换回模块的 `__annotate__` 和集合
- 有 `from __future__ import annotations` 时没有 `__annotate__`，每条带注解的赋值往模块的 `__annotations__` 字典里加一项，几段代码共用同一个字典，只需要在开头创建它（CPython 也是在开头创建）
- 只有名字的注解（`x: int`）算数，`(x): int`、`a.b: int` 不进 `__annotate__`。剩下的差异见第 12 节

### 手工把保留的函数改成原生代码

保留为 Python 的 `def` 和原生函数的接口相同：都是 globals 里的一个名字，调用方都经过它。所以在 steelix 支持对应的语法之前，也可以手工把它改写成 Rust，步骤是

1. 在模块文件里写一个 `pub fn`，签名用适配层的约定：`fn name<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>, ..) -> PyResult<Bound<'py, PyAny>>`，`*args` 是 `&Bound<'py, PyTuple>`，`**kwargs` 是 `&Bound<'py, PyDict>`（照着同一个文件里生成的原生函数写）。函数体可以先照抄 steelix 对类似代码的生成方式，也可以直接按第 15 节写原生逻辑
2. 在 `exec_module` 里原来的位置，把 `rt::exec_python!(..)?;` 换成 `rt::def!(m, name(x, y = ..))?;`，签名写 Python 的参数列表，有默认值的参数用 `rt::def!` 的写法（同样照着生成的例子）。装饰过的函数用 `rt::function!` 创建函数对象，交给装饰器，再用 `m.set_global` 绑定（见 `tests/golden/decorators.rs`）
3. 删掉 raw string 里的 Python 源码和文件开头模块文档里对应的一行，跑端到端测试

注意两点：这样改写的函数仍然只能通过 globals 调用，其他原生函数里对它的调用仍是 `m.global("name")?.call1(..)`，要直接调用就把这些调用点也改成 `name(m, ..)?`，和第 15 节"调用方"一样；改写过的模块文件从此由人维护，不能再用 steelix 覆盖。

### 覆盖率报告

`steelix transpile` 结束时在 stderr 打印一段摘要，`-v` 列出每个不支持构造的位置，`-q` 不打印，`--report FILE` 写出 JSON（`coverage.py`），可以跨模块合并（`coverage.merge`），用来决定先支持什么

```
stdlib_calendar.py: native 10/72 (14%) functions, 28/42 (67%) module statements, 200/838 (24%) lines
  kept as Python by     functions  only  statements  occurrences
  class definition             57    28          14           14
  list comprehension           12     0           2           13
```

- 统计单位：函数是模块级 `def` 和类的方法（嵌套函数算在外层函数里），语句是除 `def` 以外的顶层语句，另外按行数统计
- 每个特性的列：`functions` 是它出现在其中的函数个数（类出现时算类的所有方法），`only` 是只被它挡住的函数个数（支持它就能让这些函数变成原生代码），`statements` 是函数以外的顶层语句个数，`occurrences` 是出现次数
- JSON 里还列出每个保留的函数和语句以及挡住它的特性。真实应用的数据和据此提出的优先级见 `coverage.md`

## 14. 测试与构建

- **单元测试**：rustast 打印、作用域与绑定分析等纯 Python 逻辑
- **Golden 快照**：`tests/cases/*.py` 转换结果与 `tests/golden/*.rs` 对比，`--update-golden` 更新。rustfmt 是必须的，固定 style edition，避免快照抖动
- **`cargo check` 层**：只检查 golden 能编译，比端到端快。同时要求生成代码没有编译警告（crate 根上只放 `non_snake_case`、`unused_variables`、`unused_assignments` 三个 allow。最后一个是因为 Python 代码里常见赋值后不再读取的写法，`del x` 生成的 `x = None;` 也是这种情况），把可读性问题挡在快照阶段
- 所有用例生成到 `target/e2e/` 下同一个 cargo workspace 里一次构建，`CARGO_TARGET_DIR` 共用，PyO3 只编译一次；workspace 复用仓库的 `Cargo.lock`，离线也能解析到同一个 PyO3 版本
- rustfmt 的配置随包附带（`rustast/rustfmt.toml`，edition 与 style edition 都是 2024），用 `--config-path` 显式指定，不受运行目录影响，并复制到生成的 crate 里
- **端到端差分测试**：比较 `python case.py` 与生成的可执行程序的 stdout、退出码和 stderr。有 traceback 时比较完整的 traceback（M4 起原生函数也有帧，第 9 节），只去掉原生帧没有的 `^^^` 行和 CPython 编译时的 `SyntaxWarning`；少数用例（直接调用的递归，§12）只比较异常类型。未捕获异常的用例另外用 `2>&1` 比较输出顺序。ext 模式只保留少量冒烟测试（`python -c "import case"`）。用 pytest marker `e2e` 区分
  - 文档化的差异里有 CPython 成功而生成的程序报错的情况（§12 里被替换的直接调用）。这类用例用 `# steelix-test: expect-error 消息` 注释写明期望的错误，测试检查输出是 CPython 输出的前缀，并且最后一行是这条消息
  - stdout 里文档化的差异（§12 里原生函数运行时没有解释器帧对 `logging`、`warnings` 的影响）写在 `tests/test_e2e.py` 的 `KNOWN_DIFFERENCES` 里：CPython 的哪一行在生成的程序里应该是什么样子（正则），其余行必须相同。差异消失时测试失败，提醒删掉这一项
  - 带 `# steelix-test: interrupt after the first line of output` 的用例会一直运行，测试在它输出第一行之后发送 `SIGINT`，两边都必须以 `KeyboardInterrupt` 结束，退出码相同
  - 混合模式：保留为 Python 的代码的 traceback 帧必须和 CPython 的最后几帧完全相同（文件名、行号、源码行）。bin 模式下的 panic 必须以退出码 1 结束（用手写的模块文件替换生成的文件来触发）
- **真实应用**：`tests/apps/` 是从本机 CPython 3.14 标准库原样复制的命令行工具（calendar、base64、tabnanny、tokenize、ast、json.tool，文件开头注明来源和 PSF 许可），以混合模式转换，在几组命令行参数下与 CPython 运行同一份副本比较 stdout、退出码和 stderr（没有 traceback 时完全相同；有 traceback 时比较每一帧的文件名、行号、函数名和第一行源码，以及最后一行和 traceback 之前的输出）。它们的覆盖率数据见 `coverage.md`
- **语料**：`tests/corpus/` 下是模仿真实小程序的脚本（文本处理、argparse、路径处理、JSON、图算法、计算器等），只用已支持的子集，但把很多特性组合在一起。要求能转换、编译没有警告，输出与 CPython 一致。`tests/cases/` 每个用例针对一个特性，语料用来发现组合起来才出现的问题
- **示例应用**：`tests/test_examples.py` 检查 `examples/*/generated` 与当前转换结果完全相同，并且 `generated` 和 `rewritten` 两个版本在几组命令行参数下的输出、退出码都与 CPython 一致，改写出来的原生代码的单元测试通过
- **生成代码检查器**（§2）在每次转换时运行，所以 golden、语料和 e2e 都经过它，`tests/test_verify.py` 另有针对每条规则的单元测试
- **`PYO3_PYTHON` 必须显式设置**。`.cargo/config.toml` 只对仓库内的构建生效，生成到仓库外的 crate 会悄悄用系统 Python 构建
- 生成的 `Cargo.toml` 依赖 `steelix-runtime` 的方式：测试里用 path 依赖，发布后用固定版本
- 源码映射：函数前的 `// file.py:LINE` 注释，语句前的 `frame.line(N);`，异常的 traceback 里有原生函数的帧和 Python 行号（第 9 节）
- CI：GitHub Actions 跑 ruff、pytest（含 e2e），后续补上

## 15. 把函数改写成原生 Rust（M3b）

迁移的第 3 步是人逐个把生成的函数改写成地道的 Rust，程序的其余部分照常工作。还保留为 Python 的函数也可以手工改成原生代码，步骤见第 13 节。`examples/logsum` 是完整的例子：`generated/` 是转换的原样输出，`rewritten/` 把其中一个函数改写到第一步、两个函数改写成了原生 Rust，`tests/test_examples.py` 验证两者的输出都与 CPython 一致（除了下面说的一处有意的差异）。

### 两步走

生成的代码里，Python 的 list、dict 仍然是 libpython 的容器（`Bound<PyList>`、`Bound<PyDict>`），这是预期的起点。改写分两步：

1. **先改逻辑，数据留在 Python 容器里。** 函数体改成用 PyO3 的具体类型 API 直接操作原来的对象：`cast::<PyDict>()`、`cast::<PyList>()`、遍历、`get_item`、`set_item`，没有拷贝。参数和返回值仍是原来的 Python 对象，调用方和读取这些对象的其他函数都不受影响。例子里的 `tally` 就停在这一步：它返回的 dict 还被两个没改写的函数读取
2. **生产方和所有读取方都改写后，再把容器换成 Rust 类型。** 一个 dict 记录变成 `struct`、一个计数 dict 变成 `HashMap` 或 `Vec`，要等创建它的函数和所有读取它的函数一起改写。只改其中一个，就要在边界上来回转换，既慢又容易在转换处丢掉语义（顺序、键的类型）。例子里的 `parse_line` 和 `percentile` 做完了这一步，它们的数据在适配层里转换一次

类的方法要以整个类为单位改写（M5 之后）：实例的属性是方法之间共享的数据，只改写一个方法时它仍然要按 Python 对象读写这些属性，等同于第一步；把实例换成 Rust 结构体（`#[pyclass]`，§10）要求所有方法一起改写。

### 转换放在哪里

生成的函数本身就是面向 Python 的适配层。它的签名 `(m: &rt::Module<'py>, x: &Bound<'py, PyAny>, ..) -> PyResult<Bound<'py, PyAny>>` 是一个契约，有两类调用方依赖它：`rt::def!` 注册的函数对象（Python 代码、key 函数、装饰器都经过它调用），以及生成代码里的直接调用。改写时不改这个签名，而是

1. 把逻辑写成一个新的普通 Rust 函数，参数和返回值用原生类型，不出现 `Bound`、`Python`、`PyResult`
2. 生成的同名函数只留下类型转换：用 `extract` 把参数转成原生类型，调用新函数，再用 `into_bound_py_any`、`PyDict::set_item` 等把结果转回 Python 对象

所以 `extract`/`into_pyobject` 只出现在适配层里，每个函数一个地方。

```rust
// logsum.py:66
/// Nearest-rank percentile of an ascending list of numbers.
pub fn percentile<'py>(
    m: &rt::Module<'py>,
    sorted_values: &Bound<'py, PyAny>,
    pct: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let sorted_values: Vec<u64> = sorted_values.extract()?;
    native::percentile(&sorted_values, pct.extract()?).into_bound_py_any(m.py())
}
```

原生函数放进一个不依赖 PyO3 的模块，沿用 Python 的函数名。例子里是 `src/logsum/native.rs`，由 `src/logsum.rs` 开头的 `mod native;` 引入，不用改 `main.rs`。这样原生代码可以用 `cargo test` 单独测试，以后整个程序都变成原生代码时也不用再挪。

没有为每个函数另外生成一个转发用的适配函数（比如注册 `count_words_py`，由它调用 `count_words`）。改写之前两者完全一样，每个函数多出一个空转发，生成代码变长；改写函数体的签名之后，生成代码里的直接调用全部编译失败，要逐个在调用处加转换。现在的约定下调用方不用动。也没有让 `rt::def!` 按 `FromPyObject`/`IntoPyObject` 自动转换参数和返回值（类似 `#[pyfunction]`），那样可以直接改函数签名，但转换藏在宏展开里，出错时报在宏里，直接调用同样会全部失败。

### 改写步骤

下面是第二步（换成原生类型）的步骤，第一步只需要第 1、4 步，改的是函数体本身。

1. 选一个函数，读它的 Python 源码和调用方，确定参数和返回值的原生类型。可能是 `None` 的值用 `Option`，固定键的 dict 可以变成 struct。按调用图从叶子往上改写最省事
2. 在原生模块里写新函数，顺手写单元测试
3. 把生成的函数体换成转换加调用。原来返回 dict 或 tuple 的，在适配层里按原来的键和顺序构造，Python 一侧看到的结构不变
4. 跑端到端测试，确认输出仍与 CPython 一致

转换要与原来的 Python 语义对齐，对不齐的地方要有意识地决定。例如 `extract::<u64>()` 对负数抛 `OverflowError`、对 float 抛 `TypeError`，原来的 Python 代码也许接受它们；Rust 的 `split_whitespace`、`parse` 和 Python 的 `str.split`、`int()` 在非 ASCII 空白、负号、带下划线的数字、非 ASCII 数字等边界上不同（例子里的 `parse_line` 有意把这些行当成格式错误，README 和测试里写明了）；`HashMap` 没有插入顺序，转回 Python 的 dict 顺序会变，如果下游依赖 dict 的顺序（直接遍历输出，或者排序时只比较部分键），就要用 `Vec` 或保持顺序的 map。

### 调用方

- 生成代码里的直接调用不用改，它们仍然调用适配层，每次调用付出一次转换
- 两个函数都改写成原生代码后，原生函数之间直接调用，不经过适配层，也不需要 `m`
- 原生函数要调用还没改写的函数时，它就还不是纯 Rust。要么先改写被调用的函数，要么让它接收 `m: &rt::Module<'py>`，在调用处转换参数后调用对方的适配层
- 改变 Python 可见的签名（参数、默认值）时，要同时改 `rt::def!` 那一行和所有直接调用，和在 Python 里改函数签名是一样的工作

### 经过 Python 调用的函数

作为 key 函数传给 `sorted`、注册成回调、赋给别的变量或者被装饰（`@functools.lru_cache`）的函数，都经过 `rt::def!` 创建的函数对象调用。函数对象按 Python 的规则绑定参数，再调用生成的函数，也就是适配层。所以这些函数的改写方式完全一样，不需要额外处理。装饰器包装的仍然是函数对象，`functools.wraps` 看到的名字和 docstring 来自 `rt::def!` 那一行，不受改写影响。

嵌套函数和 lambda 也一样：生成的 fn item 就是适配层，捕获的变量（`&Bound` 或 `&rt::Cell`）在适配层里取出、转换后传给原生函数。

### 改写之后不再成立的东西

- monkeypatch：原生函数内部不再经过 Python，替换它读的全局变量（`mock.patch("app.LIMIT")`）或它调用的函数都对它不起作用。替换函数本身的效果不变，只影响经过 Python 的调用（第 12 节）
- 异常：原生函数里没有 Python 异常，失败用 `Option`/`Result` 表示，由适配层转回去。约定是适配层抛出和原来的 Python 代码相同类型的异常，调用方（生成的代码或者保留的 Python 代码）的 `except` 才能照常接住；确实要改变行为时要有意识地决定并写明。例子里的 `parse_line` 遇到不是数字的大小或时间时返回 `Err(native::NotANumber(text))`，适配层把它转成和 `int()` 一样的 `ValueError`（`invalid literal for int() with base 10: '12x'`），生成的 `load` 照常用 `except ValueError` 把这一行当成格式错误。panic 经过 PyO3 会变成 `PanicException`，不要用它表达预期中的错误
- 数值语义：Python 的 int 没有上限，换成 `u64`/`usize` 后溢出在 debug 构建里 panic，在 release 构建里回绕。浮点运算的顺序变了，结果可能在最后一位上不同
- 重新生成：改写过的模块文件从此由人维护，不能再用 steelix 覆盖。Python 源码有变化时，重新生成到别的目录，再把没有改写过的函数手工合并过来

## 16. 待定问题

1. 参数传递用 `&Bound` 还是 `Bound`。初版用 `&Bound`，如果生成的 clone 太多再调整
2. ABI 策略：按版本编译还是 abi3。runtime 里的 ffi 调用尽量限制在 limited API 内，保留 abi3 的可能
3. free-threaded（3.14t）是否支持，暂不考虑
