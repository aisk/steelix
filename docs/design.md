# Steelix 设计文档

## 1. 目标

把 Python 源码转换成**可读的** Rust 代码。生成的代码通过 PyO3 继续调用 libpython，所以标准库和第三方 Python 库照常可用。典型迁移路径如下。

1. 用 steelix 把一个 Python 模块（或包）整体转成 Rust crate，行为与原 Python 一致。
2. 生成的 crate 作为 PyO3 扩展模块替换原模块，或者作为嵌入 libpython 的可执行程序运行。
3. 开发者逐个函数把 `Bound<PyAny>` 改写成原生 Rust 类型和逻辑，最终摆脱对 Python 的依赖。

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

期望输出（示意，`rt::def!` 等 runtime 接口在 M1 实现时确定）

```rust
use pyo3::prelude::*;
use steelix_runtime as rt;

// fib.py:1
pub fn fib<'py>(py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let _depth = rt::RecursionGuard::enter(py)?;
    if n.lt(2)? {
        return Ok(n.clone());
    }
    fib(py, &n.sub(1)?)?.add(fib(py, &n.sub(2)?)?)
}

pub fn exec_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    rt::def!(m, fib(n))?;
    rt::builtins::print(py, (fib(py, &rt::int(py, 10))?,))?;
    Ok(())
}
```

要点

- 所有 Python 值默认都是 `Bound<'py, PyAny>`，运算直接用 PyO3 的 `add`、`lt`、`getattr`、`call` 等方法
- 函数体是普通的 Rust fn，显式接收 `py: Python<'py>`，参数用 `&Bound`
- 对 Python 可见的函数对象由 runtime 的 `Function` 包装（见第 6 节），注册只占一行
- 模块级函数若只绑定一次，调用时直接调用 Rust 函数（见第 7 节）
- 会递归的函数（调用图里有环）开头加 `RecursionGuard`，否则栈溢出会直接 SIGSEGV 而不是抛 `RecursionError`
- runtime 一律通过 `rt::` 路径引用，不用 glob import，避免被 Python 里叫 `int`、`print` 的局部变量遮蔽
- 字面量规则：作为 `IntoPyObject` 参数时直接写字面量（`n.lt(2)`），超出 i32 范围的加 `i64` 后缀或走 `rt::int`，需要独立的 Python 对象时用 `rt::int(py, 10)`
- 尽量保留 Python 的名字、注释、docstring（转成 `///`），并在函数前标注源位置注释

## 5. 变量模型

Python 局部变量的作用域是整个函数，Rust 的 `let` 是块作用域，不能逐句对应。

- 每个局部变量在函数开头（或支配所有使用点的位置）声明一次，循环变量、`if`/`try` 内的赋值都是对它的赋值，不再重新 `let`
- 自行做定值分析，规则与 rustc 一致或更保守。能确定已赋值的变量用 `Bound`，不能确定的用 `Option<Bound>`，读取时经过 runtime 辅助函数，未赋值时抛 `UnboundLocalError`
- `del x` 让变量变成未赋值状态，所以该变量必须是 `Option`
- 被内层函数捕获的变量（cell）单独处理，见第 6 节

## 6. 函数对象、闭包与默认参数

`wrap_pyfunction!` 得到的是 `builtin_function_or_method`，已验证它有以下问题：`functools.wraps` 无法写 `__name__`，不能设置属性，放进类里不会绑定 `self`。`PyCFunction::new_closure` 也有同样的问题，并且要求 `Send + Sync + 'static`，无法捕获 `Bound`。

因此 runtime 提供 `#[pyclass(dict)] Function`

- 实现 `__call__`（按存储的签名描述绑定参数）和 `__get__`（像 Python 函数一样绑定方法）
- `__name__`、`__qualname__`、`__doc__`、`__module__`、`__defaults__`、`__kwdefaults__`、`__dict__` 可写
- 持有指向生成的 Rust fn 的指针、所属模块的 globals、捕获的 cell
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
- 直接调用无法感知其他模块的 `mock.patch("mod.fn")`，这是有意保留的语义差异（第 12 节）。提供 `--dynamic-globals` 选项让所有全局调用都走动态查找，用于跑依赖 mock 的测试

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

