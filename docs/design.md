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
- 模块级变量的读写走 globals 字典：`rt::global(globals, "LIMIT")?` 先查 globals 再查 builtins，找不到抛 `NameError`；写入用 `globals.set_item("LIMIT", 20)?`。模块里没有绑定的名字视为 builtin，`print` 用 `rt::builtins::print`，其余用 `rt::builtin(py, "len")?`
- 比较在条件里用 `lt` 等返回 bool 的方法，需要值时用 `rich_compare(.., CompareOp::Lt)`
- 函数末尾的 `return` 写成尾表达式，返回值本身是 `PyResult` 时不再包一层 `Ok(..?)`
- 对 Python 可见的函数对象由 runtime 的 `Function` 包装（见第 6 节），注册只占一行：`rt::def!(globals, fib(n))?`。需要 globals 的写成 `rt::def!(globals, scale(globals, x))`，Rust 名字与 Python 名字不同时加 `name = "..."`，有 docstring 时加 `doc = "..."`
- 模块级函数若只绑定一次，调用时直接调用 Rust 函数（见第 7 节）
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
- 不可达的语句（`return` 之后等）直接丢弃，函数末尾可达时补 `Ok(rt::none(py))`
- `del x` 让变量变成未赋值状态，所以该变量必须是 `Option`
- 被内层函数捕获的变量（cell）单独处理，见第 6 节

## 6. 函数对象、闭包与默认参数

`wrap_pyfunction!` 得到的是 `builtin_function_or_method`，已验证它有以下问题：`functools.wraps` 无法写 `__name__`，不能设置属性，放进类里不会绑定 `self`。`PyCFunction::new_closure` 也有同样的问题，并且要求 `Send + Sync + 'static`，无法捕获 `Bound`。

因此 runtime 提供 `#[pyclass(dict)] Function`

- 实现 `__call__`（按存储的签名描述绑定参数）和 `__get__`（像 Python 函数一样绑定方法，返回 `types.MethodType`）
- `__name__`、`__qualname__`、`__doc__`、`__module__`、`__defaults__`、`__kwdefaults__`、`__dict__` 可写
  - M1 的实现：`#[pyclass(frozen, dict)]`，元信息直接放在实例 `__dict__` 里，天然可写，`__call__` 也不需要 borrow，函数体里修改自身属性不会冲突
  - `__module__` 取自 globals 的 `__name__`，与 CPython 一致；提供只读的 `__globals__`
- 持有指向生成的 Rust fn 的指针（统一签名的适配闭包，由 `rt::def!` 生成）、所属模块的 globals、捕获的 cell。通过 `__traverse__` 让 GC 看到 globals，模块 dict 和函数之间的循环引用可以回收
- 参数个数不对时抛和 CPython 文字一致的 `TypeError`。M1 只支持位置参数，收到关键字参数时抛 `TypeError` 说明暂不支持
- **默认参数在 `def` 执行时求值一次**，存在 `__defaults__` 里。不用 `#[pyo3(signature)]`，因为它每次调用都会重新求值，也不能引用模块全局变量
- 嵌套函数和 lambda 提升为具名 Rust fn，捕获的变量以 cell（`PyCell_New` 创建的 Python cell 对象）作为参数传入
- 装饰器按 Python 语义执行：先创建 `Function`，再调用装饰器并重新绑定名字

## 7. 模块、全局变量与 import

- 一个 Python 项目生成一个 crate，每个 Python 模块对应一个 Rust `mod`，模块间可以循环引用
- PyO3 0.30 使用多阶段初始化，执行顶层代码时模块已在 `sys.modules` 里，循环 import 能正常工作
- 包只导出一个 `PyInit_*`。包初始化时安装一个 meta path finder，子模块在被 import 时才执行，保持 Python 的惰性 import 顺序和副作用
- **全局变量不用进程级 static 保存模块对象**，因为 `importlib.reload` 或删除 `sys.modules` 后重新 import 会产生新的模块对象。globals 由 `Function` 持有并显式传给需要它的函数体
- 可以缓存 import 得到的标准库模块对象（`PyOnceLock`），但不能缓存它们的属性，否则 monkeypatch 失效
- **直接调用的条件**：名字在模块内只绑定一次（`def` 算一次），且模块里没有 `from x import *`、`globals()`、对自身模块的 `setattr`。否则走 globals 字典动态查找
- 直接调用无法感知其他模块的 `mock.patch("mod.fn")`，这是有意保留的语义差异（第 12 节）。应用场景里别的模块很少 patch 应用自身的函数，所以影响不大。以后如有需要再提供 `--dynamic-globals` 选项让所有全局调用都走动态查找
- 应用场景下 `importlib.reload` 也很少见，不用 static 保存模块对象主要是为了让同一份生成代码既能作为 `__main__` 运行，也能被别的模块按真实名字 import