- **ext 模式**（默认，M1 只做这个）：cdylib 扩展模块，替换原 Python 模块
- **bin 模式**：嵌入 libpython 的可执行程序，已验证的问题较多，单独一个里程碑
  - 不用 `auto-initialize`，用 `PyConfig` 初始化，设置 `executable`/`home`，否则 `sys.path` 里没有 `.venv` 的 site-packages。同时传入真实的 `argv`
  - 以 `__main__` 身份执行模块。结束时调用 `Py_FinalizeEx`，这样会 flush stdout 并执行 atexit
  - `SystemExit` 和未捕获异常映射为退出码，并 `PyErr_Print`
  - 运行时动态链接器会优先找到系统的 libpython，需要设置 rpath

## 12. 名字卫生与已知语义差异

名字卫生，由一个确定性的重命名表统一处理

- Rust 关键字用 `r#`（`type`、`match`、`loop`、`ref`、`move`、edition 2024 保留的 `gen` 等），PyO3 会去掉参数名的 `r#`
- `self`、`super`、`crate` 不能做 raw identifier，`self` 改名为 `slf`
- 生成代码自己使用的名字（`py`、`m`、`Ok`、`Err`、`Some`）和 Python 名字冲突时，改名 Python 名字
- 含孤立代理（lone surrogate）的字符串不能写成 Rust `&str` 字面量，需要走转义构造

已知且有意保留的语义差异

- 直接调用的模块函数不受其他模块 monkeypatch 影响（可用 `--dynamic-globals` 关闭）
- traceback 里没有 Python 行号（缓解措施见第 14 节）
- 只有 C API 调用的热循环不经过解释器的检查点，Ctrl-C 需要 `PyErr_CheckSignals` 才能打断，PoC 阶段在循环里插入检查，或者先作为已知问题记录
- 同理，这类循环里 GIL 不会自动切换

## 13. 不支持特性的处理

- 默认遇到不支持的语法就报错，诊断信息带 `文件:行:列` 和特性名，便于统计覆盖率
- **混合模式**：不支持的函数或类保留原 Python 源码嵌入 crate，在模块执行时编译并绑定到 globals。整个项目可以先跑起来，再逐步扩大可转换的范围。generator、async 适合用这种方式兜底。这是转换真实项目的前提，所以在 M5 前后实现

## 14. 测试与构建

- **单元测试**：rustast 打印、作用域与绑定分析等纯 Python 逻辑
- **Golden 快照**：`tests/cases/*.py` 转换结果与 `tests/golden/*.rs` 对比，`--update-golden` 更新。rustfmt 是必须的，固定 style edition，避免快照抖动
- **`cargo check` 层**：只检查 golden 能编译，比端到端快
- **端到端差分测试**：ext 模式下比较 `python case.py` 与 `python -c "import case"`（以及 bin 模式就绪后的可执行程序）的 stdout、退出码，以及预期异常的类型和消息（不比较 traceback）。多个用例打包成同一个测试 crate 的多个模块，共用 `CARGO_TARGET_DIR`，减少编译和链接次数。用 pytest marker `e2e` 区分
- **`PYO3_PYTHON` 必须显式设置**。`.cargo/config.toml` 只对仓库内的构建生效，生成到仓库外的 crate 会悄悄用系统 Python 构建
- 生成的 `Cargo.toml` 依赖 `steelix-runtime` 的方式：测试里用 path 依赖，发布后用固定版本
- 源码映射：函数和语句前的 `// file.py:LINE` 注释，异常经过时用 `add_note` 附加 Python 位置。对迁移的人来说行号很重要，优先级不低
- CI：GitHub Actions 跑 ruff、pytest（含 e2e），后续补上

## 15. 待定问题

1. 参数传递用 `&Bound` 还是 `Bound`。初版用 `&Bound`，如果生成的 clone 太多再调整
2. ABI 策略：按版本编译还是 abi3。runtime 里的 ffi 调用尽量限制在 limited API 内，保留 abi3 的可能
3. free-threaded（3.14t）是否支持，暂不考虑