## 8. 表达式与求值顺序

Rust 方法调用的形状会改变求值顺序，例如 `x in y` 写成 `y.contains(x)`、`o.m(f())` 写成 `o.call_method1("m", (f(),))`、`a[i] = f()` 写成 `a.set_item(i, f())`。规则如下

- 多个子表达式中有可能产生副作用的，按 Python 顺序先求值到临时变量。纯名字和字面量不需要临时变量，以保持可读
- 赋值语句先求值右边，再求值目标（`a[i] = f()` 是先 `f()`，再 `a`、`i`）
- 方法调用先 `getattr` 再求值参数（`o.missing(f())` 不应调用 `f`）
- 比较：在 `if`/`while` 条件里用 `lt` 等返回 bool 的方法；需要值时（`m = a < b`，numpy 之类会返回非 bool）用 `rich_compare`。链式比较中间操作数只求值一次
- 增量赋值：PyO3 没有 in-place 运算方法，用 runtime 的 `rt::iadd` 等（基于 `PyNumber_InPlaceAdd`），`a[i] += v`、`o.x += v` 中的 `a`、`i`、`o` 只求值一次
- `and`/`or`：值语义的 block 表达式，在条件里优化成 `&&`/`||`

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
  - 已知差异：`sys.argv[0]` 是二进制路径，`sys.path[0]` 不是脚本所在目录，`__file__` 未设置
- **ext 模式**（`--mode ext`，次要场景）：cdylib 扩展模块，替换原 Python 模块

### 分发（待设计）

可执行程序在别的机器上需要找到 libpython、标准库和第三方依赖。可能的方案有：随程序附带 python-build-standalone 加上 venv 的 site-packages，运行时按相对于二进制的路径定位；或者静态链接 libpython 并把纯 Python 依赖打包成 zip 放进 `sys.path`。在 M7 做 spike 再定。

## 12. 名字卫生与已知语义差异

名字卫生，由一个确定性的重命名表统一处理

- Rust 关键字用 `r#`（`type`、`match`、`loop`、`ref`、`move`、edition 2024 保留的 `gen` 等），PyO3 会去掉参数名的 `r#`
- `self`、`super`、`crate` 不能做 raw identifier，`self` 改名为 `slf`
- 生成代码自己使用的名字和 Python 名字冲突时，改名 Python 名字，加 `_` 后缀直到不冲突。函数体内保留 `py`、`globals`、`_guard`、`Ok`、`Err`、`Some`、`None`；模块级函数名保留 `exec_module` 等。不需要改名的名字优先保留原拼写，改名不会抢占它们
- Python 可见的函数名由 `rt::def!` 去掉 `r#` 得到，改过名的用 `name = "..."` 显式给出。参数名只用于报错信息，改过名的参数（如 `globals` → `globals_`）报错时显示 Rust 名字，M3 做完整签名时再处理
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

## 13. 不支持特性的处理

- 默认遇到不支持的语法就报错，诊断信息带 `文件:行:列` 和特性名，便于统计覆盖率
- **混合模式**：不支持的函数或类保留原 Python 源码嵌入 crate，在模块执行时编译并绑定到 globals。整个项目可以先跑起来，再逐步扩大可转换的范围。generator、async 适合用这种方式兜底。这是转换真实项目的前提，所以在 M5 前后实现

## 14. 测试与构建

- **单元测试**：rustast 打印、作用域与绑定分析等纯 Python 逻辑
- **Golden 快照**：`tests/cases/*.py` 转换结果与 `tests/golden/*.rs` 对比，`--update-golden` 更新。rustfmt 是必须的，固定 style edition，避免快照抖动
- **`cargo check` 层**：只检查 golden 能编译，比端到端快。同时要求生成代码没有编译警告（crate 根上只放 `non_snake_case`、`unused_variables` 两个 allow），把可读性问题挡在快照阶段
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
