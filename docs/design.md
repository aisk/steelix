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

- 性能优化，类型推导出原生类型。按类型注解生成原生类型（`int` 变成 `i64` 等）也不做：注解在运行时不强制，`def f(x: float)` 用 `3` 调用时结果必须是 int，要保持语义就得同时生成两套代码路径。唯一的例外是 Python 语义保证只会是 `bool` 的局部变量（§5），它不依赖注解
- 100% 语义兼容。允许有文档化的差异（见第 12 节）
- metaclass 自定义、`exec`/`eval` 等复杂特性的原生转换。用到它们的代码通过混合模式保留为 Python 源码照常运行（第 13 节）。generator 和 async/await 原来也在这里，M13 找到了可读的做法并原生转换（§8 "协程和生成器"），只剩异步生成器、异步推导式等少数写法保留为 Python。match 也一样，M14 用 let chains 原生转换（§8 "match 语句"），只有带捕获的 OR 模式保留为 Python

## 2. 总体流水线

```
.py 源码
  │  project.py              找出项目里的模块（入口脚本或包，§7）
  │  frontend/parse.py       ast.parse + tokenize 收集注释
  ▼
Python AST + 注释
  │  frontend/desugar.py     先改写成更简单的等价写法（async 节点、局部变量的注解等）
  │  frontend/subset.py      找出还没有 lowering 的构造
  │  frontend/fallback.py    混合模式：决定哪些语句保留为 Python（§13）
  │  frontend/scope.py       symtable 分析作用域（local/global/nonlocal/cell/free），函数的种类
  │  frontend/module_bindings.py  自行遍历 AST，统计模块级名字的绑定次数（§7）
  │  frontend/locals.py      局部变量的定值分析（definite assignment，§5），模块和类体的可达性
  │  frontend/regions.py     lowering 放进 Rust 闭包的代码（§9）
  │  frontend/booleans.py    只会是 bool 的局部变量（§5）
  │  frontend/resolve.py     全局名字的引用怎样编译：直接调用、globals 查找、特化的 builtin（§7）
  │  frontend/callgraph.py   直接调用的调用图（递归、Box::pin）
  │  frontend/patterns.py    case 模式的分析（§8）
  │  frontend/classinfo.py   编译器算出的类的 __static_attributes__（§10）
  ▼
带作用域信息的 AST
  │  lower/module.py         整个模块（exec_module、保留的语句生成 rt::exec_python!）
  │  lower/function.py       def 和 lambda，lower/classes.py 类，lower/stmt.py 语句，
  │  lower/expr.py ...       lower/expr.py 表达式（按 Python 求值顺序生成临时变量），calls.py 调用，
  │                          defs.py 函数对象（rt::function!、装饰器），values.py 表达式的形态（Value），
  │                          handling.py（try/raise/with）、comprehension.py、patterns.py（match）、
  │                          temps.py、rustflow.py、names.py（名字卫生，§12）、docstrings.py
  ▼
Rust AST（rustast/nodes.py）
  │  rustast/cleanup.py      返回的局部变量 move 而不是 clone，声明移到第一次使用之前（§8）
  │  rustast/rewrite.py      函数体的通用改写
  │  rustast/verify.py       检查作用域、跳转标签、带类型的接收者（rustast/pyo3.py）
  │  rustast/printer.py      渲染成源码，再强制过一遍 rustfmt（rustast/fmt.py）
  ▼
.rs 源码
  │  emit/crate.py           生成 Cargo.toml、main.rs/lib.rs、每个 Python 模块一个 Rust mod
  ▼
Cargo crate
  │  dist.py                 `steelix build`：cargo build --release，组装可以搬走的 dist/app/（§11）
  ▼
可执行程序
```

命令行（`cli.py`）的子命令：`transpile`（生成 crate）、`build`（生成、构建并组装分发，§11）、`test`（用项目自己的测试检查转换后的程序，pytest 插件在 `pytest_native.py`，§15）、`coverage`（只算覆盖率，不写 crate，§13）、`attrs`（属性读写索引和 `#[pyclass]` 草稿，`attrs.py`、`pyclass.py`，§15）。

选择先构建一个小型 Rust AST 再打印，而不是直接拼字符串。这样 lowering 和格式化解耦，也方便做局部的可读性改写（`rustast/cleanup.py` 把返回的局部变量 move 出去而不是 clone，§8）。Rust AST 只覆盖生成代码用得到的部分，不追求完整。

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
├── docs/
│   ├── design.md              本文档
│   ├── roadmap.md             里程碑与任务拆解
│   ├── coverage.md            真实应用的转换覆盖率与优先级建议
│   └── m10-skime.md           真实项目 skime 的测量（M10、M12）
├── src/steelix/
│   ├── cli.py                 命令行入口：transpile、build、test、coverage、attrs（§2）
│   ├── coverage.py            转换覆盖率报告
│   ├── project.py             找出项目里的模块（§7）
│   ├── dist.py                `steelix build` 的分发目录（§11）
│   ├── pytest_native.py       `steelix test` 的 pytest 插件（§15）
│   ├── attrs.py               属性读写索引 `steelix attrs`（§15）
│   ├── pyclass.py             `#[pyclass]` 草稿 `steelix attrs --pyclass`（§15）
│   ├── diagnostics.py         诊断信息、源码位置、UnsupportedError
│   ├── frontend/              解析、改写、作用域和绑定分析、混合模式的判定（§2）
│   ├── lower/                 Python AST → Rust AST（§2）
│   ├── rustast/               Rust AST 定义、清理、检查、打印、rustfmt
│   └── emit/                  生成完整 crate
├── crates/
│   └── steelix-runtime/       生成代码依赖的运行时（`rt::`）：function.rs（Function）、
│                              macros.rs（rt::function!、rt::def!）、values.rs（字面量、slice、
│                              f-string）、names.rs（驻留的名字）、frame.rs（帧和异常）、class.rs、
│                              coroutine/（协程和生成器：state、channel、awaits、objects、finalize）、
│                              generator.rs（生成器表达式）、matching.rs（match）、
│                              python.rs（混合模式）、importer.rs、embed.rs（bin 模式）、
│                              multiprocessing.rs 等
├── examples/                  示例应用：转换的原样输出和手工改写的版本（第 15 节）
└── tests/
    ├── cases/                 端到端用例（*.py），同时作为 golden 输入
    ├── apps/                  从标准库复制的真实命令行工具（混合模式）
    ├── projects/              多模块的程序（M6）：测试应用、命名空间包、标准库 json 包和 skime 的副本
    ├── attrs/                 属性读写索引的测试项目
    ├── corpus/                模仿真实小程序的脚本，和 CPython 比较输出（§14）
    ├── dist_apps/             分发（`steelix build`）的测试程序
    ├── golden/                期望生成的 .rs（快照测试），projects/ 下是项目的整个 src/，attrs/ 下是索引和草稿
    └── test_*.py
```

`CLAUDE.md`（给后续会话的约定与命令速查）是本地文件，被全局的 gitignore 忽略，不在仓库里。

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
    rt::frame!(m, "fib", 1, |_| {
        if n.lt(2)? {
            return Ok(n.clone());
        }
        fib(m, &n.sub(1)?)?.add(fib(m, &n.sub(2)?)?)
    })
}

/// Executes the body of `fib.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
        rt::def!(m, fib(n))?;
        rt::builtins::print(py, (fib(m, &rt::int(py, 10))?,))?;
        Ok(())
    })
}
```

要点

- 所有 Python 值默认都是 `Bound<'py, PyAny>`，运算直接用 PyO3 的 `add`、`lt`、`get_item`、`call` 等方法
- 名字写在源码里的属性和方法（`o.x`、`o.x = v`、`o.m(a)`）用 runtime 的扩展 trait `rt::Attributes`（M11）：`o.attr("x")?`、`o.set_attr("x", v)?`、`o.del_attr("x")?`、`o.method0("m")?`、`o.method1("m", (a,))?`。它们就是 PyO3 的 `getattr`、`setattr`、`delattr`、`call_method0`、`call_method1`，区别只在名字必须是 `'static` 的字面量，runtime 为每个名字只创建一次驻留（interned）的 Python 字符串（第 6 节"调用的开销"）。用到它们的模块多一行 `use steelix_runtime::Attributes;`。trait 对所有的 `Bound<'py, T>` 实现（M11a），手写代码里的 `Bound<PyDict>`、`Bound<PyTuple>`、`#[pyclass]` 的值不用先 `.as_any()`，`Borrowed` 通过 `Deref` 也能用。名字在运行时才知道的（手写的代码里）照常用 PyO3 的方法
- 函数体和模块体包在 `rt::frame!` 里，异常的 traceback 因此有原生函数的帧，函数名和 Python 一样，行号是 `def` 所在的行。闭包参数 `frame` 只在用到时出现（`raise`、`except` 等），否则写 `|_|`。`--traceback-lines` 在可能出错的语句前加 `frame.line(N);`，行号精确到语句（见第 9 节）
- 函数体是普通的 Rust fn，第一个参数是模块上下文 `m: &rt::Module<'py>`，Python 参数用 `&Bound`。`m.py()` 给出 `Python<'py>`，函数体里用到不止一次时开头写 `let py = m.py();`，只用一次时就地写 `m.py()`
- 模块体生成 `exec_module(m)`，只依赖传入的模块上下文，与输出模式无关
- 模块级变量的读写走 globals 字典：`m.global("LIMIT")?` 先查 globals 再查 builtins，找不到抛 `NameError`；写入用 `m.set_global("LIMIT", 20)?`。模块里没有绑定的名字视为 builtin，`print` 用 `rt::builtins::print`，`len(x)` 等少数几个直接写成 PyO3 方法（见第 8 节），其余用 `rt::builtin(py, "max")?`
- 比较在条件里用 `lt` 等返回 bool 的方法，需要值时用 `rich_compare(.., CompareOp::Lt)`
- 函数末尾的 `return` 写成尾表达式，返回值本身是 `PyResult` 时不再包一层 `Ok(..?)`
- 带 `else` 且有 `break` 的循环包在带标签的 block 里，`break` 写成 `break 'for_else`，正常结束时执行 `else`。`else` 里的 `break`/`continue` 属于外层循环，要跨出这个 block，rustc 不允许不带标签的跳转跨出带标签的 block（E0695），所以只在这时给外层循环加标签 `'outer: for ..`，写成 `continue 'outer`。其余循环都不带标签
- 对 Python 可见的函数对象由 runtime 的 `Function` 包装（见第 6 节），注册只占一行：`rt::def!(m, fib(n))?`，签名的写法和 Python 一样（`greet(name, greeting = "Hello", *, punct = "!")`）。Rust 名字与 Python 名字不同时写成 `helper_ as "helper"(x)`，有 docstring 时加 `doc = "..."`，装饰器写在名字前面（`rt::def!(m, @functools.cache fib(n))?`，第 6 节）。多行的 docstring（还有类和模块的 `__doc__`）在 Rust 字面量里也是一行对一行，行尾用 `\` 续行（M11a），续行的缩进和行首的空白不属于字符串，所以值的行首空格写成 `\x20`

  ```rust
  rt::def!(
      m,
      dumps(obj),
      doc = "Serialize ``obj`` to a JSON formatted ``str``.\n\
          \n\
          If ``skipkeys`` is true then ``dict`` keys that are not basic types\n\
          \x20   (indented in the docstring)\n"
  )?;
  ```
- 模块级函数若只绑定一次，并且能在转换时绑定参数，调用时直接调用 Rust 函数（见第 6、7 节）
- 嵌套函数和 lambda 是写在外层函数体里的 Rust fn，被捕获并会改变的变量是 `rt::Cell`（见第 6 节）
- 会递归的函数（直接调用构成的调用图里有环）开头加 `let _guard = rt::RecursionGuard::enter(py)?;`，否则栈溢出会直接 SIGSEGV 而不是抛 `RecursionError`。经过 Python 对象的调用由 `Function` 的入口检查（第 6 节），不需要 guard
  - 3.14 的 `Py_EnterRecursiveCall` 按 C 栈剩余空间判断，不看 `sys.getrecursionlimit()`，报错信息是 `Stack overflow (used N kB)`（见第 12 节）
  - 已验证的坑：栈快耗尽时 `PyErr::fetch` 第一次创建 PyO3 的 `PanicException` 类型会失败并在 PyO3 的懒初始化锁上死锁。`RecursionGuard::enter` 每次先确保这个类型已经创建
- runtime 一律通过 `rt::` 路径引用，不用 glob import，避免被 Python 里叫 `int`、`print` 的局部变量遮蔽
- 字面量规则：作为 `IntoPyObject` 参数时直接写字面量（`n.lt(2)`），超出 i32 范围的加 `i64` 后缀或走 `rt::int`，需要独立的 Python 对象时用 `rt::int(py, 10)`
- 尽量保留 Python 的名字、注释、docstring（转成 `///`），并在函数前标注源位置注释

## 5. 变量模型

Python 局部变量的作用域是整个函数，Rust 的 `let` 是块作用域，不能逐句对应。

- 每个局部变量只声明一次，循环变量、`if`/`try` 内的赋值都是对它的赋值，不再重新 `let`
  - 第一次出现是函数体顶层的赋值语句时，直接在那里写 `let x = ...;`（顶层语句按顺序执行，它支配之后所有使用点），这样直线代码最可读
  - 否则先声明 `let x: Bound<'py, PyAny>;`，由后面的分支赋值。lowering 把这些声明放在函数开头，清理时再移到第一次用到它的语句之前，那条语句在每条路径上都给它赋值时并成 `let x = if .. { a } else { b };`（§8）
- 局部变量按第一次出现的顺序声明（参数在前，`LocalsInfo.order`），清理时再移到第一次使用之前（§8 的 `declare_late`）。函数返回时 CPython 按 `co_varnames` 的逆序释放局部变量，Rust 按声明的逆序 drop，多数时候两边一致，`__del__` 和 weakref 回调的顺序相同（`tests/cases/drop_order.py`）。不一致的情况是声明移动时越过了别的局部变量的声明，比如和 `if` 链合并成 `let x = if ..` 的变量排到了链里才赋值的变量后面（§12）。M9 到 M14 曾经从编译器取 `co_varnames`，按它排序声明并限制移动，让释放顺序完全一致，代价是按 qualname 和首行从编译器的 code 对象里找每个函数、按它排序和限制移动的逻辑，以及二十多个不能合并的 `if` 链。程序很少依赖函数返回时局部变量之间的释放顺序，M14 之后的 review 去掉了这个限制
  - 可能重复赋值（另做一遍"可能已赋值"分析，循环取不动点）时才加 `mut`，与 rustc 的判断一致，不产生 `unused_mut`
  - 被重新赋值的参数在开头遮蔽成 `let mut n = n.clone();`
- 自行做定值分析，规则与 rustc 一致或更保守（条件一律视为不透明，`while True` 没有 `break` 时之后不可达）。能确定已赋值的变量用 `Bound`，不能确定的用 `Option<Bound>`：声明为 `let mut x: Option<Bound<'py, PyAny>> = None;`，赋值写 `x = Some(..)`，读取写 `rt::local(&x, "x")?`，未赋值时抛和 CPython 一样的 `UnboundLocalError`
- 不可达的语句（`return`、`break`、`continue` 之后，没有 `break` 的 `while True` 之后）直接丢弃，函数末尾可达时补 `Ok(rt::none(py))`。模块级代码也做同样的可达性分析，避免 rustc 报 unreachable 警告
- 定值分析覆盖 `for`/`while`（在循环头取不动点）、`break`/`continue`、循环 `else`、解包赋值、增量赋值（先读后写）、`import`（绑定局部变量）和 `del`
- `del x` 让变量变成未赋值状态，所以被 `del` 的局部变量（包括参数）一律是 `Option`。删除时能确定已赋值就写 `x = None;`，否则写 `rt::del_local(&mut x, "x")?;`，未赋值时抛 `UnboundLocalError`。被 `del` 的参数在开头遮蔽成 `let mut a: Option<..> = Some(a.clone());`
- `Option` 变量如果第一次出现是顶层赋值，也在那里声明：`let mut x = Some(..);`
- 循环变量：如果一个名字只被某个 `for` 的目标绑定（可以是嵌套元组），并且只在这个循环体内读取，就在每次迭代开头声明，不提前声明也不需要 `mut`

  ```rust
  for x in rt::iter(xs)? {
      let x = x?;
      ..
  }
  for item in rt::iter(pairs)? {
      let [key, value] = rt::unpack(&item?)?;
      ..
  }
  ```

  否则（循环后还要读，或者别处也赋值）照常赋值给事先声明的变量：`for item in rt::iter(xs)? { x = item?; .. }`
- 解包赋值的目标都是普通局部变量时，第一次出现写成 `let [a, b] = rt::unpack(&value)?;`，之后写成解构赋值 `[a, b] = rt::unpack(&value)?;`。目标里有属性、下标、全局变量或 `Option` 变量时，先解包到临时变量，再按顺序逐个赋值
- 函数内的带注解赋值 `x: int = v` 在分析之前改写成 `x = v`（`frontend/desugar.py`），因为局部变量的注解从不求值。没有值的 `x: int` 只让 `x` 成为局部变量，直接去掉
- 函数顶层第一次出现的 `import` 也直接 `let os = rt::import(m, "os")?;`
- `:=` 的目标第一次出现在顶层语句一定会求值的部分（表达式语句、赋值的右边、`if` 条件、`for` 的可迭代对象，不包括 `and`/`or` 的右侧、条件表达式的分支、链式比较后面的部分和推导式）时，在那里写 `let x = ..;`，不提前声明（否则 clippy 报 `needless_late_init`）
- body 不会失败的 `try`（只有 `pass`，或者一条语句把常量、已赋值的普通局部变量或它们组成的元组赋给普通局部变量、作为表达式求值或返回，如 `try: x = 1`）直接生成 body，handler 不可达，变量在 body 的赋值处声明。多条语句的 body 仍然走闭包
- A 形式的 `try`（§9）`x = expr`，没有 `else`/`finally`，每个 handler 都以给 `x` 赋值结束或者跳走（`raise`、`return`、`break`、`continue`），并且 handler 里别处不用 `x` 时，写成 `let x = match .. { Ok(value) => value, Err(..) => { ..; value } };`
- 被内层函数捕获并且之后会改变的变量（cell）用 `rt::Cell`，不参与上面的处理，见第 6 节
- 只会是 `bool` 的局部变量（M9，`frontend/booleans.py`）是 Rust `bool`：`let mut found = false;`，`found = true;`，`if !found { .. }`。条件是它每次被赋的值按 Python 的语义一定是 `bool`，与操作数的类型无关，并且 lowering 本来就把这个值算成 Rust `bool`
  - 符合的值只有 `True`/`False`、`not x`、单个的 `is`/`is not`/`in`/`not in` 比较（`in` 会把 `__contains__` 的结果转成 `bool`）、名字只可能是 builtin 时的 `isinstance(x, t)` 和 `bool(x)`、其它这样的局部变量，以及操作数都符合的 `and`/`or` 和条件表达式。`a < b`、`a == b` 不算，富比较可以返回任何对象（numpy、自定义的 `__eq__`），Python 的局部变量存的就是那个对象
  - 变量本身必须是普通的 Rust 局部变量：只由 `x = value` 赋值（没有 `for`、`with`、`except`、`import`、`del`、增量赋值、解包赋值和 `:=`），不会在可能未赋值时读取（那需要 `Option`，仍然用对象），没有被内层函数捕获，也不是单表达式 `try`（`try: x = expr`）的赋值目标（它的 `match` 得到的是对象）。函数里的 `locals()` 等本来就让整个函数保留为 Python
  - 赋值时右边按条件求值（`exprs.condition`），和 `if` 的条件写法一样。读取时是 `Kind.BOOL` 的值，和 `in`、`is` 的结果一样处理：条件里直接用，作为 `IntoPyObject` 参数（`print(found)`、`call1((found,))`）直接传 `found`，PyO3 转换成 `True`/`False`；需要 `Bound<PyAny>` 的地方（返回值、方法的接收者、`&Bound` 参数）写 `rt::bool(py, found)`。没有用 `PyBool::new(py, found)`，因为它返回 `Borrowed<PyBool>`，还要 `.to_owned().into_any()`；`found.into_pyobject(py)?` 也一样要擦除类型。全是 Rust `bool` 的列表、元组字面量直接写成 `[found, x.is_none()]`
  - 标准库 154 个模块里 7171 个转换的函数共有 12087 个局部变量，符合的有 128 个（99 个函数），`tests/apps` 的 6 个工具里有 4 个
- `try` 体、`finally` 之前的部分和 `with` 体在立即调用的闭包里执行（第 9 节），闭包不能初始化外层 fn 里声明了但还没赋值的变量，所以（`frontend/regions.py`、`frontend/locals.py`）
  - 只在一个闭包区域内出现的局部变量在闭包里声明，规则同上（区域顶层第一次赋值处 `let`，否则在闭包开头声明），每次执行闭包都从未赋值开始
  - 在闭包里赋值、但在外面也用到的变量，进入闭包时必须已经确定赋值，否则是 `Option`
  - 只有一个表达式的 `try`（`x = int(s)`）不需要闭包，`match` 之后才赋值，`x` 仍然是普通的 `Bound`
  - `except .. as e` 的名字只在 handler 里使用时是 handler 里的 `let`，`with .. as f` 的名字只在这些 `with` 里使用时是闭包参数

## 6. 函数对象、闭包与默认参数

`wrap_pyfunction!` 得到的是 `builtin_function_or_method`，已验证它有以下问题：`functools.wraps` 无法写 `__name__`，不能设置属性，放进类里不会绑定 `self`。`PyCFunction::new_closure` 也有同样的问题，并且要求 `Send + Sync + 'static`，无法捕获 `Bound`。

因此 runtime 提供 `#[pyclass(frozen, dict, weakref)] Function`

- 实现 `__call__`（按存储的签名描述绑定参数）和 `__get__`（像 Python 函数一样绑定方法，返回 `types.MethodType`），`repr` 是 `<function qualname at 0x..>`。M11 起还支持 vectorcall，并且和 Python 函数一样带 `Py_TPFLAGS_METHOD_DESCRIPTOR`（下面的"调用的开销"）
- 持有指向生成的 Rust fn 的指针（统一签名的小闭包，由 `rt::function!` 生成，把绑定好的参数逐个传给 Rust fn）、所属模块的 globals、闭包的 cell。通过 `__traverse__` 让 GC 看到它们，模块 dict 和函数之间的循环引用可以回收
- 元信息（`__name__`、`__qualname__`、`__doc__`、`__module__`、`__defaults__`、`__kwdefaults__`、`__annotations__`、`__annotate__`、`__type_params__`）放在 `Mutex` 里，都是可写的真实属性，赋值时做和 CPython 一样的类型检查（`__defaults__` 只接受 tuple 或 None 等）。M1 把它们放在实例 `__dict__` 里，但那样 `__defaults__` 无法做类型检查，调用时也要查 dict。`__dict__` 照常可写
  - 持有锁的时候不运行 Python 代码，也不释放 Python 对象：赋值时先在锁里换出旧值，释放锁之后再 drop 它（M11a，`Function::replace`），`__clear__` 也一样。旧值的 `__del__` 可能再读这个函数的元信息，`Mutex` 不可重入，在锁里 drop 会永远卡住；`__del__` 里释放 GIL 时，另一个线程读元信息也会拿着 GIL 等锁，同样死锁（`tests/cases/metadata_del.py`）。`str` 子类的 `__str__` 也是 Python 代码，`repr` 先取出 qualname 再转换。生成器表达式对象（§8）的名字同样处理
- 函数类型声明在 `steelix_runtime` 模块里（`#[pyclass(module = ..)]`，类型的 repr 是 `<class 'steelix_runtime.function'>`），但并没有可以 import 的 `steelix_runtime` 模块，按模块路径找不到这个类型。函数对象本身按 qualname pickle，不受影响
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

- `rt::function!` 返回函数对象，`rt::def!` 再把它绑定到函数名，模块代码里是 globals，类体里是类的命名空间（§10）。第一个参数是模块上下文（类体里是 `ns`），函数对象保存它的 globals，经过 Python 调用时再用它构造 `rt::Module` 传给 Rust fn。装饰器写在名字前面（下面的装饰器一节）
- 签名里 `/`、`*`、`*args`、`**kwargs` 与 Python 相同。默认值是 Rust 表达式，在宏展开处（也就是 `def` 执行时）按 Python 顺序求值一次。Rust 名字与 Python 名字不同的参数写成 `py_ as "py"`，报错信息和 `inspect.signature` 里显示 Python 名字
- 注解写成 `x: "int"`、`-> "str"`，是 Python 源码文本（见下面的注解一节）
- 方括号里是闭包捕获的变量，`&count` 是 cell，不带 `&` 的是值（见闭包一节）
- Rust 名字和 Python 名字不同的函数和参数一样写成 `helper_ as "helper"(x)`，`r#type` 这样的关键字不用写，宏去掉 `r#`
- 可选的 `qualname = ".."`、`doc = ".."`（还有和 `as` 一样的 `name = ".."`，生成的代码不用）。qualname 默认就是名字，类体里定义的函数是 `<类的 qualname>.<名字>`（§10），只在不同的时候写出来，比如嵌套函数是 `outer.<locals>.inner`，lambda 是 `<lambda>`
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

### 调用的开销（M11）

M10 在 skime 上测到 Python 调用原生函数比调用 Python 函数慢几倍（`ctx.top()` 206 ns，CPython 43 ns），原因是调用协议和属性名，与函数体无关。M11 只做了不改变语义、不影响生成代码可读性的几项

- **vectorcall（PEP 590）。** 以前只有 `tp_call`，CPython 每次调用都要先建参数 tuple（有关键字参数时还有 dict），PyO3 再解析一遍。现在 `Function` 有 vectorcall 入口，直接拿到参数数组和关键字名字的 tuple，`__call__` 仍然保留（`f.__call__(..)` 走它），两条路径共用同一个绑定函数，报错文字不变
- **不需要绑定的调用直接传参数。** 只有位置参数的函数用位置参数调用、没有关键字参数时，参数数组本身就是函数体要的参数列表，不复制。参数少于形参时，缺的那些从调用时的 `__defaults__` 末尾补上（和 CPython 一样，默认值不够时交给完整的绑定去报错），补齐的参数列表放在栈上（`SmallVec`，M14 之后，以前每次调用都分配一个 `Vec`；skime 的读取器快约 3%，Python 侧的 `p.pop()` 从 105 ns 到 85 ns）。其余情况（关键字参数、`*args`、keyword-only、`**kwargs`）走 `initialize_locals` 的完整算法
- **`Py_TPFLAGS_METHOD_DESCRIPTOR`。** 表示 `f.__get__(obj)(*args)` 等价于 `f(obj, *args)`，CPython 的 `LOAD_ATTR` 方法优化和 `PyObject_VectorcallMethod`（`method0`/`method1` 用的）因此不再为 `obj.m()` 创建 bound method。取属性 `obj.m` 本身照样经过 `__get__` 得到 bound method，内省看到的东西不变
- PyO3 的 `#[pyclass]` 不能声明这两项，所以在创建第一个函数时补到类型对象上：`tp_vectorcall_offset` 指向 `Function` 里存放入口指针的字段，`tp_flags` 加上两个标志，再 `PyType_Modified`。`Function` 不能被继承，所有实例布局相同
- vectorcall 的入口经过 PyO3 自己包装 fastcall 函数用的 trampoline（`pyo3::impl_`，随 PyO3 的版本固定）。它把错误和 panic 变成异常，并且告诉 PyO3 当前线程已经 attach。只用 `Python::assume_attached` 不够：在 Python 自己创建的线程里调用原生函数时 PyO3 不知道已经 attach，函数里释放的 `Py<T>`（例如处理完的异常）会推迟到以后才 decref，`__del__` 的时机就和 CPython 不同（`tests/cases/call_protocols.py` 最后一段检查这一点）
- CPython 在 `tp_call` 之前检查递归深度，vectorcall 之前不检查，所以入口自己调用 `Py_EnterRecursiveCall`，消息和以前一样
- **关键字参数直接 vectorcall（M14 之后）。** 没有 `**` 和 `*` 的 `rt::call!(f, x, sep = ", ")` 把位置参数和关键字参数的值放进一个数组，关键字的名字是每个调用点只建一次的 tuple（`static PyOnceLock`），直接 `PyObject_Vectorcall`，不再先建 dict 再由 CPython 拆开。`rt::call!(f, &p, idx = 1)` 从约 208 ns 到约 124 ns。有 `*`、`**` 的仍然建 tuple 和 dict，报错都由被调用的对象产生，两条路径一样（`call_protocols` 的最后一段）
- **驻留的名字。** `rt::Attributes`（第 4 节）、`m.global`、`rt::builtin` 和 `rt::call!` 的关键字参数都只接受 `'static` 的名字，按字符串字面量的地址和长度找到驻留的 Python 字符串。PyO3 从 `&str` 每次新建字符串，CPython 再算哈希，而且属性缓存只认驻留的名字，这一项在 skime 上就快了一倍。查找用一张 4096 项、按地址索引的表，读取不加锁（attach 的线程轮流运行，写入只在第一次），几个位置都被别的名字占了时退回一张加锁的 `HashMap`（M11a 之前是每个线程一张，每个线程都要为同一个名字泄漏一份）。和 `pyo3::intern!` 一样，这些字符串不释放
  - 不加锁的读取只在有 GIL 时正确：写入一个位置的几个字段之间不会有别的线程运行。free-threaded 的 Python（3.14t）没有这个保证，`Function` 的元信息、`Generator` 的状态和协程的 future（§8）也是按有 GIL 设计的，所以 runtime 在 free-threaded 的构建上直接编译失败（`build.rs` 用 `pyo3-build-config` 得到 `Py_GIL_DISABLED`，`lib.rs` 里 `compile_error!` 指向这一节），而不是生成运行时才出错的代码

在 skime 上（release，fib 22）从 2.93 s 降到 1.10 s，CPython 是 0.48 s。只换 runtime、生成代码不变时是 2.11 s。各项的数字见 `m10-skime.md` 第 7 节。

没有做的

- 参数不纯时的方法调用 `o.attr("m")?.call1((f()?,))?` 仍然先创建 bound method。要避免它得在 runtime 里重新实现 `_PyObject_GetMethod`（类型的 MRO、数据描述符、实例 dict、`__getattr__`），不算低成本
- 字符串常量 `rt::str(py, "..")` 仍然每次新建。缓存它们会让 `is` 的结果和以前不同（虽然更接近 CPython），而且不在热点上
- 循环里的 `py.check_signals()`、`rt::frame!` 和直接调用的 `RecursionGuard` 都只占几 ns，没有动（M14 之后 `for` 循环的检查挪进了 `rt::iter`，为了可读性）
- M14 之后的 review 又测过不值得的：字符串常量缓存、`__slots__` 的 inline cache（`roadmap.md` 的性能候选项）；去掉 vectorcall 入口的 `RecursionGuard` 最多快 6.5%，但 Python 侧的递归要靠它抛 `RecursionError`
- 全局变量的缓存（dict watcher），理由见第 7 节

### 嵌套函数与 lambda

嵌套函数和 lambda 提升为具名 Rust fn，作为 item 写在外层函数体里 Python 定义的位置，紧接着用 `rt::function!` 创建函数对象并绑定局部变量。读代码时函数体和 Python 里的位置一致。

- Rust 名字就是 Python 名字，与外层函数的局部变量、临时变量或模块函数冲突时加 `_` 后缀（fn item 的名字和局部变量在同一个命名空间，`let show = rt::function!(m, show_[&i]())?;`）。lambda 叫 `lambda`、`lambda2`
- 嵌套函数一律通过函数对象调用（`go.get()?.call1((..))`），不做直接调用。嵌套函数的名字常常在循环里重新绑定，直接调用的分析不值得
- 嵌套函数可以递归：函数名本身是外层函数的 cell，嵌套函数通过 cell 读到自己
- lambda 的函数体是一个尾表达式
- `if`、`elif`、`while`、`for` 头部表达式里的 lambda，其 fn item 放在整个语句之前，而不是第一个分支或循环体里，否则头部看不到它
- 嵌套函数的 docstring 同样生成 `///` 和 `__doc__`
- 模块级 `if`、`for`、`try` 等复合语句里的 `def`（M8 之前保留为 Python）同样是 fn item，写在 `exec_module` 里，执行到 `def` 时由 `rt::def!` 绑定，调用经过 globals。名字和模块函数冲突时加 `_` 后缀（`fn helper_`），Python 名字和参数一样用 `as` 给出：`rt::def!(m, helper_ as "helper"(x))?`
- 生成器表达式的 step（§8）也是这样的 fn item，叫 `genexpr`、`genexpr2`

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

装饰器写在 `rt::def!`（嵌套函数是 `rt::function!`）里函数名的前面，和 Python 一样（M11b）

```rust
rt::def!(
    m,
    @(trace(m, &rt::str(py, "outer"))?)
    @(trace(m, &rt::str(py, "inner"))?)
    greet(name, punct = default_value(m, &rt::str(py, "!"))?),
    doc = "Say hello."
)?;
rt::def!(m, @register first())?;
rt::def!(ns, @label.setter label_ as "label"(slf as "self", value))?;
```

- `@name` 和 `@name.attr.attr` 按定义处的规则读名字：模块代码和函数里是 `m.global("name")`，类体里是 `ns.get("name")`，属性是 `.attr("attr")`。只有生成代码本来就是这样读的名字才这样写，名字要能原样写成 Rust 标识符（不是关键字，不是 `_`）。别的表达式（调用、局部变量、builtin）写成 `@(expr)`，局部变量是 `@(&local)`
- 宏按 Python 的顺序执行：从上到下求值装饰器表达式，再求值默认参数、创建函数对象，然后从下到上应用，最后 `rt::def!` 把结果绑定到函数名（`m.set_global` 或 `ns.set`），`rt::function!` 返回它
- 装饰器或默认参数的求值要用到临时变量（lowering 的 prelude）时，这些语句会在宏之前执行，顺序就错了；装饰器或默认参数里有 `:=` 时，赋值的局部变量可能正被前面的装饰器借用。这两种情况仍然是原来的写法：装饰器先放进临时变量，每次应用一条语句

  ```rust
  let decorator = m.global("chosen")?;
  let function = rt::function!(m, walrus())?;
  m.set_global("walrus", decorator.call1((function,))?)?;
  ```

- `--traceback-lines` 时 CPython 报告的行号是装饰器自己的那一行（求值和应用都是），默认参数是 `def` 那一行。只有一个装饰器、默认参数不会出错时，这些都是同一行，宏之前的 `frame.line(N)` 就够了；否则也用上面的写法，每条语句前面是自己的行号。不带 `--traceback-lines` 时都写成宏

被装饰的名字绑定的是装饰器的返回值，所以对它的调用一律走 globals 查找，不做直接调用。已验证 `functools.wraps`、`functools.lru_cache` 和用户自己的装饰器（`tests/cases/decorators.py`、`decorators_uncaught.py`，行号是 `decorator_lines.py`）。

### 注解（PEP 649）

注解不影响生成的 Rust 类型（所有值仍是 `Bound<'py, PyAny>`），只是保存下来供运行时内省。3.14 的注解是惰性求值的，可以引用之后才定义的名字，所以注解以 Python 源码文本的形式写进签名（`width: "float"`），由 runtime 的 `__annotate__` 对象在需要时求值

- `__annotate__(1)`（VALUE）在模块 globals 里 `eval` 每个注解，得到和 CPython 一样的 `__annotations__`，结果缓存
- `__annotate__(3)`（FORWARDREF）对 `NameError` 的注解返回 `annotationlib.ForwardRef`，`__annotate__(4)`（STRING）返回源码文本，字符串字面量的注解返回它的值
- `from __future__ import annotations` 时注解就是字符串，与 CPython 一样
- 注解求值时只看得到模块 globals，引用外层函数局部变量的注解报 `UnsupportedError`

### 注释与源位置

Python 注释保留为 `//` 注释，放在它所属的语句前面：单独一行的注释属于后面的第一条语句，行尾注释属于它所在的语句（移到语句上方），复合语句头部和第一条子语句之间的注释属于那条子语句。模块末尾的注释放在 `exec_module` 末尾。每个函数（包括嵌套函数）前面保留 `// file.py:LINE` 源位置注释。

## 7. 模块、全局变量与 import

- 一个 Python 项目生成一个 crate，每个 Python 模块对应一个 Rust `mod`，模块间可以循环引用（M6，见下面的"项目"）
- 项目里的模块由 runtime 的 meta path finder 在第一次 import 时执行，保持 Python 的惰性 import 顺序和副作用。模块对象、`sys.modules` 和循环 import 都交给 importlib 处理，和从源码 import 一样
- **全局变量不用进程级 static 保存模块对象**，因为 `importlib.reload` 或删除 `sys.modules` 后重新 import 会产生新的模块对象。globals 由 `Function` 持有，包装成模块上下文 `rt::Module` 显式传给每个函数体。以后需要按模块保存的状态（例如混合模式里编译好的 Python 代码）也放在这里
- **只绑定一次的全局名字仍然每次查 globals**（M3b 评估过）。`import os`、`LIMIT = 10` 这类名字在模块里只绑定一次（`ModuleBindings` 能数出来），可以考虑在函数里缓存它们的值，但没有这样做
  - 主要原因是缓存在可读性上没有收益：有了模块上下文，`m.global("LIMIT")?` 已经和 Python 的写法一一对应，缓存只会让生成代码多出缓存槽。函数被改写成原生 Rust 时，常量自然变成 Rust 的 `const`（第 15 节）
  - 动态查找顺带让替换全局变量（`monkeypatch.setattr(app, "LIMIT", 1)`、`mock.patch("app.requests")`）照常生效，函数在名字绑定之前被调用时也照常抛 `NameError`。但这不是完整的 monkeypatch 支持：对模块函数的直接调用本来就看不到替换（下面和第 12 节），所以不能拿它作为不缓存的理由
  - 缓存而又保持语义的做法是用 dict watcher（`PyDict_AddWatcher`）在 globals 或 builtins 变化时让缓存失效，但每个使用点需要一个缓存槽（生成代码里多一个宏或 static），每个解释器最多只有 8 个 watcher，实现和审查的成本都高
  - 性能上，实测一次全局读取 44 ns，CPython 特化后的 `LOAD_GLOBAL` 约 1.5 ns。其中大部分是每次从 `&str` 新建并哈希一个 Python 字符串，所以 `m.global` 和 `rt::builtin` 只接受 `'static` 的名字，按地址缓存驻留字符串，降到 18 ns，语义不变（M11 起缓存是一张按地址索引、读取不加锁的表，属性名也用它）。dict watcher 在 M11 也没有做，理由同上
- **直接调用的条件**：名字在模块内只绑定一次（`def` 算一次），没有装饰器，模块里没有 `from x import *`、`globals()`、对自身模块的 `setattr`、不带命名空间的 `exec`/`eval`，并且调用的参数能在转换时绑定（第 6 节）。否则走 globals 字典动态查找。调用名为 `globals`、`vars`、`locals` 的函数时保守地按 `globals()` 处理，即使这个名字是模块自己定义的函数。带参数的 `vars(obj)`、`locals(..)` 读的是别的命名空间，不算
- **对自身模块的修改**（M3d，`ModuleBindings.patches_itself`）按常见写法保守地识别，原生代码和保留为 Python 的代码都检查：
  - 模块对象是 `sys.modules[__name__]`、`sys.modules["__main__"]`（也包括 `.get(..)`、`from sys import modules` 之后的 `modules[..]`）、`import __main__` 绑定的名字，以及被赋值为这些表达式的名字（不分作用域，传递）
  - 对模块对象 `setattr`/`delattr`、给它的属性赋值或删除属性、读它的 `__dict__`、`vars(模块对象)`
  - 第二个参数是字符串字面量、并且是模块里某个 `def` 的名字的 `setattr`/`delattr`，不管第一个参数是什么
  - 只有一个参数的 `exec`/`eval`（`globals()` 作命名空间时已经按 `globals()` 处理）
  - 识别不到的写法见第 12 节
- 直接调用无法感知项目以外的模块做的 `mock.patch("mod.fn")`，这是有意保留的语义差异（第 12 节）。项目内模块之间的替换按常见写法识别（下面的"项目"）。应用场景里别的模块很少 patch 应用自身的函数，所以影响不大。以后如有需要再提供 `--dynamic-globals` 选项让所有全局调用都走动态查找
- 应用场景下 `importlib.reload` 也很少见，不用 static 保存模块对象主要是为了让同一份生成代码既能作为 `__main__` 运行，也能被别的模块按真实名字 import（M6 的项目里确实会这样）
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
- 相对 import（M6）照 Python 的写法保留前面的点：`from . import x` 是 `rt::import_names(m, ".", &["x"])`，`from ..util import f` 是 `rt::import_names(m, "..util", &["f"])`。runtime 数出点的个数作为 `__import__` 的 `level`，相对于哪个包由 globals 里的 `__package__`/`__spec__` 决定，所以 "attempted relative import with no known parent package" 等错误都由 importlib 自己给出
- `rt::import_from` 失败时的文字照搬 CPython 的 `_PyEval_ImportFrom`：模块的 `__spec__._initializing` 为真时是 "cannot import name 'x' from partially initialized module 'pkg.a' (most likely due to a circular import) (/path/pkg/a.py)"；模块在 `sys.path[0]` 里时，如果和标准库模块同名，或者正在初始化，提示改名；其余情况给出模块的文件位置
- `from x import *` 仍然保留为 Python（§13）

### 项目（M6）

`steelix transpile app.py -o out/` 转换入口脚本和它用到的项目内模块，生成一个 crate，得到的程序和在项目目录里执行 `python app.py` 一样。`steelix transpile -m pkg -o out/` 相当于在当前目录（或 `--root`）执行 `python -m pkg`。

**哪些模块属于项目**（`project.py`）。项目目录是入口脚本所在的目录（解析符号链接），也就是 `python app.py` 的 `sys.path[0]`。从入口开始静态地找 import，按 CPython 的 path finder 在项目目录里找得到源码的模块就属于项目，再从它们继续找：

- `import a.b`（`a` 和 `a.b`），`from a import b`（`b` 是子模块时），相对 import，字面量名字的 `importlib.import_module("a.b")`（包括带 `package` 参数的相对名字）和 `__import__("a")`
- 函数里、`try` 里的 import 也算。转换出来的模块只是登记在表里，运行时真的 import 时才执行，所以多算进来的模块不会改变行为
- 和 path finder 一样，包（`a/__init__.py`）优先于同名的 `a.py`，旁边有同名的扩展模块时不算；builtin 模块和 frozen 模块优先于 `sys.path`，同名的项目文件不算
- 命名空间包（没有 `__init__.py` 的目录，M6 之后）里的模块也算，和 path finder 一样，同一个目录里包和模块都优先于命名空间包。命名空间包本身没有代码，不在表里：它的 Rust mod（`src/ns/mod.rs`）只声明子模块，包对象照常由 `PathFinder` 从目录创建，所以分发里即使 `--no-sources` 也保留这个目录（只带它自己的其它文件，不带子目录里的）。`sys.path` 后面有同名的普通包或模块时（比如项目里的 `json/` 目录和标准库的 `json`）CPython 用后者，命名空间包不存在，所以 finder 对父包不在表里的模块总是检查父包的 `__path__`（见下面的 finder），`tests/projects/namespaces`。`--include 目录` 时这个目录和它上面的目录可以是命名空间包，它下面的子目录仍然要是包，不把任意目录都当成模块
- 不算的：标准库和第三方包（不在项目目录里），编译不过的文件（CPython 在 import 时报 `SyntaxError`，不转换的话运行时也一样），用 `--exclude` 排除的模块。它们运行时照常从源码 import，报告里列出原因
- 名字在运行时才算出来的 import（插件之类）用 `--include` 补充，参数是模块名、`.py` 文件或者目录（目录下所有能 import 的模块）
- 入口脚本被别的模块按名字 import（`import app`）时，它也是项目里的一个模块。CPython 这时会再执行一遍 `app.py`，得到第二个模块对象，`__name__` 是 `app`；生成的程序一样，两个模块对象共用同一个 Rust mod

**布局**。每个 Python 模块一个 Rust mod，按包嵌套，文件和 Python 文件一一对应：

```
app.py                  src/app.rs
store/__init__.py       src/store/mod.rs        （开头是 `pub mod items;` 等子模块声明）
store/items.py          src/store/items.rs
store/reports/text.py   src/store/reports/text.rs
                        src/main.rs             （模块表和 `main`）
```

- 包用 `mod.rs` 而不是 `store.rs` 加 `store/`，这样 `store/__init__.py` 和它的子模块都在同一个目录里，和 Python 的目录结构一样
- Rust 名字尽量和 Python 名字相同。不是标识符的名字（`importlib` 能 import `my-tool.py`）把非法字符换成 `_`，关键字用 `r#`（`type.py` 是 `r#type`，文件仍然是 `type.rs`），`mod.py` 和 crate 根上的 `main.py` 等与生成代码冲突的名字加 `_`，同一个包里撞名时也加 `_`
- 类的 mod（§10）避开同一个文件里子模块的名字和模块自己的名字（clippy 的 `module_inception`），`store/inventory.py` 里的 `class Inventory` 是 `inventory_`
- `main.rs` 里列出所有转换过的模块，模块的源文件由名字推出（`store.items` 是 `store/items.py`）

```rust
const MODULES: &[NativeModule] = &[
    NativeModule::module("config", config::exec_module),
    NativeModule::package("store", store::exec_module),
    NativeModule::module("store.items", store::items::exec_module),
];
```

**finder 和 loader**（runtime 的 `importer.rs`）。`run_project` 在执行 `__main__` 之前把 `NativeImporter` 放进 `sys.meta_path`，位置在 `PathFinder` 前面，这样 builtin、frozen 模块仍然优先，项目模块的优先级和在 `sys.path[0]` 里一样

- `find_spec` 用 `importlib.util.spec_from_file_location` 按原来的源文件创建 spec，所以 `__file__`（项目目录下的绝对路径，项目目录在运行时确定，分发时是 `src/`，§11）、`__cached__`、`__package__`、包的 `__path__` 和 `submodule_search_locations` 都和从源码 import 时一样。importlib 自己负责创建模块对象、放进 `sys.modules`、设置 `_initializing`、失败时删掉
- 每个模块一个 `NativeLoader`（`__loader__`），和 `SourceFileLoader` 一样有 `name`、`path`，实现了 `exec_module`、`is_package`、`get_filename`、`get_source`（读原来的源文件，`inspect.getsource` 和 `linecache` 能用）。`exec_module` 通过 importlib 的 `_call_with_frames_removed` 调用生成的 `exec_module`，所以 import 时出错的 traceback 里没有 importlib 的帧，和源码模块一样
- M7a 起 `NativeLoader` 还有 `get_resource_reader`（和 `SourceFileLoader` 一样返回模块所在目录上的 `importlib.readers.FileReader`，`importlib.resources.files(pkg)` 能读包里的数据文件）、`get_data`（读文件的字节，`pkgutil.get_data` 能用）和 `get_code`。`get_code` 给 `runpy`（`python -m pkg.mod`、`runpy.run_module`）用，返回的代码对象只有一行 `__loader__._exec_in(globals())`，在 runpy 准备好的命名空间里执行生成的 `exec_module`
- 程序里的 finder 回答表里所有模块的 import（`sys.path[0]` 总是项目目录）。附带的解释器（§11 的 `python/bin/python`）也安装同一张表的 finder，但只在 CPython 能找到源文件的时候回答：顶层模块要求项目目录在 `sys.path` 里（空字符串是当前目录），子模块要求包的目录在父包的 `__path__` 里。程序里的 finder 对命名空间包里的模块（父包不在表里）也做这个检查。所以 `cd src && python/bin/python -m pkg.cli` 和 multiprocessing 的子进程（继承父进程的 `sys.path`）执行的是原生代码，在别的目录里 `import pkg` 和 CPython 一样找不到
- 表里没有的模块交给后面的 finder，照常从源码 import：标准库、第三方包、`--exclude` 的模块、转换之后才加进来的文件。包的子模块也一样，`PathFinder` 按包的 `__path__` 找得到
- 循环 import、`importlib.reload`、`importlib.import_module`、删掉 `sys.modules` 里的项之后重新 import 都由 importlib 处理，执行的是同一个生成的 `exec_module`，模块级状态在新的 globals 里，和 CPython 一样。这也是不用 static 保存模块对象的原因（上面）
- 非入口模块的 `__name__` 是它的真实名字，`if __name__ == "__main__":` 照常不执行

**跨模块调用**。调用别的模块的函数（`from helpers import f; f()`、`helpers.f()`）一律经过模块属性或者本模块的 globals（`rt::import_from` 绑定的名字），没有做跨模块的 Rust 直接调用。

- 这样别的模块替换函数（`helpers.f = ..`、`mock.patch("helpers.f")`）照常生效，生成代码也和 Python 的写法一一对应
- 直接调用要求在转换时确定 `f` 一定是 `helpers` 里那个 `def`：导入它的名字在本模块只绑定一次、`helpers.f` 在导入之后没有被替换、`from` import 发生在调用之前，而且循环 import 时 `helpers` 可能还没执行到 `def`。这些条件在转换时证明不了，而经过模块属性只多一次字典查找，所以不做
- 反过来，模块内部的直接调用（上面的条件）要防备别的模块替换它的函数。M6 起在整个项目上检查常见写法（`project.py` 的 `cross_module_patches`）：对项目模块对象（import 绑定的名字、它的属性、`sys.modules["pkg.mod"]`、`importlib.import_module("pkg.mod")`）的属性赋值和删除、`setattr`/`delattr`、`vars()`、`__dict__`，参数是模块对象和它的函数名的调用（`mock.patch.object(helpers, "f")`、`monkeypatch.setattr(helpers, "f", ..)`），以及形如 `"pkg.mod.f"` 的字符串。被替换的函数不再直接调用（`ModuleBindings.patched`），名字不是字面量时整个模块的调用都走 globals。项目以外的代码（例如没有一起转换的测试）做的替换仍然看不到，见第 12 节
- 对自身模块对象的识别也认模块的真实名字：`sys.modules["pkg.mod"]` 和 `import pkg.mod as me`。入口模块同时认 `__main__` 和它被 import 时的名字

## 8. 表达式与求值顺序

Rust 方法调用的形状会改变求值顺序，例如 `x in y` 写成 `y.contains(x)`、`o.m(f())` 写成 `o.method1("m", (f(),))`、`a[i] = f()` 写成 `a.set_item(i, f())`。规则如下

- 多个子表达式中有可能产生副作用的，按 Python 顺序先求值到临时变量。纯名字和字面量不需要临时变量，以保持可读
- 赋值语句先求值右边，再求值目标（`a[i] = f()` 是先 `f()`，再 `a`、`i`）
- 方法调用先取属性再求值参数（`o.missing(f())` 不应调用 `f`）
- 比较：在 `if`/`while` 条件里用 `lt` 等返回 bool 的方法；需要值时（`m = a < b`，numpy 之类会返回非 bool）用 `rich_compare`。链式比较中间操作数只求值一次
- 增量赋值：PyO3 没有 in-place 运算方法，用 runtime 的 `rt::iadd` 等（基于 `PyNumber_InPlaceAdd`），`a[i] += v`、`o.x += v` 中的 `a`、`i`、`o` 只求值一次
- `and`/`or`：值语义的 block 表达式，在条件里优化成 `&&`/`||`

### 实现（M2）

lowering 给每个子表达式记录它在 Rust 里的形态（`lower/values.py` 的 `Value`）：拥有的 `Bound`、`&Bound` 参数、局部变量、可以直接作为 `IntoPyObject` 的字面量、代表 Python bool 的 Rust `bool`（`in`、`is`、`not`、`isinstance`），以及代表 int 的 Rust 整数（`len`）。每个使用点选最可读的写法，例如条件里直接用 Rust `bool`，`len(xs) > 3` 直接写 `xs.len()? > 3`。

`Value` 还记录值的具体类型（`typed`），例如容器字面量是 `PyList`，`*args` 是 `PyTuple`，`str(x)` 是 `PyString`，`iter(x)` 是 `PyIterator`。这很重要：`Bound<PyDict>` 除了 `PyAnyMethods` 还有 `PyDictMethods`，同名时 Rust 优先选后者，而 `PyDict::get_item` 在 key 不存在时返回 `None`，不抛 `KeyError`。`Bound<PyIterator>` 实现了 `Iterator`，`eq`、`lt` 等会解析成 `Iterator` 的方法。所以生成的方法调用都经过 `Value.method(name, ..)`，它查 `rustast/pyo3.py` 的表，被遮蔽且行为不同的方法先 `.as_any()` 擦除类型。行为完全相同的（精确类型上的 `contains`、`len`，dict 的 `set_item`、`del_item`）保留原写法，其中 `len` 不会失败，写成 `args.len()`，不带 `?`。

临时变量是语句前面的 `let`（prelude）。只有 Rust 的求值顺序与 Python 不同、或者一个值要用两次时才引入：

- 只在操作数"不纯"时需要。纯的是局部变量、参数和字面量，它们没有副作用也不会失败。全局变量读取不纯（可能抛 `NameError`，也可能被前面的调用改掉）
- 某个操作数需要 prelude 时，Python 顺序里排在它前面的不纯操作数也一起提升到它的 prelude 之前（`operands`），否则 prelude 会抢先执行
- 临时变量尽量按表达式命名（`let item = ..`、`let index = ..`、全局变量或属性的名字），只使用一次的临时变量直接 move，不 clone
- 只在条件成立时执行的代码有自己的 prelude，放进对应的 block：`and`/`or` 的右侧、条件表达式的分支、链式比较后面的部分。`while` 条件需要 prelude 时生成 `loop { prelude; if !cond { break; } .. }`，`elif` 条件需要 prelude 时生成 `else { prelude; if cond { .. } }`

```rust
// data[trace("i", 1)] = trace("v", 5)
let value = trace(m, &rt::str(py, "v"), &rt::int(py, 5))?;
data.set_item(trace(m, &rt::str(py, "i"), &rt::int(py, 1))?, value)?;
// g() in h()
let item = g.call0()?;
h.call0()?.contains(&item)?
```

临时变量不能比它所在的 Python 语句活得久（M8a，`lower/temps.py`）。Rust 的 `let` 活到块结束，模块级就是整个 `exec_module`，而 CPython 在语句用完一个值就释放它，`__del__`、weakref 回调、`weakref.finalize` 都能观察到。每条语句（连同 prelude）生成之后

- 临时变量最后一次使用是一定会执行的 `impl IntoPyObject` 参数（`set_item("k", &value)`）时直接 move 进去（`set_item("k", value)`），在那里释放。最后一次使用本来就是 move 的（`Ok(value)`、`into_any()`、`finally.resume()`）不用处理
- 仍然持有对象的临时变量（被借用、用了两次、只在某些路径上 move）连同语句放进自己的块 `{ let value = ..; .. }`。语句声明了 Python 局部变量时写成 `let x = { ..; value };`。语句总是跳走（`return`、`break`）时不加块
- 不持有对象的临时变量（Rust `bool`、`usize`、引用，`import` 得到的模块，提前查找的 builtin）不处理，因为别处一直持有这些对象
- 块结束后临时变量的名字可以重新使用，后面的语句又从 `lhs`、`value` 开始编号
- 粒度是语句：CPython 可能在语句中途就释放（`print(f() and 1)` 里 `f()` 的结果在 `print` 之前释放），这里要等语句结束。Python 局部变量对应的 Rust 变量不受影响，和 CPython 一样活到重新赋值或者函数返回

```rust
// x = y = make("pair")
{
    let value = make(m, &rt::str(py, "pair"))?;
    m.set_global("x", &value)?;
    m.set_global("y", &value)?;
}
// h.attr += 1
{
    let h = m.global("h")?;
    h.set_attr("attr", rt::iadd(&h.attr("attr")?, 1)?)?;
}
```

返回值和赋值的清理（M9）。下面这些只在语义上完全等价时做，不改变求值顺序，也不改变任何对象释放的时机（`tests/cases/returned_locals.py` 用 `__del__` 和 weakref 检查）

- 返回或者 yield 一个在同一个 fn 或闭包里声明的局部变量时直接 move（`return Ok(x)`、`return Ok(Some(x))`），不 clone（`rustast/cleanup.py`，在 Rust AST 上做，因为只有那里知道变量在哪个闭包里声明）。返回之后这个变量不会再被用到，对象由返回值继续持有，不会有别的对象因此提前或推迟释放。外层 fn 或闭包的变量（被闭包按引用捕获）、参数（`&Bound`）、`Option` 变量被 guard 借用的变量和 `except` 块的 `rt::Caught`（`let e = frame.catch(err);`）照旧 clone
- 目标和右边个数相同、都没有 `*` 的解包赋值 `a, b = b, a` 不再构造元组再 `rt::unpack`，写成 `[a, b] = [b.clone(), a.clone()];`。Python 也是先按顺序求值右边的每一项，再依次赋值，中间的元组看不到（CPython 对两三项的情况本来就不构造元组）
- `globals()` 是模块持有的 `&Bound<PyDict>`，直接用 `m.globals()`，不再 `.clone().into_any()`（`m.globals().contains("n")?`）
- 带装饰器的类和以前带装饰器的函数一样，最外层的装饰器在保存时调用：`m.set_global("Point", decorator.call1((point,))?)?`（M11b 起函数的装饰器写在 `rt::def!` 里，§6）
- 先声明后赋值的局部变量（`let kind: Bound<'py, PyAny>;` 和 `let mut x: Option<..> = None;`，§5）移到第一次用到它的语句之前（`rustast/cleanup.py` 的 `declare_late`，在每个函数 lower 完之后做）。第一次使用就是给它赋值时在那里带值声明，对应 clippy 的 `needless_late_init`。第一次使用可以是 `x = v;`，也可以是带 `else` 的 `if`/`else if` 链或 `match`（`try` 生成的），要求每个分支的最后一条语句给它赋值，或者离开（`return`、`break`、`continue`，`raise` 也是 `return Err(..)`），分支里别的地方和条件里都不用到它。嵌套的 `if` 只要符合同样的条件也算赋值。分支里的赋值变成块的尾表达式，至少要有一个分支给出值

  ```rust
  let word = if n.lt(0)? {
      rt::str(py, "negative")
  } else if n.eq(0)? {
      return Ok(rt::str(py, "zero"));
  } else {
      rt::str(py, "positive")
  };
  ```

  声明原来不持有对象（或者只持有 `None`），所以求值顺序和释放时机都不变。分支里的临时变量和以前一样在分支结束时释放，值由变量接着持有。声明只在同一层块里往下移，不移进循环或分支，变量照旧活到函数返回。往下移时可以越过注释、别的语句、临时变量和别的局部变量的声明，后者会改变函数返回时两个变量的释放顺序（§5、§12）。几个声明移到同一条语句之前时保持原来的先后。类型标注去掉，由值推断（分支的值本来就是这个类型），只有 `Option` 变量的值里可能有 `None` 时保留。后面还会被赋值时保留 `mut`，否则去掉

  标准库 154 个模块里先声明后赋值的变量从 3445 个变成 3137 个，合并了 308 个（M9）。剩下的第一次使用是没有 `else` 的 `if`、循环，或者分支最后赋值的是别的变量（一条 `if` 给两个变量赋值时，排在后面的那个合并，前面的保留声明），都移到了使用之前。M9 时因为 `co_varnames` 的顺序而不能合并的 `if` 链有 23 个，比如 `userhome` 和 `drive` 由同一条 `if` 赋值，`drive` 的声明必须排在 `userhome` 后面。M14 之后去掉这个限制时重新数了一遍（M14 转换的代码比 M9 多，基数不同）：没有值的声明从 3774 个变成 3746 个，多合并了 28 个

量过但没有做的（标准库 154 个模块，32 万行）：`.clone()` 共 2422 个，其中 1150 个是被重新赋值的参数 `let mut n = n.clone();`（参数是 `&Bound`），约 400 个是 `x = y.clone()`（两个 Python 变量都持有这个对象，move 会让对象在 `x` 被重新赋值时提前释放），约 270 个是返回参数（`&Bound`，只能 clone），约 170 个是条件表达式和 `and`/`or` 的值形式里的局部变量，都是必要的。临时变量在 lowering 时已经只在求值顺序需要时才引入，3052 个临时变量里能原样内联而不改变求值顺序的只有装饰器的 `let decorator`/`let function`（650 多个，内联之后是一个很长的嵌套调用，不比现在好读；M11b 把它们写进了 `rt::def!`，见 §6）和上面的装饰类

具体写法

- 方法调用：参数都是纯的时写 `o.method1("m", (x, 1))?`（没有参数时 `o.method0("m")?`），否则写 `o.attr("m")?.call1((f()?,))?`，保证先查找方法。不把参数先放进临时变量再写成 `method1`（M11 的 review 建议过，可以省掉 bound method）：那样参数在查找方法之前求值，`o` 的 property、`__getattr__` 或者找不到方法时的 `AttributeError` 都能看出顺序不同，`o.missing(f())` 会先调用 `f`。省掉 bound method 要在 runtime 里按 `_PyObject_GetMethod` 先查找再调用，见第 6 节"调用的开销"
- `in`/`not in` 用 `contains`，`is` 用 `Bound::is`，与 `None` 比较用 `is_none()`。它们的结果是 Rust `bool`，需要对象时才 `rt::bool(py, ..)`
- `and`/`or` 在条件里是 `&&`/`||`，右侧有 prelude 时写成 block。需要值时生成 `if` 表达式，左操作数先放进临时变量，决定结果时直接 move 出来；左操作数是 Rust `bool` 时不需要临时变量。左操作数是常量时在转换时就决定。作为语句的 `a or f()` 生成 `if !a.is_truthy()? { f()?; }`
- 链式比较：`a < b < c` 在条件里是 `a.lt(&b)? && b.lt(c)?`；中间操作数需要临时变量时，它和之后所有用到它的比较放在同一个 block 里：`a.lt(b)? && { let rhs = f(py)?; b.lt(&rhs)? && rhs.lt(d)? }`。需要值时第一个比较结果放进临时变量，`if result.is_truthy()? { b.rich_compare(c, ..)? } else { result }`
- 一元运算用 PyO3 的 `neg`/`pos`/`bitnot`，`not` 是 `!cond`
- 增量赋值的 runtime 函数按 `operator` 模块命名：`rt::iadd`、`rt::itruediv`、`rt::ior` 等。`x += v` 写成 `x = rt::iadd(&x, v)?;`
- 容器字面量用 PyO3 的构造函数：`PyList::new(py, [1, 2, 3])?`、`PyTuple::new(..)`、`PySet::new(..)`、`[("a", 1)].into_py_dict(py)?`，空的用 `PyList::empty(py)`、`PyDict::new(py)`。元素都是同类型的字面量时直接写，否则统一写成 `&Bound`。所有元素先求值再构造，与 `BUILD_LIST`/`BUILD_MAP` 一致
- 带展开的容器字面量（M8a）用 runtime 的宏，写法接近 Python：`rt::list!(py, a, *b)?`、`rt::tuple!(..)`、`rt::set!(..)`、`rt::dict!(py, **d, (k, v))?`。宏按书写顺序求值，连续的普通元素成组加入，`*x` 和 `LIST_EXTEND`/`SET_UPDATE` 一样迭代，`**m` 和 `DICT_UPDATE` 一样合并（后面的 key 覆盖前面的），不可迭代、不是 mapping 时的报错与 CPython 一致。`tuple!` 先建 list 再转换，与 CPython 相同
- 下标 `get_item`/`set_item`/`del_item`，属性 `attr`/`set_attr`/`del_attr`（`rt::Attributes`，见第 4 节）。切片用 `rt::slice(py, start, stop)?` 和 `rt::slice_step(..)`，省略的部分是 `py.None()`，因为 `PySlice::new` 只接受整数，而 Python 的切片对象保留 `None`
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

### 推导式、生成器表达式与 `:=`（M8）

实现在 `lower/comprehension.py`，生成器对象在 runtime 的 `generator.rs`。

**list、set、dict 推导式**按 3.12 之后 CPython 的方式内联（PEP 709）：不创建函数，循环直接写在语句的 prelude 里，结果放在一个临时变量里。推导式的变量是循环体里的 `let`，不会泄漏到外面，也不遮蔽外面同名的变量；多个 `for` 是嵌套的循环，`if` 是嵌套的 `if`（后一个条件不需要 prelude 时合并成 `&&`）：

```rust
// return [x * x for x in xs]
let list = PyList::empty(py);
for x in rt::iter(xs)? {
    let x = x?;
    list.append(x.mul(&x)?)?;
}
Ok(list.into_any())

// return {k: v for k, v in d.items() if v % 2 == 0}
let dict = PyDict::new(py);
for item in rt::iter(&d.method0("items")?)? {
    let [k, v] = rt::unpack(&item?)?;
    if v.rem(2)?.eq(0)? {
        dict.set_item(&k, &v)?;
    }
}
Ok(dict.into_any())
```

- 作用域和 CPython 一样：第一个 `for` 的可迭代对象在外层求值，其余部分在推导式里。`symtable` 把推导式的变量算作外层函数的局部变量，分析（`frontend/scope.py` 的 `walk_scope`）把推导式里对它们的使用排除在外，所以 `[x for x in x]`、和全局变量同名的推导式变量都是对的。函数里推导式变量的 Rust 名字用函数的重命名表，模块级和类体里另取不冲突的名字，推导式结束后临时变量可以再用这个名字
- 类体里的推导式看不到类的命名空间（第一个可迭代对象除外），名字直接查 globals 和 builtins，和 CPython 一样抛 `NameError`
- dict 推导式先求值 key 再求值 value（3.8 起的顺序），重复的 key 后者覆盖前者。set 推导式的 `set.add(..)` 是 `PySet` 自己的方法，生成的方法调用带 `concrete` 标记，检查器（§2）不要求擦除类型
- 和普通的 `for` 一样用 `rt::iter` 迭代，取每个后续元素之前检查信号
- 行号：错误报告在外层帧里。默认（§9）就是外层函数的 `def` 行。`--traceback-lines` 时行号是出错的部分所在的行：推导式跨行时，第一个可迭代对象、条件、元素前面各写 `frame.line(N)`，迭代体改变了行号时在结尾设回 `for` 的行（取下一个元素时出错报这一行），推导式结束后设回语句的行。推导式写在一行里时没有额外的 marker
- 推导式里定义的 lambda 或生成器表达式捕获推导式的变量时，这个变量是一个在循环之前创建的 `rt::Cell`，所有迭代共用（`[lambda: i for i in range(3)]` 的三个函数都返回 2），和 CPython 内联之后的行为一样
- `locals()`、`vars()`、`dir()`（不带参数）和不带命名空间的 `exec`/`eval` 在推导式里会看到推导式的变量，所在的函数或语句保留为 Python
- 变量在绑定它的 `for` 之前就被读到的推导式（`[y for x in xs for y in f(y)]`，CPython 抛 `UnboundLocalError`）保留为 Python

**生成器表达式**是真正惰性的迭代器，不物化成列表。创建时只求值第一个可迭代对象并调用 `iter()`，其余部分在每次 `next()` 时执行，异常也在那时抛出，捕获的变量在执行时才读（late binding）。生成的代码是一个 fn item，叫 *step*，它从迭代器取下一个元素，返回下一个值，迭代器耗尽时返回 `None`；`rt::genexpr!` 用它和迭代器创建生成器对象：

```rust
// return any(x > 3 for x in xs)
fn genexpr<'py>(
    m: &rt::Module<'py>,
    xs: &Bound<'py, PyIterator>,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    rt::frame!(m, "has_big.<locals>.<genexpr>", 10, |_| {
        if let Some(x) = rt::next(xs)? {
            return Ok(Some(x.rich_compare(3, CompareOp::Gt)?));
        }
        Ok(None)
    })
}
rt::builtin(m.py(), "any")?.call1((rt::genexpr!(
    m,
    genexpr(xs.try_iter()?),
    qualname = "has_big.<locals>.<genexpr>"
)?,))
```

- 有 `if` 时 step 是一个 `for` 循环，`return` 从循环中间返回，下次调用继续迭代同一个迭代器（位置保存在 Python 的迭代器里），所以不需要状态机：

  ```rust
  for word in rt::iter(words)? {
      let word = word?;
      if word.is_truthy()? {
          return Ok(Some(word.method0("upper")?));
      }
  }
  Ok(None)
  ```

- 捕获的变量和嵌套函数一样写在 `genexpr[n, &count](..)` 里（第 6 节），step 的参数是 `&Bound` 或 `&rt::Cell`。迭代器参数按第一个可迭代对象命名（`xs`、`words`），不是名字或属性时叫 `iter`
- step 在自己的帧里运行，traceback 里有 `<genexpr>` 帧，qualname 是 `f.<locals>.<genexpr>`、`C.<genexpr>` 或 `<genexpr>`，和 CPython 一样；迭代出错报第一个可迭代对象的行
- 生成器对象（`rt::GenExpr`，`#[pyclass]`）实现了调用方能看出来的行为：迭代、`send`（未开始时只能 `send(None)`）、`throw`（包括三参数形式的 `DeprecationWarning`）、`close`、`gi_running`、`gi_suspended`，重入时抛 `ValueError: generator already executing`，step 里抛出的 `StopIteration` 换成 `RuntimeError: generator raised StopIteration`（PEP 479，`__cause__` 是原来的异常），`__name__`、`__qualname__` 可写，`repr` 是 `<generator object f.<locals>.<genexpr> at 0x..>`。它支持弱引用，参与循环垃圾回收（`__traverse__`/`__clear__`）。耗尽、出错或关闭之后释放迭代器，所以 `break` 之后 `close()` 或者 `del` 会让底层的 generator 立刻收到 `close()`
- 状态放在 `Mutex` 里，锁只在取出、放回迭代器时短暂持有，执行 Python 代码（step、迭代器的析构）时从不持有
- 只有一个 `for` 的生成器表达式是原生的。多个 `for` 的需要在两次调用之间保存外层循环的元素和内层迭代器（一个小状态机），标准库里只有 3 个，保留为 Python。生成器表达式的变量被它里面的 lambda 或生成器表达式捕获时（cell 要和生成器一样长寿）也保留为 Python
- 常见的消费者（`sum`、`any`、`all`、`"".join`、`sorted`、`list`、`tuple`、`dict`、`max`/`min` 的 `key`、`next`、`itertools.islice`）不做特化，照常调用 builtin。`list(x for x in xs)` 改写成推导式之类的特化在语义上不等价：少了 `<genexpr>` 帧和 PEP 479 的转换，名字也可能被重新绑定（见下）

**`:=`** 在 prelude 里赋值，然后读这个变量。推导式和生成器表达式里的 `:=` 绑定外层的函数或模块作用域（PEP 572），生成器表达式里是 step 捕获的 cell：

```rust
// ys = [y for x in xs if (y := x * 2) > 2]，y 可能没有赋值
let list = PyList::empty(py);
for x in rt::iter(xs)? {
    let x = x?;
    let y2 = x.mul(2)?;
    y = Some(y2.clone());
    if y2.gt(2)? {
        list.append(rt::local(&y, "y")?)?;
    }
}
```

- 函数里被 `:=` 赋值的局部变量，读取不再算"纯"的（§8 开头），同一个表达式里前面的读取先放进临时变量：`y + (y := 10) + y` 是 `let y2 = y.clone(); y = rt::int(py, 10); y2.add(&y)?.add(&y)?`
- 定值分析（第 5 节）按求值顺序处理 `:=`：`if (n := len(xs)) > 1:` 之后 `n` 是确定赋值的，推导式条件里的 `:=` 之后不是（推导式可能一次也不迭代）

否决的方案

- 生成器表达式物化成列表（原计划）：`any()` 的短路、无限迭代器、late binding 和异常时机都会变，不能接受
- runtime 用一个闭包逐个过滤、映射元素（`filter_map` 风格）：取元素时出错就没有 `<genexpr>` 帧，`if` 和元素也不在同一个函数里，不如一个 step fn 好读
- 每层循环一个 `Box<dyn FnMut>` 闭包：生命周期和 `Send` 的约束带来很多噪音，可读性差
- 把生成器对象做成 `unsendable` 的 pyclass：在别的线程上使用会 panic，而 CPython 的生成器可以跨线程传递；用 `Mutex` 就够了
- `list(genexp)`、`"".join(genexp)` 等特化成推导式或直接循环：见上，语义不完全一样，收益只是少创建一个对象
- 改用生成器函数的机制（`rt::def!(.., gen ..)`、`rt::yield_`，M14 之后的 review 测过）：生成的代码和生成器函数一样，多个 `for` 也不用保留为 Python，但每个元素慢很多。`sum(x * 2 for x in xs)`（release，LTO）用 step 约 35 ns/元素，同样的函数体写成原生生成器约 63 ns（CPython 约 22 ns）。多出来的是每次恢复的固定成本，分散在状态的取出和放回、通道和 waker、C 栈检查、正在处理的异常的保存和恢复、几层 async 状态机里，去掉 C 栈检查、异常状态和状态的 `Mutex` 也只到约 59 ns，所以目前保留 step

### generator 函数（M8 调研，M13 改了结论）

含 `yield`、`yield from` 的函数到 M12 为止整个保留为 Python，M8 之后它们是剩下的主要障碍（`coverage.md`，M13 之后的数字也在那里）：

- 标准库 154 个模块里只因为 `yield`/`yield from` 保留的函数有 129 个（全部 6466 个函数的 2.0%），都支持的话原生函数从 96.4% 到 98.4%。6 个工具里剩下的 11 个保留函数全是 generator，支持后是 100%
- 标准库这些模块里共有 134 个 generator 函数（含嵌套的）。没有一个用到 `yield` 表达式的值（`x = yield`，即 `send()` 驱动的协程写法）；24 个的 `yield` 在 `try` 里，4 个在 `with` 里，29 个用了 `yield from`。`yield` 不在 `try`/`with` 里、也没有 `yield from` 的有 86 个。6 个工具里分别是 11、0、2、0、2、7

M8 考虑过三种做法，M13 加了第四种

1. **状态机**：和生成器表达式的 step 一样，每次 `next()` 调用一个 Rust fn，但函数体要能从任意一个 `yield` 之后继续。局部变量要搬进生成器对象里（`st.total` 而不是 `total`），函数体按 `yield` 切成几段，循环里的 `yield` 要把循环改写成显式的状态转移（`match st.state { .. }`），`try`/`finally`、`with` 跨过 `yield` 时还要记住待执行的清理，`close()` 要在暂停的 `yield` 处抛 `GeneratorExit` 并执行 `finally`，`yield from` 要转发 `send`/`throw`/`close`。Rust 稳定版没有 generator（`gen` 块和 `Coroutine` 只在 nightly），状态机得自己生成。语义可以做到和 CPython 一致，可读性代价最大：生成的代码不再和 Python 逐句对应，也很难手工改写，而可读性是 steelix 的第一目标
2. **另一个栈上运行**：函数体照常生成直线代码，`yield` 写成 `co.yield_(value)?`，`send`/`throw`/`close` 就是带值或带异常恢复执行，代码最好读。用 OS 线程实现时每次切换都要交出、取回 GIL（每个元素微秒级），生成器的代码在另一个线程上运行，线程局部的状态（`contextvars`、`decimal` 的上下文、`threading.local`）和 CPython 不同，traceback 也要跨线程拼起来；用有栈协程（`corosensei` 一类）在同一个线程里切换栈，则会让 CPython 按线程记录的 C 栈边界检查（3.12 起的栈保护）失效，greenlet 能做是因为它针对每个 CPython 版本处理解释器内部状态。两种都要引入 unsafe 的依赖，正确性风险大
3. **保留为 Python**（到 M12 为止的做法）：没有代价，`send`/`throw`/`close` 和帧都是 CPython 自己的，只是这 2% 的函数不能逐步改写成 Rust
4. **用 Rust 稳定版的 `async` 当状态机**（M13）：函数体写成 `async fn`，每个 `yield` 是 `rt::yield_(&x).await?`，状态机由 rustc 生成，就是做法 1 想要的东西，但生成的代码和做法 2 一样是直线代码，和 Python 逐句对应。不需要执行器（executor），也不需要另一个栈：`send`/`throw`/`close` 各 poll 一次 future，`Poll::Pending` 只表示"有一个值要交给调用方"。`try`/`finally`、`with` 跨过 `yield` 时的清理就是 Rust 的控制流本身，`close()` 在暂停的地方抛 `GeneratorExit`，`finally` 照常执行。unsafe 只有 runtime 里集中的几处（下一节），没有栈切换，也不碰解释器的内部状态

M8 的结论是暂时保留为 Python，理由是做法 1 的可读性代价和做法 2 的正确性风险。做法 4 两样都没有，而且同一套机制也能做 `async def`（下一节），所以结论改为：生成器函数和协程都按做法 4 原生转换，M13 先在 runtime 里做了 PoC，再做了 lowering（下一节）。上面"用户最终多半会改成 `Iterator`"的顾虑仍然成立，但原生的生成器至少让周围的函数可以照常改写，生成器本身的改写方式见下一节

### 协程和生成器（M13）

M13 先做了 runtime 的 PoC（手写"生成代码"，和 CPython 比较输出），再做了 lowering。PoC 的程序现在是 e2e 用例 `tests/cases/coroutines.py`，由 lowering 生成，另外有 `async_basics`、`async_with_for`、`async_uncaught`、`async_kept`、`generators` 几个用例。

生成的代码，以 `async def` 为例：

```rust
// async def fetch(session, url):
//     resp = await session.get(url)
//     return await resp.text()
pub async fn fetch<'py>(
    m: &rt::Module<'py>,
    session: &Bound<'py, PyAny>,
    url: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "fetch", 1, async |_| {
        let resp = rt::await_(session.method1("get", (url,))?).await?;
        rt::await_(resp.method0("text")?).await
    })
}
rt::def!(m, async fetch(session, url))?;
```

生成器函数一样，`yield x` 是 `rt::yield_(&x).await?`，`x = yield` 的值就是它的结果（送进来的值），单独的 `yield` 交出 `rt::none(py)`，`yield from it` 是 `rt::yield_from(it).await?`，值是委托对象的返回值，`return v` 成为 `StopIteration.value`。定义写成 `rt::def!(m, gen count_up(n))?`（`gen` 是 2024 edition 为 `gen` 块保留的关键字，宏里可以直接用）：

```rust
// def running_total():
//     total = 0
//     while True:
//         x = yield total
//         if x is None:
//             return total
//         total += x
pub async fn running_total<'py>(m: &rt::Module<'py>) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "running_total", 1, async |_| {
        let mut total = rt::int(py, 0);
        let mut x: Bound<'py, PyAny>;
        loop {
            x = rt::yield_(&total).await?;
            if x.is_none() {
                return Ok(total);
            }
            total = rt::iadd(&total, &x)?;
            py.check_signals()?;
        }
    })
}
rt::def!(m, gen running_total())?;
```

lowering（`lower/expr.py` 的 `suspension`、`lower/handling.py`、`lower/stmt.py` 的 `async_for`）的规则

- 函数体里有 `yield`、`yield from` 的 `def` 是生成器函数，`async def` 是协程（`frontend/scope.py` 的 `function_kind`）。两者的 Rust fn 都是 `pub async fn`，参数和同步函数一样，函数体总在 `rt::frame!(.., async |frame| ..)` 里（即使不会出错也要帧：没开始就 `throw` 的异常要在第一条语句之前抛出），没有 `RecursionGuard`（下面的"递归深度"）。`rt::def!`、`rt::function!` 的签名前加 `async` 或 `gen`，方法（`ns`）、装饰器、嵌套函数和捕获的变量都和同步函数一样。前端把 `async def`、`async for`、`async with` 换成 `def`、`for`、`with` 节点的子类（`frontend/desugar.py`），作用域、绑定和区域的分析照常处理它们
- `await e` 是 `rt::await_(e).await?`。`e` 的 Rust 表达式会留下临时值时（在调用结果上调用方法、借用不是变量的值、宏），先用 `let` 绑定（下面"GC 看得到的东西"）：`let sleep = m.global("asyncio")?.method1("sleep", (0,))?; rt::await_(sleep).await?;`，名字取被调用的函数名，被占用时用 `awaitable`。`rt::yield_` 的值和 `rt::yield_from` 的对象同理（`value`、`iterable`）。局部变量、参数、字面量和 `x.method0("f")?` 这类表达式不留下临时值，直接写在括号里
- `try` 的区域（§9）和 `with` 的函数体只有在会暂停（里面有 `await`、`yield`、`yield from`、`async for`、`async with`，不算嵌套函数）时才变成 `async`，其余和同步函数完全一样。会暂停的 `try` 部分是 `let result: PyResult<..> = async { .. }.await;`，再 `match result` 或 `frame.finally(result)`：`match async { .. }.await` 推断不出 `?` 的错误类型，类型要写在 `let` 上。只有一个表达式的 `try`（§9 的 A 形式）直接 `match` 那个表达式，它是 PyO3 或 runtime 返回的 `PyResult`，需要先执行语句时是 `match async { ..; rt::await_(x).await }.await`。会暂停的 `with` 是 `frame.with_async(&manager, async |value| ..).await?`。`return`、`break`、`continue` 和同步代码一样经过 `Flow`
- `async with` 是 `frame.async_with(&manager, async |value| ..).await?`，几个 item 嵌套。`__aexit__`、`__aenter__` 按 CPython 的顺序在类型上查找，找不到、返回的对象不能 await 时的 `TypeError` 和 3.14 的文字一样（包括"Did you mean to use 'with'?"）
- `async for` 写成一个块，结束时释放迭代器，然后才执行 `else`：

  ```rust
  {
      let iter = rt::aiter(&xs)?;
      while let Some(x) = rt::anext(&iter).await? {
          ..
          py.check_signals()?;
      }
  }
  ```

  `rt::aiter` 是 `GET_AITER`，`rt::anext` 是 `GET_ANEXT` 加上等待它的结果，`StopAsyncIteration` 变成 `None`，报错的文字和 `__cause__`/`__context__` 都和 CPython 一样
- 原生协程之间的直接调用只有 `await f(x)`，原生生成器之间只有 `yield from g(x)`（`frontend/resolve.py`，按第 7 节的直接调用条件），写成 `rt::delegate(f(m, &x)).await?`。调用图里在同一个环上的写成 `rt::delegate(Box::pin(f(m, &x))).await?`（`CallGraph.in_cycle`）。别的调用（`c = f(x)`、`list(g())`、`await` 别的协程的生成器）经过函数对象，创建协程或生成器对象，和 CPython 一样
- 保留为 Python 的（§13，覆盖率报告里的特性名）：异步生成器（`asynchronous generator`，`async def` 里有 `yield`，需要 `asend`/`athrow`/`aclose` 和 `sys.set_asyncgen_hooks`）、异步推导式（`asynchronous comprehension`）、推导式和生成器表达式里的 `await`（`await in a comprehension`，CPython 把列表推导式变成异步推导式，lowering 又会把它放进循环）、lambda 里的 `yield`（`yield in a lambda`）、`types.coroutine` 装饰的生成器（`types.coroutine generator`，它的 `yield from` 委托给协程，`rt::yield_from` 和普通生成器一样拒绝）、`except` 子句里的 `await`（`await in an except clause`，异常类型在同步闭包 `frame.matches` 里求值）、嵌套函数的装饰器和默认值里的 `await`（`await in a decorator or default`，很少见，不值得为它确认宏的展开）

- **签名和普通函数一样**，参数是 `&rt::Module`、`&Bound`，返回 `PyResult<Bound>`，只是多了 `async`。调用会创建 future，future 比调用活得久，所以模块、捕获的变量和参数由协程对象持有（`rt::coroutine::Locals`，放在 `Rc` 里），`rt::def!` 生成的 `start` 返回一个 `async move` 块，它持有另一个 `Rc`，用引用调用这个 `async fn`（自引用的 future 是 `async` 块允许的）。按值捕获的变量在调用时从 cell 里读出，`&count` 这样的 cell 原样保留，和同步函数一样。直接调用因此和同步函数一样写，只是外面套一层 `rt::delegate`（下面的"直接 await 和递归"）：`rt::delegate(add_later(m, &x, &rt::int(py, 1))).await?`
- **没有执行器。** `rt::Coroutine`（Python 里的类型名是 `coroutine`）和生成器的对象各自持有一个 `Pin<Box<dyn Future>>`。`send(v)` 把 `v` 放进这一次 poll 的通道（`Channel`，在 `send` 的栈上），poll 一次：future 返回 `Pending` 时把通道里待交出的值返回给 Python，`Ready(Ok(r))` 抛 `StopIteration(r)`，`Ready(Err(e))` 抛 `e`。`throw(e)` 一样，只是放进通道的是异常。waker 什么都不做，它的数据指针是通道的地址，`rt::await_` 这些 future 通过它找到通道（再和线程局部的"当前通道"比较，不是当前这一次 poll 的就不用）。所以进展只来自 Python 调用 `send`/`throw`/`close`，驱动它的可以是 asyncio，也可以是任何手写的 trampoline（`tests/cases/coroutines.py` 里有一个不用 asyncio 的循环）
- **`rt::await_(x)`** 是手写的 future，按 PEP 492 取 `x.__await__()`（CPython 的协程和 `types.coroutine` 的生成器直接用，`'int' object can't be awaited`、`__await__() returned non-iterator of type 'int'` 等文字和 3.14 一样），然后按 PEP 380 委托：通道里的值用 `PyIter_Send` 转给它，异常转给它的 `throw`，`GeneratorExit` 先 `close()` 它再在本地抛出（`_gen_throw` 的规则），它 yield 的值放进通道并返回 `Pending`，`StopIteration(r)` 变成 `Ready(r)`。`rt::yield_from` 是同一个 future，只是取 `iter(x)`，并且和 CPython 一样拒绝协程对象。`rt::yield_(v)` 第一次 poll 把 `v` 放进通道返回 `Pending`，第二次返回送进来的值或抛出送进来的异常
- **对象的行为和 CPython 一样**：没开始时只能 `send(None)`，重入时 `ValueError: coroutine already executing`，暂停在 await 上的协程再被 await（Python 的 `await c`、`rt::await_(c)`，原生的和 CPython 的协程都一样）时 `RuntimeError: coroutine is being awaited already`，结束后再用 `RuntimeError: cannot reuse already awaited coroutine`（生成器是 `StopIteration`），函数体里的 `StopIteration` 变成 `RuntimeError: coroutine raised StopIteration`（PEP 479），没开始就 `throw` 时异常在第一条语句之前抛出（traceback 里有这个函数的 `def` 行），`close()` 在暂停处抛 `GeneratorExit`、`finally` 照常执行，捕获后又暂停的抛 `RuntimeError: coroutine ignored GeneratorExit`，捕获后 `return` 的值是 `close()` 的结果（3.13）。在 `except` 里暂停时 `throw`、`close` 和 asyncio 的取消送进来的异常以这个块正在处理的异常为 `__context__`（CPython 的 `_PyErr_ChainStackItem`，用 `PyErr_SetObject` 断开环），有委托对象时是从委托对象出来的异常。函数体自己没有正在处理的异常时不设置，调用方正在处理的异常不算（`Channel` 记着这一次 poll 函数体有没有自己的异常）。`__await__` 返回 `coroutine_wrapper` 那样的迭代器。`cr_running`、`cr_suspended`、`gi_running`、`gi_suspended`、可写的 `__name__`/`__qualname__`（调用时取自函数对象）、`repr`。`StopAsyncIteration` 的转换只属于异步生成器，协程没有。类型的 `__name__` 是 `generator`、`coroutine`、`coroutine_wrapper`，`__module__` 是 `builtins`，`__doc__` 和 CPython 一样（对象的是 None，`coroutine_wrapper` 有 CPython 那句说明），不是 Rust 的文档注释（`#[classattr] __doc__`）。PyO3 的 `tp_name` 总是带模块（`builtins.generator`），报错信息（`cannot pickle 'generator' object`、`'generator' object is not subscriptable`）用的是它，所以 runtime 创建对象时把它换成不带模块的名字（`plain_type_name`），生成器表达式的对象也一样
- **释放时的行为**：开始了没结束的协程和生成器在释放时 `close()`，`finally` 的输出顺序和 CPython 一样（`del` 时、`gc.collect()` 回收循环时），出错时经过 `sys.unraisablehook`，消息是 CPython 的 `Exception ignored while closing generator <..>`；从没开始的协程发出 `RuntimeWarning: coroutine '..' was never awaited`（经过 `warnings._warn_unawaited_coroutine`，调用方是 Python 代码时位置也一样，原生代码没有 Python 帧，§12）。CPython 在 `tp_finalize` 里做这些，PyO3 的类不能声明它，PyO3 的 `tp_dealloc` 也不调用终结器，所以和 M11 的 vectorcall 一样，在创建第一个对象时补上类型对象的 `tp_finalize`，并把 `tp_dealloc` 换成先调用 `PyObject_CallFinalizerFromDealloc` 再调用 PyO3 原来的那个
- **`asyncio` 接受它们**：`collections.abc.Coroutine` 的 `__subclasshook__` 按方法认出 `rt::Coroutine`，所以 `asyncio.iscoroutine` 为 True，`create_task`、`gather`、取消都能用。函数对象的 `__code__` 带 `CO_COROUTINE`（生成器是 `CO_GENERATOR`），`inspect.iscoroutinefunction`、`inspect.isgeneratorfunction` 为 True。`inspect.iscoroutine` 为 False（§12）
- **帧**：`rt::frame!` 的闭包可以是 `async` 闭包（Rust 1.85 起稳定），宏替它 `.await`。帧和函数体一样长寿，跨过多次暂停的异常只有一项，行号的 marker 照常工作，经过 `await` 往外传的异常的 traceback 和 CPython 一样（测试比较了帧的名字和行号）。`try`/`except` 和 `try`/`finally` 的 `try` 部分从同步代码的立即调用的闭包 `(|| { .. })()` 换成 `async { .. }.await`，`with` 的函数体会暂停时用 `frame.with_async(&manager, async |value| { .. }).await?`
- **正在处理的异常**：CPython 的生成器有自己的异常状态（`gi_exc_state`），在 `except` 里暂停时 `sys.exception()` 不会泄漏给调用方，恢复后又回来，自己没有时看到调用方的。runtime 在 poll 前后做同样的保存和恢复（`State::Suspended` 的 `handled`），所以 `except E:` 里 `await` 之后 `raise F` 的 `F.__context__` 仍然是 `E`。`rt::Handling` 和同步代码完全一样，结束时恢复它开始时的异常，跨过暂停的块因此恢复的是开始那一次 poll 的调用方的异常，而不是结束时的调用方的（§12）。函数体在自己的 poll 以外被释放时（暂停在 `except` 里的生成器被释放或被 GC 清理），`Body` 在 drop 前后保存和恢复线程的正在处理的异常，块里的 `Handling` 不会改掉释放它的地方的状态。M13 曾经让 `Handling` 记住"之前的异常是调用方的"，并让直接委托的内层和 `throw` 进委托对象时看到调用方的异常，M14 之后的 review 为了简单去掉了这些（§12）
- **直接 await 和递归**：两个原生的 `async def` 之间满足第 7 节直接调用条件的 `await f(x)` 直接 await Rust 的 future，没有 Python 对象；原生生成器之间的 `yield from g(x)` 也一样，内层的 `rt::yield_` 用的是同一次 poll 的通道，交出的值直接到外层的调用方，`send` 和 `throw` 也直接到内层。其余的经过函数对象和 `rt::await_`、`rt::yield_from`。递归的 `async fn` 要 `Box::pin`，lowering 按调用图给递归的直接 await 加上：`rt::delegate(Box::pin(countdown(m, &n.sub(1)?))).await?`
  - 直接 await 的 future 外面套一层 `rt::delegate`，它写明这里是委托，并且补上 CPython `_gen_throw` 对委托对象的特殊处理（M13/M14 之后的修复，`generator_close` 用例）。`close()` 和 `throw(GeneratorExit)` 不是简单地把 `GeneratorExit` 送到最里层：CPython 先关闭委托对象（`gen_close_iter`，内层收到一个新的 `GeneratorExit`，它的返回值丢掉），然后不管内层怎样结束，都在外层的 `yield from`/`await` 处抛出 `GeneratorExit`；内层又暂停时是 `RuntimeError: generator ignored GeneratorExit`，同样在外层的委托处抛出，内层随即被释放并再次关闭（终结器）；关闭内层出错时抛出那个错误。`rt::delegate` 在被暂停的 future 收到 `GeneratorExit` 时照这样做，多层直接委托时每一层各做一次，和 CPython 递归的 `gen_close` 一样。`throw` 进来的别的异常照常送到内层，从内层出来的异常（包括关闭内层的 `GeneratorExit` 和错误）在外层的委托处按外层正在处理的异常重新设置 `__context__`（`_gen_throw` 的 `gen_send_ex(.., exc=1)`），和 `rt::await_` 一样
  - 直接委托共用一个通道和一份异常状态，内层自己没有正在处理的异常时看到的是外层的。CPython 往委托对象里 `throw` 时外层的帧不在执行，内层看到的是 `throw()` 的调用方的。`rt::await_` 调用委托对象的 `throw`、`close` 时同样是外层的异常（§12）
  - 委托对象不是生成器或协程时，`throw()` 的参数原样传给它的 `throw` 方法（`g.throw(ValueError)` 传的是类，不是实例），和 `_gen_throw` 一样。生成器和协程（CPython 的和原生的）收到规范化之后的异常对象
- **递归深度**：和同步函数的直接递归一样只检查 C 栈（§12）。每次 `send`/`throw`/`close` 恢复函数体时（`Core::resume`）用 `Py_EnterRecursiveCall` 检查一次，经过 Python 对象的 `yield from`、`await` 链每一层都经过这里；直接委托（`rt::delegate`，包括 `Box::pin` 的递归）在同一次 resume 里嵌套 poll，所以 `rt::delegate` 每次 poll 也检查一次。链太深时抛 `RecursionError: Stack overflow (used N kB) ...`，debug 和 release 构建都不会崩溃（`coroutines` 用例的 10 万层链）。M13 曾经按 3.14 的 `PyThreadState` 布局读写 Python 的递归计数，让最大深度和 CPython 完全相同，代价是依赖解释器的内部布局、每次 send 多一次检查，而程序很少依赖确切的深度，所以 M14 之后的 review 去掉了它。启动时就超过深度的协程和 CPython 一样直接结束，不会再发出 never awaited 的警告。`rt::await_` 遇到原生协程时直接调用它的 `__await__` 的 Rust 实现，不经过 Python 的方法调用，因为那次调用自己的递归检查可能在协程开始之前失败，留下一个没有开始的协程和 never awaited 的警告（CI 上偶发过）
- **GC 看得到的东西**：future 对 GC 是不透明的，它自己持有的引用 GC 看不见，被这样引用的对象一定活着，经过它的循环永远不会被回收。所以函数体暂停时 runtime 需要的对象都放在协程对象里，由 `__traverse__` 访问：模块的 globals、捕获的变量和参数（`Locals`，future 只借用）、函数体正在处理的异常、`rt::await_` 和 `rt::yield_from` 委托的迭代器、函数体的 traceback 帧对象（后两个在 `Held` 里，runtime 的 future 只拿一个 key，只在这个协程自己的 poll 里取用；`Frame` 也改成借用 globals，不再 clone）。协程对象有 `tp_clear`（PyO3 的 `__clear__`），先 drop future，再 drop 其余的。测试覆盖了模块的 globals 里从没开始的生成器、暂停的生成器和协程，以及处理过异常后暂停的协程在退出时的终结，方法生成器经过 `self` 的循环，以及 `self.task = create_task(self.run())` 这样的循环（任务等着一个只有 `self` 引用的 `asyncio.Event`），结果和 CPython 一样。函数体自己的代码跨过暂停持有的东西仍然看不见（§12）。其中语句的临时值是 lowering 要注意的：Rust 的临时值活到语句结束，也就是跨过这条语句里的 `.await`，`rt::await_(m.global("helper")?.call1((x,))?).await?` 里的函数对象 `helper` 就这样被 future 持有，经过它的 `__globals__` 又回到模块。CPython 在暂停前已经用完了它们。所以 lowering 在被 await 的表达式会留下临时值时先用 `let` 绑定可等待对象（`let helper = ..; rt::await_(helper).await?`，`rt::await_` 按值接收后自己保留），`rt::yield_` 的值和 `rt::yield_from` 的对象同理（上面的规则）。同一条语句里先于 `await` 求值的别的操作数（`f(a) + await g()` 里 `f(a)` 的结果，相当于 CPython 值栈上的东西）和直接 await 的实参仍然跨过暂停，和局部变量一样 GC 看不见（§12）
- **改写时**（§15）：生成器函数通常会改写成 Rust 的 `Iterator`，适配层把它包成 Python 的迭代器即可，不需要这里的机制。改写过的协程如果 await 了 Rust 生态的 future（例如 tokio 的），那个 future 返回 `Pending` 之后不会被唤醒（waker 什么都不做），runtime 会报 `RuntimeError: coroutine '..' awaited a Rust future that is not driven by Python`。需要的话可以加一个桥接的 helper，用 asyncio 的 future 当 waker：Rust future 返回 `Pending` 时交出一个 asyncio future，waker 被调用时（可能在别的线程）用 `call_soon_threadsafe` 完成它。只有手工改写的代码需要，生成的代码不会用到

**生命周期和 unsafe。** future 存在 Python 对象里，必须是 `'static`，而生成的代码用的是 `Bound<'py, ..>`。runtime 在创建协程时用 `Python::assume_attached()` 得到 `Python<'static>`，把模块、闭包和参数重新绑定到这个 token，再调用存下来的 `for<'py> fn(..)`（`AsyncBody` 的 `capture` 和 `start`），也就是用 `'py = 'static` 实例化函数体。这是唯一的一处，理由是：future 只在 `send`/`throw`/`close` 和终结器里 poll，只在这些地方、对象释放或被 GC 清理时 drop，都是 attach 的线程，free-threaded 的构建本来就编译不过（§6），所以 attach 就是持有 GIL；函数体对所有 `'py` 都成立，不能把这个 token 或对象存到 future 以外的地方，对象离开 future 时是 `Py<PyAny>`（交出的值）或重新绑定到调用方的 token（结果）。其余的 unsafe 也都集中在 `coroutine/` 里，各有 SAFETY 注释：future 的 `Send`（它和 `Locals` 的两个 `Rc` 一起在对象的 `Mutex` 后面，poll 时取出，只在 attach 的线程上使用，和 CPython 一样可以在别的线程上恢复。前提是函数体跨过 `.await` 持有的只有 Python 对象和 `Send` 的值，生成的代码满足，手工改写的代码要遵守，§15）、`Held` 通过当前通道的 owner 找到协程（只在 owner 就是正在 poll 的协程时，它的 `resume` 在栈上）、waker（函数不使用数据指针）、通过 waker 找到通道（先和线程局部的当前通道比较）、`tp_finalize`/`tp_dealloc` 的替换、`PyIter_Send` 和 `PyErr_FormatUnraisable` 的 ffi。

否决的方案

- PyO3 的 `experimental-async`：waker 绑定 asyncio，`throw` 直接丢掉 future（`finally` 不执行），也不能委托给 Python 的 awaitable
- 第一个参数传一个 `co` 对象让函数体 `co.yield_(..)`：通道通过 waker 找到，生成代码里不需要多一个参数
- 参数按值传（`session: Bound<'py, PyAny>`）：直接调用时到处是 `.clone()`，和同步函数的写法也不一样

### match 语句（M14）

Python 的模式全是运行时的检查（`Py_TPFLAGS_SEQUENCE`、`len()`、`.get()`、`isinstance`、`__match_args__`、`getattr`、`==`、`is`），Rust 的 `match` 对不上。对得上的是 let chains（edition 2024）：每个 `case` 是一个 `if let .. && ..`，模式的每一层是链上的一节，和 Python 原文一行对一行。

```rust
// match message:
//     case {"type": "click", "pos": [x, y], **rest}:
//         ...
//     case {"type": "key", "key": str(key)} if len(key) == 1:
//         ...
if let Some([type_, pos]) = rt::match_map(message, ["type", "pos"])?
    && type_.eq("click")?
    && let Some([x, y]) = rt::match_seq(&pos)?
    && let rest = rt::match_rest(message, ["type", "pos"])?
{
    ..
} else if let Some([type_, key2]) = rt::match_map(message, ["type", "key"])?
    && type_.eq("key")?
    && let Some([key]) = rt::match_class(&key2, &rt::builtin(py, "str")?, 1, [])?
    && key.len()? == 1
{
    ..
}
```

runtime 的 helper（`crates/steelix-runtime/src/matching.rs`）各做 CPython 的一条指令对一层模式做的事，一次交出这一层的所有值（`Option<[Bound; N]>`），链再按深度优先、从左到右检查子模式。所以求值顺序、短路和报错都和 CPython 一样（对照 3.14 的 `Python/codegen.c`、`Python/ceval.c`）。

- 序列：`rt::match_seq::<N>` 是 `MATCH_SEQUENCE`、长度相等和 `UNPACK_SEQUENCE`（迭代取值），`rt::match_seq_star::<B, A>` 是 `UNPACK_EX`，带名字的星号得到 list，返回 `([..; B], rest, [..; A])`。只有通配符时只检查长度（`rt::match_seq_len`，`[*_]` 连 `len()` 都不调用）。有 `*_` 而别的子模式要取值时，和 CPython 一样不解包，`rt::match_seq_min` 之后按下标取 `subject.get_item(i)?`，星号之后的是 `rt::item_from_end(&subject, k)?`（`subject[len(subject) - k]`，每次重新调用 `len()`），取值和子模式交替进行。自定义的 Sequence 能看出这些区别，`match_order` 用例逐一比较过
- 映射：key 全是字面量时是 `rt::match_map(subject, ["a", "b"])`（`MATCH_MAPPING`、`len() >= N`、`MATCH_KEYS`），别的 key 是 `rt::match_mapping(subject, n)` 之后先求值 key（`&& let key = ..`）再 `rt::match_keys(subject, (&key,))`，和 CPython 一样在映射检查之后才求值值模式。`MATCH_KEYS` 用 `subject.get(key, 哨兵)`，第一个找不到的 key 就停下，两个相等的值模式抛 `ValueError: mapping pattern checks duplicate key (..)`。`**rest` 是 `rt::match_rest`，`dict.update` 之后删掉匹配过的 key
- 类：没有子模式时是 `rt::match_instance`，否则 `rt::match_class(subject, cls, 位置子模式的个数, ["关键字", ..])`，返回先位置后关键字的值。不是类型时抛 `TypeError: called match pattern must be a class`，只有有位置子模式时才读 `__match_args__`（必须是 tuple，元素必须是 str，个数不够时 `C() accepts 1 positional sub-pattern (2 given)`），重复的属性 `TypeError`，`getattr` 的 `AttributeError` 算不匹配，`bool`、`int`、`str`、`list` 等内置类型（`_Py_TPFLAGS_MATCH_SELF`）的一个位置子模式匹配主体本身。错误文字都和 3.14 一样（`match_errors`）
- 字面量用 `==`，主体在左边（`value.eq(1)?`，`COMPARE_OP` 之后 `TO_BOOL`），`None`/`True`/`False` 用 `is`。`-1`、`1 + 2j` 和 CPython 一样当成常量。值模式（`Color.RED`）走到那一步才求值
- 不带捕获的 OR 模式写成 `||`，嵌套在别的模式里时每个分支是 `matches!(rt::match_seq(&item)?, Some([a, b]) if a.eq(1)? && b.eq(0)?)`
- 通配符和没被读到的值用 `_`，模式里不绑定名字的中间值取名 `item`（被局部变量占用时 `item2`），类的关键字子模式用属性名，映射的值用 key 的名字
- 主体是不会被重新赋值的局部变量或参数时直接用，否则先 `let subject = ..;`

捕获的名字按 CPython 的顺序（后序，`as` 的名字在里面的名字之后，`**rest` 最后）在整个模式成功之后、guard 之前存进变量，所以 guard 失败时也已经绑定，`match` 之后能读到。存法有两种

- 只在这个 case 的 guard 和体里用到的名字（`LocalsInfo.case_scoped`，和 `for` 的 `loop_scoped` 一样的分析）直接用 `if let` 绑定的名字，和链上别的节一样
- 别的（`match` 之后还读、模块级和类体里的名字、cell）先赋给局部变量（`x = Some(x2);`、`m.set_global`、`ns.set`），再求值 guard。这时 guard 不能是链上的一节（失败也要已经赋值），整个 `match` 写成带标签的块，每个 case 一个 `if`，体结束时 `break 'match_`

```rust
// match values:
//     case [x, y] if x > y: ...
//     case [x, *_] if x < 0: ...
'match_: {
    if let Some([x2, y2]) = rt::match_seq(values)? {
        x = Some(x2);
        y = Some(y2);
        if rt::local(&x, "x")?.gt(rt::local(&y, "y")?)? {
            ..
            break 'match_;
        }
    }
    if rt::match_seq_min(values, 1)?
        && let x2 = values.get_item(0)?
    {
        x = Some(x2);
        if rt::local(&x, "x")?.lt(0)? { .. }
    }
}
```

没有这种 case 时整个 `match` 是 `if .. else if .. else` 链，最后一个不会失败的 case 是 `else`，体是空的就省掉。guard 里的 `:=`、生成器表达式、lambda 照常（嵌套的 fn item 放在 `match` 之前）。`match` 可以出现在模块级、类体、协程和生成器里（`match_async`），`return`、`break`、`continue` 照常。`--traceback-lines` 时每一节之前标出它所在的行（多行的模式里出错的子模式所在的行，guard 的行），和 CPython 一样（`match_uncaught`）。链的第一节的 marker 写在 `if` 之前，后面的 case 因此是 `else { frame.line(n); if .. }`（clippy 的 `blocks_in_conditions` 不允许整个条件是一个块）。

保留为 Python 的（§13，覆盖率报告里的特性名）：带捕获的 OR 模式（`OR pattern with captures`，每个分支要绑定同样的名字，链上表达不出来）和有不会失败的分支的 OR 模式（`OR pattern with an irrefutable alternative`，CPython 只允许它在最后，很少见）。`match_kept` 用例。

否决的方案：每个 case 一个闭包返回 `Option<(..)>`，名字和值离得太远；Rust 的 `match` 加 guard，所有检查都在 guard 里，等于没用上模式。

## 9. 异常（M4）

目标是让最常见的写法读起来像手写的 Rust，少数几种固定的结构覆盖其余情况。运行时的部分在 `crates/steelix-runtime/src/frame.rs`，生成的部分在 `lower/handling.py`。

### 帧与行号

原生函数运行时没有 Python 帧。和 Cython 一样，异常离开函数时再把这个函数补进 traceback

- 每个函数体包在 `rt::frame!(m, "qualname", def 所在行, |frame| { .. })` 里，模块体用 `<module>`，行号是 1。`RecursionGuard` 在它外面。函数体里不可能出错的（没有 `?`）不包。函数体不用 `frame`（没有 `raise`、`except`、`finally`、`with` 等）时闭包参数写 `|_|`
- 异常离开函数（或者到达 handler、`finally`、`with` 的 `__exit__`）时，`Frame` 用 `PyCode_NewEmpty` 和 `code.replace(co_qualname=..)` 创建 code 对象（每个函数只创建一次，`PyOnceLock` 缓存），`PyFrame_New` 创建帧，再用 `types.TracebackType(tb_next, frame, -1, line)` 插到 traceback 前面。同一次调用只创建一个帧对象，异常已经带有这个帧的条目时（在这里 `raise`，或者裸 `raise` 重新抛出）不再重复添加，和 CPython 一样
- 默认（M9a 起）traceback 里原生帧的函数名、文件和帧的顺序都和 CPython 一样，行号是 `def` 所在的行（有装饰器时是第一个装饰器的行），显示的源码行也是这一行。例外是 `except` 的类型不是 builtin 异常类时，`frame.matches(&err, LINE, ..)` 要在 `except` 那一行求值类型，之后的行号停在这一行
- `--traceback-lines`（`CrateOptions.traceback_lines`）恢复精确到语句的行号，是调试和验证用的模式，e2e、`tests/apps`、`tests/projects` 的测试都用它和 CPython 逐行比较 traceback
  - 每条可能出错的语句前写 `frame.line(N);`，只是写一个 `Cell<u32>`。复合语句的 marker 写在头部（`if`、`while` 的条件，`for` 的迭代），循环体正常走完时把行号设回循环头，和 CPython 在 `JUMP_BACKWARD` 处的行号一致。不会出错的语句、`break`/`continue`、和上一条行号相同的语句不写
  - 多行语句的行号取值所在的那一行（`return`、赋值、表达式语句取值的第一行），traceback 只显示这一行源码。`tb_lasti` 是 -1，所以原生帧没有 `^^^` 定位，这是和 CPython traceback 唯一的文字差异
  - 实现在 lowering 里（`StmtLowering.at_line` 和推导式的 `at_line`）：两种模式的行号记录完全一样，只是默认不输出 marker，`rustflow` 的分析本来就跳过 marker，所以两种模式生成的代码结构相同。除了 marker 和闭包参数（`|frame|`/`|_|`），唯一的区别是去掉 marker 后 `else { if .. }` 会合并成 `else if`。`rt::frame!` 的闭包体总是带花括号，只有一个表达式时也写 `|_| { expr }`，不让 rustfmt 把长表达式拆在宏调用的几行里
- 为什么默认不写 marker
  - 噪音：M9a 之前 marker 占 golden 非空行的 11.6%，转换后的标准库工具（`tests/apps`）的 14.1%。去掉之后 golden 从 20493 行变成 18114 行，`tests/apps` 的输出从 11041 行变成 9419 行
  - 会过期：生成的代码是要给人改的（第 15 节），改过的 Rust 里 marker 很容易和 Python 源码对不上，这时 traceback 会很肯定地报一个错的行号，比只报 `def` 行更误导人。`def` 行不会错，函数名加上 Rust 的源码已经足够定位
- 示例（`total_size`，第 24 行起），默认

  ```rust
  rt::frame!(m, "total_size", 24, |_| {
      let mut text: Bound<'py, PyAny>;
      let mut total = rt::int(py, 0);
      for path in rt::iter(paths)? {
          let path = path?;
          text = read_first_line(m, &path)?;
          total = rt::iadd(&total, text.len()?)?;
      }
      Ok(total)
  })
  ```

  `--traceback-lines`

  ```rust
  rt::frame!(m, "total_size", 24, |frame| {
      let mut text: Bound<'py, PyAny>;
      let mut total = rt::int(py, 0);
      frame.line(26);
      for path in rt::iter(paths)? {
          let path = path?;
          frame.line(27);
          text = read_first_line(m, &path)?;
          frame.line(28);
          total = rt::iadd(&total, text.len()?)?;
          frame.line(26);
      }
      Ok(total)
  })
  ```

- 否决的方案
  - `#[rt::frame]` 属性宏代替 `rt::frame!` 闭包：只省 2% 到 3% 的行，要多依赖一个 proc macro crate，函数体里的 `frame` 也会凭空出现
  - 只在调用 Python 的语句前写 marker：不可靠，运算符也会调用用户的 `__mod__` 等方法，几乎每条语句都可能进入 Python 代码
  - 每个函数一张行号表，语句处只写表的下标：每条语句仍然要一个 key，噪音没有少
  - 用 `#[track_caller]` 或 Rust 源码行号报告 Rust 的位置：rustfmt 和手工修改都会打乱和 Python 行号的对应关系。直接在 traceback 里报 Rust 文件和行号以后可以考虑，但那是另一个产品决定（traceback 指向 Rust 而不是 Python）
  - proc macro 改写函数体，或者把每个 `?` 改写成带行号的形式：生成的代码不再是普通 Rust，也增加编译时间
  - 为每条语句生成伪造的行号表（`co_linetable`），可以得到多行显示和 `^` 定位，但要在生成代码里放一张静态位置表，噪音太大
  - 在调用期间压入真正的解释器帧：没有公开 API，见下面的 logging
  - 在 Rust AST 上做后处理删掉 marker（M9 原来的计划）：lowering 里不输出更简单，`|frame|`/`|_|` 也能一起决定
- 每次调用的代价是 `frame!` 里几个字段的初始化，`--traceback-lines` 时再加每条语句一次 `Cell::set`，`fib(27)` 和 `for i in range(n): total += i` 在 release 下测不出差别（M4 前后都是 21 ms 和 130 ms）
- logging、warnings 仍然看不到原生帧。它们读的是线程状态里正在执行的解释器帧（`sys._getframe`、`PyEval_GetFrame`），原生函数运行时这个链上没有它，而在调用期间伪造一个解释器帧没有公开 API（`_PyInterpreterFrame` 是内部结构，`f_lineno` 只能在 trace 函数里设置），所以只记录为已知差异（第 12 节）

### raise

- `raise X` 写成 `return Err(frame.raise(X));`，`raise X from Y` 写成 `frame.raise_from(X, Y)`，`from None` 传 `py.None()`。`X` 可以是类（会实例化，构造结果不是 `BaseException` 实例时抛 `TypeError`）、实例、或者 `PyErr`，`__cause__`、`__context__`、`__suppress_context__`、`__traceback__` 都和 CPython 一致（`__context__` 由 `PyErr_SetObject` 设置）
- builtin 异常类加字面量参数直接写成 PyO3 的写法：`frame.raise(PyValueError::new_err("bad value"))`。`assert` 同样写成 `frame.raise(PyAssertionError::new_err(..))`
- 裸 `raise` 写成 `frame.reraise()`，重新抛出当前正在处理的异常（`PyErr_GetHandledException`），所以在 handler 调用的辅助函数里也能用。没有正在处理的异常时抛 `RuntimeError: No active exception to reraise`

### try/except

两种结构。第一种（下面叫 A）是 `try` 里只有一条语句，形如 `x = expr`、`return expr` 或者 `expr`，并且 handler 不需要异常对象：直接对表达式的结果 `match`

```rust
let int = rt::builtin(py, "int")?;
match int.call1((s,)) {
    Ok(value) => x = value,
    Err(err) if err.is_instance_of::<PyValueError>(py) => x = rt::int(py, 0),
    Err(err) => return Err(err),
}
```

- 表达式里查找 builtin 或全局名字的 `?` 提到 `match` 前面（`let int = ..;`），这样 scrutinee 里没有 `?`。还有别的 `?` 时写成 `(|| expr)()`
- `else` 的语句放在 `Ok` 分支里
- handler 的类型是 builtin 异常时写成 `err.is_instance_of::<PyX>(py)`，元组写成几个 `||`。其余写 `frame.matches(&err, LINE, || handler)?`：先把这一帧补进 traceback 并把异常设为正在处理的异常，再求值 handler 的类型。类型不是异常类时和 CPython 一样抛 `TypeError`，它的 `__context__` 是原来的异常。`LINE` 在 `--traceback-lines` 时是 `except` 所在的行，traceback 指向这一行；默认是 `def` 行，这样求值之后不会留下一个过期的 `except` 行号
- body 不会失败的 `try` 只生成 body（§5）。scrutinee 是不带 `?` 的 `Ok(..)` 时写成 `Ok::<_, PyErr>(..)`，否则 rustc 推断不出错误类型（E0282）
- 最后一个分支 `Err(err) => return Err(err)` 把没有匹配的异常传出去

其余的（B）把 `try` 体放进立即调用的闭包，`match` 闭包的结果。闭包里的 `?` 被 `match` 接住，`return`/`break`/`continue` 由闭包返回 `rt::Flow`

```rust
match (|| -> PyResult<()> {
    ..
    Ok(())
})() {
    Ok(()) => {}
    Err(err) if frame.matches(&err, 66, || m.global("ValueError"))? => {
        let e = frame.catch(err);
        ..
    }
    Err(err) => return Err(err),
}
```

- 闭包里没有会被 `match` 接住的东西（没有 `?`，结果只是最后的 `Ok`/`Err`，例如 `try: raise E`）时不生成闭包：前面的语句直接放在 `match` 之前，`match` 的是结果本身。闭包里有 `let` 时写成 `let result: PyResult<()> = { .. }; match result { .. }`，让这些变量在 handler 执行前离开作用域（clippy 的 `redundant_closure_call`、`blocks_in_conditions`）
- 函数和闭包最后的 `return x;` 写成尾表达式 `x`，最后的 `if`/`match` 每个分支都 `return` 时整体变成表达式（clippy 的 `needless_return`，`lower/rustflow.py` 的 `returns_as_tail`）
- 闭包的返回类型按块里的跳转决定：没有跳转是 `PyResult<()>`；只有 `return` 并且结尾不可达时直接返回值，写 `return Ok(..)`；其余是 `PyResult<rt::Flow<'py>>`，`Flow` 有 `Next`、`Break`、`Continue`、`Return(value)`。外面按 `Flow` 分派，只有一种跳转时写成 `if let rt::Flow::Return(value) = ..? { return Ok(value); }`，否则写 `match`
- handler 的 `let e = frame.catch(err);` 把这一帧补进 traceback，得到 `rt::Caught`：它 deref 成异常对象（`&Bound`），也实现了 `IntoPyObject`、`IntoException`，所以 `e` 和别的局部变量一样用；同时它在块结束前是正在处理的异常（内部是一个 `rt::Handling`）。不用 `as` 时写 `let _caught = frame.catch(err);`。handler 只有 `pass`、赋值字面量这类不会调用 Python 代码的语句时省略这一行
- 异常对象由 `match` 的 `err` 持有，离开 handler（包括 `continue`、`break`）时释放，和 CPython 一样，`__del__` 和参数的 weakref 在下一条语句之前生效。`Frame` 为了不重复添加 traceback 条目，要记住哪个异常已经有这一帧的条目（在这里 `raise` 的、重新抛出的），M11a 之前它持有这个异常，不经过 `frame.catch` 的 handler（`pass`、只有 `continue`）接住之后异常要等函数返回才释放。现在只记地址：异常的地址，和当时它的 traceback 第一项的帧（这一帧，或者裸 `raise` 时正在处理它的那一帧，都还在运行）。异常释放之后新的异常可能用同一个地址，但它是从被调用的函数来的，traceback 的第一项是别的帧，不会被当成已经有条目（`tests/cases/handler_release.py`）
- `rt::Caught` 里的 `rt::Handling` 是 RAII guard：创建时用 `PyErr_GetHandledException`/`PyErr_SetHandledException` 保存并设置正在处理的异常，`Drop` 时恢复。所以 handler 里 `sys.exception()`、`logging.exception()`、`traceback.print_exc()`、新异常的 `__context__`、辅助函数里的裸 `raise` 都和 CPython 一样，从 `?`、`return`、`break`、panic 离开时也会恢复
- `as e` 的名字只在 handler 内使用时就是 handler 里的 `let`，离开 handler 自然消失，这就是 CPython 结尾的隐式 `del e`。模块级的名字写入 globals，handler 结束时 `m.unbind_global("e")?`。函数里 handler 之外也用到这个名字的，所在的函数保留为 Python（`except-as name used outside its handler`，标准库里只有 1 处）
- `except*` 保留为 Python（§13），异常组的拆分和重新组合不值得用 Rust 再实现一遍

### finally

```rust
let result = (|| -> PyResult<rt::Flow<'py>> {
    if item.eq(target)? {
        return Ok(rt::Flow::Return(item.clone()));
    }
    Ok(rt::Flow::Next)
})();
let finally = frame.finally(result);
rt::builtins::print(py, ("checked", &item))?;
if let rt::Flow::Return(value) = finally.resume()? {
    return Ok(value);
}
```

- `try`（连同 `except`、`else`）的结果先存进 `result`，`frame.finally(result)` 在结果是异常时把这一帧补进 traceback，并在 `finally` 块执行期间把它设为正在处理的异常。`finally` 块是外层的普通代码，它自己的 `return`、`break`、`continue`、`raise` 就是外层的跳转，自然覆盖 `try` 里的 `return` 或者吞掉异常，和 CPython 一样。`finally` 块总是跳走时不会走到 `resume`，变量写成 `_finally`
- `finally.resume()?` 在块正常结束时继续 `try` 的结果：重新抛出异常（不再添加 traceback 条目），或者按 `Flow` 跳转

### with

```rust
if let rt::Flow::Return(value) =
    frame.with(&rt::builtin(py, "open")?.call1((path,))?, |f| {
        Ok(rt::Flow::Return(f.method0("readline")?))
    })?
{
    return Ok(value);
}
```

- `frame.with(manager, |value| body)` 在类型上查找 `__enter__`、`__exit__`（`_PyObject_LookupSpecial` 的规则，找不到时的报错和 CPython 一样，包括"支持异步上下文管理协议"的提示），调用 `__enter__`，执行 body，最后调用 `__exit__`：正常结束传三个 `None`，异常时传异常并设为正在处理的异常，返回真值时吞掉异常。`__exit__` 在当前行执行（默认是 def 行，`--traceback-lines` 时是 `with` 所在的行）
- `as` 的目标是只在这个 `with` 里使用的局部名字时，直接是闭包参数 `|f|`；元组、属性、全局变量等目标在闭包开头赋值。多个 item 写成嵌套的 `frame.with`
- body 的跳转和 `try` 一样用 `Flow`。协程里的 `async with` 是 `frame.async_with`，函数体会暂停的 `with` 是 `frame.with_async`（§8 "协程和生成器"）

### 改写成原生代码时的约定

手工改写的函数（第 15 节）对 Python 一侧抛出的异常类型要和原来一样，调用方的 `except` 才能照常工作。原生的错误用 Rust 的 `Result`/`Option` 表达，在 adapter 里转换成原来的 Python 异常，例如 `examples/logsum` 的 `parse_line` 把 `native::NotANumber` 转成和 `int()` 一样的 `ValueError`，生成的 `load` 照常用 `except ValueError` 接住

## 10. 类（M5）

类用动态建类，语义和 `builtins.__build_class__` 完全一样（metaclass、`__prepare__`、`__mro_entries__`、`__set_name__`、`__init_subclass__`、`__classcell__`），方法是第 6 节的 `Function`。为了可读，每个类生成一个 Rust mod，里面是类体 `body` 和每个方法一个 fn，class 语句本身是一个简短的 `rt::class!` 调用。下面是 `class Point(Base)` 生成的代码（节选）：

```rust
pub mod point {
    use super::*;

    // point.py:6
    /// The body of `class Point`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "Point", 6, |_| {
            let __class__ = ns.class_cell();
            rt::def!(ns, __init__(slf as "self", x, y))?;
            rt::def!(ns, @property norm(slf as "self"))?;
            rt::def!(ns, describe[&__class__](slf as "self"))?;
            Ok(())
        })
    }

    // point.py:7
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        x: &Bound<'py, PyAny>,
        y: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.__init__", 7, |_| {
            slf.set_attr("x", x)?;
            slf.set_attr("y", y)?;
            Ok(rt::none(m.py()))
        })
    }

    // point.py:19
    pub fn describe<'py>(
        m: &rt::Module<'py>,
        __class__: &rt::Cell<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "Point.describe", 19, |_| {
            rt::str(m.py(), "point on ").add(rt::super_(__class__, slf)?.method0("describe")?)
        })
    }
}

// 在 exec_module 里 class 语句原来的位置
m.set_global(
    "Point",
    rt::class!(m, "Point"(m.global("Base")?), point::body, line = 6, attributes = ["x", "y"])?,
)?;
```

### 建类

- `rt::class!` 的第二个参数按 `rt::call!` 的写法列出名字、bases 和关键字（`metaclass=`、传给 `__init_subclass__` 的关键字、`*`、`**`），之后是类体 fn 和几个只读的选项：`line` 是 `__firstlineno__`（有装饰器时是第一个装饰器的行），`qualname` 只在和名字不同时给出（嵌套类），`attributes` 是 `__static_attributes__`，`annotations` 是类体里注解的源码，`future_annotations` 跟在 `line` 后面（上面的"方法"）。`__static_attributes__` 的规则由 CPython 的代码生成器决定，`frontend/classinfo.py` 把模块编译一次，从每个类体的代码里读出来
- 宏创建一个 `rt::class::BuildClass` 对象，像 `__build_class__` 一样以名字和 bases 为位置参数调用，所以参数的求值顺序和 `*`/`**` 的报错与 CPython 一致。运行时（`class.rs` 的 `build_class`）逐步复刻 `__build_class__` 的 C 代码：`__mro_entries__`（以及 `__orig_bases__`）、确定 metaclass（`metaclass conflict`）、`__prepare__`（返回值必须是 mapping）、执行类体、调用 metaclass，最后检查 `__classcell__`。错误消息和 CPython 相同
- 编译器在类体前后生成的代码也由运行时补上，顺序相同。之前是 `__module__`、`__qualname__`、`__firstlineno__`、`__doc__`（`from __future__ import annotations` 时还有 `__annotations__`），之后是 `__annotate_func__`、`__static_attributes__`、`__classdictcell__`、`__classcell__`
- 装饰器在 `rt::class!` 之前求值，建类之后倒序应用，每次应用的行号是装饰器自己的那一行，最后绑定名字（与 CPython 一样）
- `type.__new__` 只把 Python 函数类型的 `__new__`、`__init_subclass__`、`__class_getitem__` 自动包成 staticmethod/classmethod，运行时在建类之后对 `Function` 补上同样的包装

### 类体

- 类体在自己的帧里运行（`rt::frame!(ns, "Point", 6, ..)`，traceback 里是 `in Point`，`f_locals` 是命名空间）。`ns: &rt::Namespace` 包着 `__prepare__` 返回的命名空间，名字按 `LOAD_NAME` 的规则查找：先查命名空间（精确的 dict 走快速路径，别的 mapping 用 `__getitem__`，只接住 `KeyError`），再查 globals 和 builtins。读是 `ns.get("x")?`，写是 `ns.set("x", v)?`，`del x` 是 `ns.del("x")?`
- 所以类体里的任意语句（`if`、`for`、`try`、`with`、嵌套类、条件定义的方法、有副作用的调用）都是原生代码，执行顺序和副作用与 CPython 一样
- 方法、lambda 看不到类的命名空间（类作用域的规则），它们引用的名字照常是全局变量或闭包变量。方法里和类属性同名的全局变量读的仍是全局变量
- 私有名字改编（类 `Point` 里的 `__x` 是 `_Point__x`）在 frontend 的 desugar 里按 `_Py_Mangle` 的规则做，和 symtable 一致，生成代码里直接写改编后的名字。`def`、`class` 的 `__name__` 不改编，只改编绑定的名字
- 类体里不带参数的 `locals()`、`vars()`、`dir()`、`exec()`、`eval()`、`globals()`、`super()` 要看调用方的帧，原生的类体没有解释器帧，所在的语句保留为 Python（§13）

### 方法

- 类体里的 `def` 是 `rt::def!(ns, ..)`：创建 `Function`，应用装饰器，用 `ns.set` 绑定到方法名（M11b）。Rust fn 放在类的 mod 里，名字就是方法名，和模块级的函数名、mod 里别的名字冲突时加 `_`（property 的 setter、deleter 是 `label_`、`label__`），Python 名字用 `as` 给出：`rt::def!(ns, @label.setter label_ as "label"(slf as "self", value))?`。Rust 关键字照常是 `r#type`，宏去掉 `r#`。`self` 参数叫 `slf`，写成 `slf as "self"`
- 方法的 `__qualname__` 由 runtime 推出：`rt::class!` 把类的 qualname 交给 `rt::Namespace`，在 `ns` 里定义的函数是 `<类的 qualname>.<名字>`，嵌套类的方法是 `Outer.Inner.method`。只有和推出的值不同时才写 `qualname = ".."`。名字改编过的方法（`__secret` 绑定到 `_Point__secret`）绑定的名字和 `__name__` 不同，仍然写成 `ns.set("_Point__secret", rt::function!(ns, __secret(slf as "self"))?)?`
- 类体里有方法时 CPython 创建 `__classdict__` cell（方法的注解在类的命名空间里查找名字），在 `ns` 里创建函数也是这样。`from __future__ import annotations` 时 CPython 不创建它，这时 `rt::class!` 带上 `future_annotations`，方法同样写 `rt::def!(ns, ..)`
- 方法和模块函数一样接收 `m`。实例属性是 `slf.attr("x")?`、`slf.set_attr("x", v)?`，调用自己的方法是 `slf.method0("describe")?`，都是动态的，和 Python 一样受子类覆盖、`__getattr__`、`__slots__` 影响；方法之间不直接调用
- `staticmethod`、`classmethod`、`property` 和别的装饰器一样处理，不需要特殊支持
- `Function` 有 `__get__`，绑定后是 `types.MethodType`，和 Python 函数一样。M5 还给它加了一个 `__code__`：没有字节码，只描述签名（`co_name`、`co_qualname`、参数个数、`co_varnames`、`co_freevars`、flags），给检查 `co_freevars` 的代码（`dataclass(slots=True)`）用

### `super()` 和 `__class__`

- 用了零参数 `super()` 或 `__class__` 的方法和 CPython 一样捕获类的 `__class__` cell：`body` 开头 `let __class__ = ns.class_cell();`，方法按闭包捕获它（`describe[&__class__](..)`，第 6 节），Rust 签名多一个 `__class__: &rt::Cell<'py>` 参数。cell 作为 `__classcell__` 交给 metaclass，由 `type.__new__` 设为新建的类，所以类装饰器换掉类、metaclass 改动类时，结果都和 CPython 一样
- `super()` 生成 `rt::super_(__class__, slf)?`，第二个参数是方法的第一个位置参数，cell 为空或者不是类型时抛出和 CPython 相同的 `RuntimeError`/`TypeError`。在 classmethod、property、`__init_subclass__` 里同样可用。没有位置参数的方法、类体里的 `super()` 保留为 Python
- 原来的设计是把 `super()` 降级成 `super(Cls, self)`，Cls 取名字绑定的类。没有这样做：类装饰器返回另一个类时，CPython 的 `super()` 用的是 cell 里原来的类，名字绑定的却是新的类，两者不同。cell 的做法和 CPython 完全相同，生成的代码一样短

### 实例状态与继承

实例状态在 `__dict__` 里（或 `__slots__` 声明的描述符里），由 Python 的对象模型管理，所以 pickle、`copy.deepcopy`、`__eq__`/`__hash__`、`__repr__`、`isinstance` 都和 CPython 一样。可以继承任何类：Python 定义的类（包括保留为 Python 的类，反过来也可以），Exception（自定义异常可以在原生代码和保留的代码之间抛出、接住），Enum、NamedTuple、TypedDict、`abc.ABC`、`threading.Thread`、`argparse.Action`，以及多重继承。dataclass 可用（含 `frozen=True`、`order=True`），见下面的注解。

### 注解（PEP 649）

- 类体里的注解编译成类的 `__annotate__`：运行时由 `annotations = [("x", "int")]` 创建 `__annotate_func__`，注解源码在第一次读取时 `eval`，名字先在类的命名空间（`__classdict__` cell）里找，再到 globals。FORWARDREF 格式得到的 `ForwardRef` 带 `owner` 和 `is_class`，和 CPython 一样。dataclass、`typing.get_type_hints`、`annotationlib.get_annotations` 的三种格式都可用
- 有 `from __future__ import annotations` 时，类体开头 `ns.setup_annotations()?` 创建 `__annotations__`，每条带注解的赋值之后 `ns.annotate("x", "int")?`
- 方法的注解按第 6 节处理，它们引用的类里的名字同样经过 `__classdict__` 查找（方法用 `ns` 作为 `rt::def!` 的第一个参数，这样才拿得到这个 cell，也知道类的 qualname）

### 类出现的位置

- 模块级的类、`if` 里或循环里的类、嵌套类都是原生代码。嵌套类的 mod 放在外层类的 mod 里，带 `qualname = "Outer.Inner"`
- 函数里的 `class` 还不支持，所在的函数保留为 Python（标准库里有 9 个函数）。类体要能读外层函数的局部变量，需要把 cell 传给 `body`，以后再做
- 带类型参数的类（`class C[T]:`，PEP 695）整个保留为 Python

### 不选的方案

- `types.new_class` 的语义和 `__build_class__` 相同，但类体要以 Python 回调的形式传进去，`__qualname__`、`__module__`、`__firstlineno__`、`__classcell__` 和类体的帧都要在回调外另外处理。直接复刻 `__build_class__` 的 C 代码，求值顺序和报错更容易对齐
- `#[pyclass]` 不作为默认方案（已验证）：不能继承 Python 定义的类（TestCase、Enum、NamedTuple、Thread 等），不支持 pickle/copy，和 Exception 混合继承会出现 layout conflict，类体内的逻辑、条件定义的类、局部类都无法表达。它是手工改写时的升级路径（第 15 节）。自动升级符合条件的类（只继承 object 或原生类型、有类型注解）也不做：注解不强制，字段的 Rust 类型在运行时可能放不下 Python 实际传进来的值，`#[pyclass]` 还会改变 pickle、`__dict__`、被 Python 类继承等可见的行为，要保持语义就需要两套实现。M12 的 `steelix attrs --pyclass` 只为用户指定的类写一份草稿，列出阻碍升级的事实，留给人完成（§15）
- 方法级混合模式的另一种做法是把保留的方法单独编译成函数，再放进命名空间。没有这样做，因为 `super()`、`__class__`、私有名字改编、`__qualname__` 都依赖编译器把它当作类体的一部分编译。现在的做法见 §13

## 11. 输出模式

两种模式共用生成的 `exec_module(m)`，区别只在 crate 的入口。

和模式无关的生成选项（`steelix transpile` 的参数，`emit.crate.CrateOptions` 的字段）

- `--no-fallback`（`fallback`）：不支持的语句报错，不保留为 Python（§13）
- `--traceback-lines`（`traceback_lines`，M9a）：在可能出错的语句前写 `frame.line(N);`，原生帧的行号精确到语句。默认不写，行号是 `def` 行（§9）。调试转换结果、和 CPython 逐行比较 traceback 时打开

- **bin 模式**（默认，主要场景）：嵌入 libpython 的可执行程序，`main.rs` 只调用 `rt::embed::run_main`
  - 不用 `auto-initialize`，用 `PyConfig` 初始化，设置 `executable`，否则 `sys.path` 里没有 `.venv` 的 site-packages。同时传入真实的 `argv`，`parse_argv = 0`
  - 以 `__main__` 身份执行模块。结束时调用 `Py_FinalizeEx`，这样会 flush stdout 并执行 atexit
  - `SystemExit` 和未捕获异常交给 `PyErr_Print` 处理，退出码与 CPython 一致。之前和 CPython 的 `flush_io` 一样先 flush `sys.stderr` 和 `sys.stdout`（忽略错误），否则 stdout 和 stderr 重定向到同一个文件时，缓冲区里的输出会排在 traceback 后面
  - 生成的 `Cargo.toml` 在 dev profile 下把 pyo3、pyo3-ffi 和 runtime 设为 `opt-level = 3`，生成的 crate 本身设为 1。没有优化的 PyO3 让每个 Python 操作慢几倍；不用 `package."*"`，因为它也会优化 proc macro，让干净构建的时间翻倍。实测（M3a）一个递归加循环的基准：全部 `opt-level = 0` 时 1.39s，现在的配置 0.40s，而 CPython 是 0.18s；干净构建从 11.5s 增加到 15.3s，改一行后重新构建从 0.5s 增加到 1.0s。转换出来的代码每个操作仍然经过 C API，没有字节码解释的开销但也没有 CPython 的特化和内联缓存，在函数被改写成原生 Rust 之前不会比 CPython 快
  - release profile 打开 `lto = "fat"` 和 `codegen-units = 1`。生成代码调用大量很小的 runtime 函数（`attr`、`global`、`frame::run` 等），没有 LTO 时它们跨 crate 内联不了。skime 的 fib 22（release）从 0.92s 降到 0.85s（CPython 0.41s），二进制小 7%。代价是 release 构建变慢：改生成代码后重新构建从 33s 到 51s，干净构建从 44s 到 62s。只影响编译方式，不改变行为和生成的代码
  - 只有 crate 是 workspace 根时 profile 才生效，所以只在独立 crate（默认）里生成
  - 开发构建（在生成的 crate 里 `cargo build`）：build.rs 设置指向构建时 libpython 的 rpath，避免运行时加载到系统的 libpython，并把构建用的解释器（`PYO3_PYTHON`）和项目目录作为 `STEELIX_PYTHON`、`STEELIX_PROJECT` 交给编译器。程序用这个解释器的安装和 venv，只适合在本机运行。分发用的构建见下面的"分发"
  - 与 `python app.py` 一致的脚本信息（M2，M7 改成运行时确定）。`main.rs` 里只有脚本相对于项目目录的名字 `steelix_runtime::script!("app.py")`，宏在生成的 crate 里读取 build.rs 给出的两个路径（分发构建时没有），runtime 在启动时算出下面这些路径。所以 `main.rs` 里没有本机的绝对路径，golden 也不用替换路径
    - `sys.path[0]` 是项目目录（脚本所在目录，解析符号链接），所以脚本旁边还没转换的 Python 模块照样能 import
    - `__main__` 的 `__file__` 是项目目录加上脚本名，是绝对路径（3.9 起 CPython 也是绝对路径），`__cached__` 是 `None`
    - `sys.argv[0]` 和 `__file__` 相同，后面接程序收到的参数，相当于 `python /绝对路径/app.py`。M2 到 M6 保留传给 `steelix transpile` 的写法（相对路径仍是相对路径），但程序不一定在转换时的目录里运行，分发之后更不是，相对路径就指错了地方。`argparse` 的程序名只用文件名，不受影响。二进制自己的真实命令行放在 `sys.orig_argv`
  - 已知差异：`__main__` 的 `__loader__` 不是 `SourceFileLoader`
  - 有项目模块时（M6，§7 的"项目"）`main.rs` 多一张模块表，调用 `rt::embed::run_project`，它在执行 `__main__` 之前安装 finder。只有一个模块时生成的代码和以前完全一样，仍然调用 `run_main`
  - `-m pkg`（M6）调用 `rt::embed::run_module(.., "pkg.__main__")`，照 `runpy._run_module_as_main` 的做法：`sys.path[0]` 是项目目录，先 import 父包，`sys.argv[0]` 和 `__file__` 是 `__main__.py` 的绝对路径，`__main__` 的 `__spec__`、`__package__`、`__loader__`、`__cached__` 来自 `pkg.__main__` 的 spec，然后在 `__main__` 模块里执行生成的代码。`argparse` 由此得到和 CPython 一样的程序名 `python -m pkg`。`-m pkg.mod` 运行普通模块也可以
- **ext 模式**（`--mode ext`，次要场景）：cdylib 扩展模块，替换原 Python 模块。M12 起只用于把转换过的库作为扩展模块发布，改写时用项目自己的测试检查原生代码不再需要它，用的是 bin 的解释器角色（§15 "用项目自己的测试检查"）
  - 有项目模块时（M6），`PyInit_*` 先调用 `rt::init_extension(m, MODULES)` 安装 finder，项目目录是扩展模块所在的目录，其余模块照样惰性执行
  - `--mode ext -m pkg` 生成包形式的扩展模块：`pkg.so` 只导出 `PyInit_pkg`，`init_extension` 给它设置 `__path__`、`__package__` 和 spec 的 `submodule_search_locations`，于是它成了包，相对 import 和 `import pkg.sub` 都能用。包里所有的模块都转换（库的任何子模块都可能被 import），不只是静态 import 到的。这样安装时扩展模块不能和包的源码目录放在一起，否则 `PathFinder` 先找到源码目录
  - 也可以把它安装成包目录里的 `pkg/__init__.so`（M12）。`FileFinder` 认 `__init__` 加上扩展模块后缀的文件，所以 import 之前 spec 就是包，`python -m pkg` 能找到 `pkg.__main__`（M6 遗留的问题：`pkg.so` 的 spec 不是包，`runpy` 在执行它之前就按普通模块处理，于是只有 `python -m pkg.__main__` 可以）。扩展模块后缀排在 `.py` 前面，`pkg/__init__.so` 和源码放在一起时也是它先被找到，包目录里也可以只有它。`init_extension` 看到文件名是 `__init__.*` 时把项目目录往上提到包目录的外面。`PyInit_pkg` 的名字不用变，`ExtensionFileLoader` 按模块名而不是文件名找它

### 分发（M7）

`steelix build app.py`（参数和 `transpile` 一样，也可以 `-m pkg`）转换程序，用 `cargo build --release` 构建分发版本，再把程序需要的东西放进 `dist/app/`，加 `--archive` 另外打成 `dist/app.tar.gz`。生成的 crate 留在 `build/app/`，再次构建是增量的。目前只支持 Linux x86_64。

```
dist/app/app                          可执行程序，rpath 是 $ORIGIN/python/lib
dist/app/python/lib/libpython3.14.so.1.0
dist/app/python/lib/python3.14/       标准库，预编译成 .pyc
dist/app/python/lib/python3.14/site-packages/
dist/app/python/bin/python            sys.executable，指向 ../../app 的符号链接
dist/app/src/                         项目目录，sys.path[0]
```

**定位**。`steelix build` 构建时设置环境变量 `STEELIX_DIST`，build.rs 于是只加 `-Wl,-rpath,$ORIGIN/python/lib`，不设置 `STEELIX_PYTHON`、`STEELIX_PROJECT`，程序里没有构建机器的路径。runtime 发现这两个值都没有时，从可执行文件的位置（`/proc/self/exe`，解析符号链接，和 `$ORIGIN` 一致）找 `python/` 和 `src/`，用 `PyConfig.home` 指定 Python 的安装，`sys.prefix` 就是 `dist/app/python`，`site` 照常把它下面的 site-packages 加进 `sys.path`。项目模块的 `__file__`（§7 的 finder）、`__main__` 的 `__file__` 和 `sys.argv[0]` 都在 `dist/app/src/` 下面。panic 信息里的源码位置用 `--remap-path-prefix` 把 cargo registry 和 runtime 的目录换成 `cargo/`、`steelix/`。这些参数加在用户自己的 rustflags 后面：环境变量里有 `RUSTFLAGS` 或 `CARGO_ENCODED_RUSTFLAGS` 时追加到它们后面（它们会让 cargo 忽略配置文件里的 rustflags），否则用 `--config build.rustflags=[..]` 传给 cargo，cargo 把它和配置文件里的 `build.rustflags` 合并。配置文件里有 `target.<triple>.rustflags` 时它优先，路径不会被替换，只影响 panic 信息。

- Python 的安装放在 `python/` 并保持 python-build-standalone 原来的 `bin/`、`lib/` 结构，这样 `sys.prefix` 是一个普通的 Python 安装，`sysconfig`、`site` 和看安装布局的工具都不用特殊处理
- 源码目录叫 `src/` 而不是程序名，因为程序名已经是可执行文件的名字
- 分发的程序和环境隔离（M7a）：`PyConfig.use_environment = 0`，忽略所有 `PYTHON*` 环境变量（`PYTHONPATH`、`PYTHONHOME`、`PYTHONSTARTUP`、`PYTHONUTF8`、`PYTHONSAFEPATH` 等），`user_site_directory = 0`，不加用户的 site-packages（`~/.local/lib/python3.14/site-packages` 和里面的 `usercustomize`）。效果相当于 `python -E -s app.py`，`sys.flags.ignore_environment` 和 `no_user_site` 是 1。目标机器上为别的 Python 设置的环境变量、用户自己装的包都不应该改变程序的行为，这和 PyInstaller 等工具的做法一致。需要这些变量时（调试、`PYTHONIOENCODING`、`PYTHONUTF8`）设置 `STEELIX_PYTHON_ENV=1`，程序就和 `python app.py` 一样读取它们和用户的 site-packages。`PYTHONHOME` 总是被 `PyConfig.home` 覆盖
- multiprocessing 按 `subprocess._args_from_interpreter_flags()` 把 `-E -s` 传给子进程，所以子进程也是隔离的；设置了 `STEELIX_PYTHON_ENV=1` 时子进程同样读取环境变量
- 附带的解释器 `python/bin/python` 是普通的解释器，读取环境变量，命令行选项（`-E`、`-I`、`-X dev`、`-X utf8` 等）和 python 一样。只是像 venv 一样不加用户的 site-packages（`sys.flags.no_user_site` 是 1），和程序保持一致，也避免用户的包遮蔽分发里的包；`STEELIX_PYTHON_ENV=1` 时照常加上
- 开发构建不做这些改动，和构建用的解释器一样读取环境变量和用户的 site-packages
- 两个角色都先用命令行调用 `Py_PreInitializeFromBytesArgs`，再设置 `PyConfig`。否则 `PyConfig_SetBytesString` 会在没有命令行的情况下提前做 pre-initialization，`-X dev`、`-X utf8`、`-E` 对 pre-initialization 的部分（dev mode、UTF-8 mode、内存分配器）不起作用
- 程序和构建机器的 glibc 链接，目标机器的 glibc 不能比它旧（python-build-standalone 的 libpython 只要求 2.17）。要支持更旧的系统得在旧的系统或容器里构建

**用哪个 Python**。`--python PATH`（默认是 `$PYO3_PYTHON`，再没有就是运行 steelix 的解释器）同时决定构建用的解释器和分发里的内容：它的基础安装（venv 的 `sys.base_prefix`）提供 libpython 和标准库，它自己（通常是 venv）的 purelib、platlib 提供 site-packages。要求版本和 steelix 转换时用的一样（3.14），并且有共享的 libpython（`Py_ENABLE_SHARED`，python-build-standalone 和 uv 的 Python 都是）。venv 的 `pyvenv.cfg` 里 `include-system-site-packages = true` 时，开发时能 import 的系统 site-packages 不会进入分发，`steelix build` 给出警告。

**site-packages**。复制构建解释器的整个 site-packages，按 `importlib.metadata` 的 RECORD 去掉 steelix 自己（构建工具，editable 安装时是一个 `.pth`）和 pip，不复制 `__pycache__`。没有按 import 分析只挑程序用到的包：插件、entry point、字符串的动态 import 都分析不到，少一个包只能到运行时才发现，多复制的只是体积。所以推荐用一个只装了程序依赖的 venv 构建。`.pth` 里指向 site-packages 以外的绝对路径（别的 editable 安装）在别的机器上不存在，构建时给出警告。

**源文件**。默认把转换过的模块的 `.py` 复制到 `src/`，traceback 的源码行、`inspect.getsource`、`linecache` 和 `NativeLoader.get_source` 都要用它。`--no-sources` 不复制，程序照常运行，traceback 只是没有源码行（`tests/test_dist.py` 测了两种情况）。保留为 Python 的代码（§13）编译的是嵌在程序里的字符串，文件名用 `__file__`，不读文件。不管有没有 `--no-sources` 都会复制运行时要用到的项目文件：项目里每个包目录下除了 `__pycache__` 以外的所有文件（M7a 起不只是 `.py`：数据文件是包的一部分，`importlib.resources`、`pkgutil.get_data` 和按 `__file__` 读的模板都要用；`.py` 是转换时没有静态找到、运行时按算出来的名字 import 的子模块），`--exclude` 的模块、命名空间包、编译不过的文件（`Project.left_out`），以及 `--add-data` 指定的文件或目录。`--no-sources` 只去掉转换过的模块的 `.py`。项目顶层的其它文件不复制，程序读取的数据文件要用 `--add-data`。项目顶层既没有转换也没有复制的 `.py`（比如只被 `importlib.import_module("dy" + "n")` 用到的模块）在分发里 import 不到，`steelix build` 对它们给出警告，建议用 `--include`。

**标准库**。默认不复制 `idlelib`、`tkinter`、`turtle.py`、`turtledemo`、`ensurepip`、`test`（python-build-standalone 的 install_only 本来就没有）、`config-3.14-*`（构建扩展模块用的 Makefile 和 `python.o`）、`lib-dynload/_tkinter*`（Tcl/Tk 没有带上），以及基础安装的 site-packages（pip）。`--full-stdlib` 复制全部。复制之后用构建解释器的 `compileall` 把标准库和 site-packages 编译成 `.pyc`：分发的目录多半不可写，没有 `.pyc` 时每次启动都要重新编译用到的模块。M7a 起用 `--invalidation-mode unchecked-hash`（PEP 552），`.pyc` 不再和源文件的修改时间比较。原来的时间戳校验在 `cp -r`、`scp`、部分解压工具丢掉修改时间之后全部失效，只读目录里每次启动都要重新编译（启动时间从 35 ms 回到 120 ms 左右）。标准库和 site-packages 在分发里不应该被修改，不检查源文件是安全的；真要改的话删掉对应的 `__pycache__`。`src/` 里从源码 import 的文件（上面复制的包里的 `.py`、`left_out`）也编译，用 `checked-hash`：比较源文件的哈希，不依赖修改时间，用户改了也会重新编译。转换过的模块的 `.py` 不编译。`compileall` 并行编译（`-j 0`）需要 POSIX 信号量（`/dev/shm`），失败时改为单进程再试一次；单个文件编译不过（一些包里故意放的测试数据）只给出警告，和从源码 import 时一样到 import 时才报错；`compileall` 本身失败时构建失败。

**`sys.executable` 和 multiprocessing**。`sys.executable` 保持是一个 Python 解释器：开发构建是构建用的 venv 的 python，分发里是 `python/bin/python`，它是指向程序本身的符号链接，程序发现自己以 `python`、`python3`、`python3.N` 的名字运行时，用 `parse_argv = 1` 初始化，然后交给 `Py_RunMain`，行为就是 python 解释器（`-c`、`-m`、脚本、交互模式）。这样应用里常见的 `subprocess.run([sys.executable, "-m", ...])`、`[sys.executable, "-c", ...]` 都能用。解释器也安装项目模块的 finder（§7，只在 CPython 能找到源文件时回答），`--no-sources` 的分发里 `subprocess.run([sys.executable, "-m", "pkg.cli"], cwd=os.path.dirname(__file__))` 照常执行原生代码。环境变量 `STEELIX_NATIVE_IMPORTS=1` 让解释器像程序一样回答所有项目模块的 import，不管项目目录在不在 `sys.path` 上，`steelix test` 用它让测试拿到原生代码（M12，§15）。开发构建同样有这个角色，只是要通过名为 `python` 的链接启动。

- 另一个选择是让 `sys.executable` 就是程序（PyInstaller 的做法），再让程序像 python 一样接受 `-c`、`-m`。但程序自己的参数也可能是 `-c`、`-m`，没法区分，所以只在可执行文件的名字上区分。代价是名为 `python`、`python3` 或 `python3.N` 的程序（脚本叫 `python.py`）不能正常运行
- multiprocessing 的 spawn 和 forkserver（3.14 在 Linux 上默认是 forkserver）启动的子进程用 `multiprocessing.spawn.get_executable()`，默认是 `sys.executable`，命令行是 `python [解释器选项] -c "from multiprocessing.spawn import spawn_main; ..."`（还有 forkserver 和 resource tracker 的 `-c` 命令），子进程会用 python 重新执行原来的 `.py`
- 分发里子进程就是 `python/bin/python` 启动的解释器。解释器发现命令行是这三种 `-c` 之一时，把 `multiprocessing.spawn` 的 `_fixup_main_from_path`、`_fixup_main_from_name` 换成原生的版本，再交给 `Py_RunMain`。CPython 的这两个函数用 `runpy` 把父进程的 `__main__` 从源文件重新执行一遍，作为 `__mp_main__`，函数和类才能 unpickle。原生的版本在路径（或者 `-m` 时的模块名）是这个程序的 `__main__` 时，新建 `__mp_main__` 模块执行生成的 `exec_module`，再让 `sys.modules["__main__"]` 指向它（执行出错时和 `runpy._TempModule` 一样恢复 `sys.modules["__mp_main__"]`），别的情况交给原来的函数。所以子进程不需要源文件
- 开发构建没有这样的解释器（`sys.executable` 是 venv 的 python，不知道生成的代码），子进程只能是程序自己。runtime 在 `sys.meta_path` 最前面放一个 `SpawnFinder`，`multiprocessing.spawn` 第一次 import 时由它包装 loader，模块执行完后调用 `set_executable(程序自己)`，在 `os.environ` 里设置 `STEELIX_MP_CHILD=程序的路径`，再把 `__loader__` 和 `__spec__.loader` 换回原来的，并把自己从 `sys.meta_path` 里去掉。程序只有在命令行是上面三种 `-c`、并且 `STEELIX_MP_CHILD` 等于自己的路径时，才作为子进程（也就是解释器）运行，否则命令行都是程序自己的参数。没有在启动时直接 import，因为 `import multiprocessing.spawn` 要 12 ms，而整个程序启动只要 20 ms 左右
- **为什么不能只看命令行**（M7a）。M7 的程序只要看到 `程序 -c "from multiprocessing.spawn import spawn_main; ..."` 就当成子进程，用 python 的方式执行 `-c` 后面的代码。程序的参数常常来自不可信的输入（Web 服务把用户给的参数传给命令行工具），这等于任意代码执行，而 CPython 的 `python app.py -c ...` 只是把这些参数交给脚本。现在程序的参数永远不决定角色：分发里只有以 `python` 的名字运行才是解释器，能控制 `argv[0]` 的人本来就能直接运行 `python/bin/python`；开发构建要求环境变量里的标记，能控制环境变量的人本来就能用 `LD_PRELOAD` 等方式执行代码。标记的值是程序的路径，所以一个 steelix 程序启动另一个 steelix 程序时不会让对方误认。标记设置之后由这个进程启动的所有子进程都会继承，只对同一个路径的程序、并且命令行是子进程命令时才有意义。开发构建不是部署的目标，部署要用 `steelix build`
- 没有用 `sys.frozen`（PyInstaller 的做法，spawn 改用 `程序 --multiprocessing-fork` 的命令行），因为 forkserver 和 resource tracker 不管 `frozen` 都用 `-c`，而且 `sys.frozen` 会改变别的库的行为

**Spike 的结论**。在本机用 uv 的 python-build-standalone 3.14.9 测量，不下载别的东西。hello 是 `tests/dist_apps/hello`（用到 json、subprocess 和一个第三方包），json 是 `tests/projects/stdlib_json`。启动时间是中位数，分发目录只读。

| 方案 | hello | json | tar.gz | 文件数 | 启动（hello / json） |
| --- | --- | --- | --- | --- | --- |
| 精简的标准库加 `.pyc`（默认） | 58.9 MB | 59.5 MB | 19.2 MB | 1137 | 35 ms / 22 ms |
| 精简的标准库，没有 `.pyc` | 47.6 MB | 48.1 MB | 14.9 MB | 580 | 120 ms / 68 ms |
| 完整的标准库加 `.pyc` | 64.7 MB | 65.2 MB | 22.3 MB | 1378 | 39 ms / 21 ms |
| 标准库打包成 `python314.zip`，只有 `.pyc` | 41.7 MB | 42.2 MB | 17.1 MB | 11 | 46 ms / 28 ms |
| `python314.zip`，`.py` 加 `.pyc` | 44.5 MB | 45.0 MB | 19.8 MB | 11 | 51 ms / 30 ms |

CPython 直接运行 json 的驱动脚本是 27 ms（`python -c pass` 是 16 ms）。体积的大头是 libpython（33.6 MB，python-build-standalone 把 OpenSSL、SQLite 等扩展模块都静态链接在里面，`strip` 也只能减到 32.2 MB），程序本身不到 1 MB。

- **附带 python-build-standalone 加 site-packages**（现在的做法）：可以重定位，扩展模块照常工作，和开发时用的是同一个解释器。默认是去掉 GUI、IDLE、ensurepip 的标准库加 `.pyc`：`.pyc` 多占 11 MB，但只读目录里的启动时间从 120 ms 降到 35 ms
- **标准库 zip**：文件数从一千多个降到 11 个，体积少 17 MB，代价是启动多 6 到 11 ms，只有 `.pyc` 时标准库的 traceback 没有源码行。文件数只在拷贝和解压时有影响，暂时不做，以后可以作为选项
- **静态链接 libpython**：python-build-standalone 有静态的变体（本机的 install_only 没有 `libpython3.14.a`）。好处是少一个 `.so` 和 rpath，总体积差不多（libpython 的内容进了程序）。问题是 `lib-dynload` 和第三方的 C 扩展模块要从进程里解析 libpython 的符号，程序必须用 `--export-dynamic` 导出它们；musl 的完全静态版本不能 `dlopen`，根本不能加载扩展模块；PyO3 要用 `PYO3_CONFIG_FILE` 配置静态链接，还要链接 libpython 依赖的静态库。收益小，复杂度高，不做
- **用系统的 Python**：要求目标机器上正好有同一个次版本的 Python，Debian 系还把标准库拆成多个包，venv 的依赖也要另外安装，没有控制力，不做

**其它平台**（没有实现）。macOS 上 rpath 写成 `@executable_path/python/lib`，libpython 是 `libpython3.14.dylib`，定位可执行文件用 `_NSGetExecutablePath`（Rust 的 `current_exe` 已经处理），还要处理代码签名和公证（复制进来的 dylib 要重新签名）。Windows 没有 rpath，`python314.dll` 放在可执行文件旁边由加载器找到，标准库在 `Lib/`、扩展模块在 `DLLs/`，`sys.executable` 不能是符号链接（要复制一个 `python.exe` 或者用别的方式区分角色），multiprocessing 的 spawn 在 Windows 上用 `_winapi.CreateProcess`，命令行的识别方式一样。runtime 的 `embed` 现在只在 `cfg(unix)` 下编译。

## 12. 名字卫生与已知语义差异

名字卫生，由一个确定性的重命名表统一处理

- Rust 关键字用 `r#`（`type`、`match`、`loop`、`ref`、`move`、edition 2024 保留的 `gen` 等），PyO3 会去掉参数名的 `r#`
- `self`、`super`、`crate` 不能做 raw identifier，`self` 改名为 `slf`
- 生成代码自己使用的名字和 Python 名字冲突时，改名 Python 名字，加 `_` 后缀直到不冲突。函数体内保留 `py`、`_guard`、`frame`、`_handling`、`Ok`、`Err`、`Some`、`None`；模块级函数名保留 `exec_module`、`m`、`module`、`py`、`frame`，它们是生成的函数的局部变量（名为 `m` 的 Python 函数生成 `fn m_`）。不需要改名的名字优先保留原拼写，改名不会抢占它们
- 模块上下文参数叫 `m`，和 `#[pymodule]` 函数的模块参数一样。函数里有名为 `m` 的 Python 局部变量（常见的是正则的 match 对象）时，保留局部变量的名字，这个函数的上下文参数改叫 `module`
- 嵌套函数的 Rust fn 和外层函数的局部变量、临时变量在同一个命名空间，冲突时同样加 `_` 后缀
- 类的 Rust mod 名是类名的 snake case（`TextCalendar` 是 `text_calendar`），和模块里别的 mod、`rt`、`pyo3`、`std`、`body`、Rust 关键字冲突时加 `_`。mod 里的方法 fn 避开 `body`、模块级函数名（`use super::*` 会被遮蔽）和生成代码自己的名字，类体 fn 的局部变量 `ns`、`__class__` 同样保留
- Python 可见的函数名由 `rt::def!` 去掉 `r#` 得到，改过名的用 `name = "..."` 显式给出。改过名的参数在签名里写成 `py_ as "py"`，报错信息、关键字参数和 `inspect.signature` 都用 Python 名字
- 含孤立代理（lone surrogate）的字符串不能写成 Rust `&str` 字面量，用 `rt::str_surrogatepass(py, b"...")?` 从 `surrogatepass` 编码的字节构造
- 字面量的具体写法：i32 范围内的整数直接写，超出的加 `i64`/`i128` 后缀，更大的用 `rt::bigint(py, "...")?`；浮点数用 `repr` 的最短往返表示，`inf` 写 `f64::INFINITY`；`None` 作参数时写 `py.None()`；`bytes` 一律用 `rt::bytes(py, b"...")`，不依赖 PyO3 对字节数组的隐式转换；虚数字面量 `2j` 写成 `rt::complex(py, 0.0, 2.0)`（M10，PyO3 没有对应的 Rust 字面量，`-2j` 照常是 `.neg()`）
- docstring 按 3.13+ 编译器的规则去缩进（与 `inspect.cleandoc` 不同，保留首尾空行），同时生成 `///` 和 `__doc__`

已知且有意保留的语义差异

- 局部变量释放的时机（§5、§8）。函数返回时的释放顺序多数时候和 CPython 一样，但有几处不同（`__del__`、weakref 回调能看出来，M11a 的 review 逐一和 CPython 比较过）
  - 局部变量按第一次出现的顺序声明，再移到第一次使用之前，Rust 按声明的逆序 drop，CPython 按 `co_varnames` 的逆序释放。声明越过别的局部变量时两者不同，例如 `if` 的两个分支最后都给 `a` 赋值、`else` 分支先给 `b` 赋值时，`a` 和 `if` 链合并成 `let a = if ..`，声明排在 `b` 后面，函数返回时先释放 `a`，CPython 先释放 `b`（`drop_order` 用例的 `crossing`，`KNOWN_DIFFERENCES`）
  - `for` 的目标变量只在循环里用到时在每次迭代开头声明（§5），所以最后一个元素在循环结束时释放，CPython 要等到函数返回或者变量被重新赋值
  - 只在 `try` 体里用到的变量声明在 `try` 生成的闭包里（§9），`try` 语句结束时就释放。同样，只在 `with` 里用到的 `as` 目标是 `frame.with` 闭包的参数（§9），`with` 结束时释放，CPython 要等到函数返回
  - 被重新赋值的参数（`let mut n = n.clone();`）不会让实参提前释放，调用方一直持有实参到调用返回，CPython 的帧拥有实参，`f(T())` 里的 `T()` 在 `n` 被重新赋值时就释放
  - 经过函数对象的调用（`f.call1((a, b))?`，例如嵌套函数和不能直接调用的模块函数）的参数在调用返回后按书写顺序释放，先 `a` 后 `b`，因为 Rust 的元组按字段顺序 drop。CPython 先释放 `b`。直接调用的参数是语句里的临时变量，按相反的顺序释放，和 CPython 一样
  - `match`（§8 "match 语句"，`tests/cases/match_release.py`）的主体活到 `match` 结束，模式取出来但没有绑定名字的值（`if let Some([item, x])` 里的 `item`、类模式的属性）活到这个 case 结束，CPython 在执行体之前就释放了它们。只在一个 case 里用到的捕获名字在这个 case 结束时释放，和 `for` 的目标变量一样，CPython 要等到函数返回
  - 一条语句里的临时值活到语句结束（§8 的"粒度是语句"）：`print("made", D(tag).tag)` 里的 `D` 在 `print` 之后才释放，CPython 取完 `.tag` 就释放
  - 异常离开原生函数时，函数的局部变量随着 Rust 的返回立即释放。CPython 的 traceback 引用帧，帧持有局部变量，它们要等 traceback 释放（通常是调用方的 `except` 结束）才释放。原生帧没有 `f_locals`，traceback 里也看不到它们

- 直接调用的模块函数不受其他模块 monkeypatch 影响（§7。以后有需要可以加一个让全局调用都走动态查找的 `--dynamic-globals` 选项，现在没有）
- 原生函数的 traceback 帧是异常离开时合成的（第 9 节）。和 CPython 的区别：默认行号是 `def` 所在的行（`<module>` 是 1），显示的源码行也是这一行；`--traceback-lines` 时是语句所在的行。原生帧没有 `^^^` 定位（`tb_lasti` 是 -1），`f_locals` 为空。保留为 Python 的代码（§13）的帧和 CPython 完全一样
- 以下行号的细节只在 `--traceback-lines` 时有意义：多行语句只显示取值所在的一行；装饰器写在 `rt::def!` 里时，宏之前只有一个 `frame.line(N)`，所以只在一个装饰器、默认参数不会出错的时候才这样写，别的情况拆开写（§6），行号和 CPython 一样；`continue` 之前检查信号时的行号是上一条执行的语句；只有一个表达式的 `try`（第 9 节的 A）里查找 builtin 和全局名字提前到 `match` 之前，只有这个查找本身出错（`NameError`）时才看得出顺序不同
- 编译期的 `SyntaxWarning`（例如 3.14 PEP 765 对 `finally` 里 `return` 的警告）由 CPython 在编译时输出，二进制程序没有编译 Python 源码这一步，不输出
- 合成的帧只在异常经过时存在，运行期间解释器的帧链上没有原生函数，所以按调用方的帧工作的标准库功能看到的是离它最近的 Python 帧（`tests/cases/frames_logging.py` 记录了现在的输出）
  - `logging` 的 `%(filename)s`、`%(lineno)d`、`%(funcName)s` 指向 logging 自己的帧，例如 `__init__.py:2191 warning` 而不是 `app.py:8 step`
  - `warnings.warn` 的位置是调用这个原生函数的保留代码所在的行；整条调用链都是原生代码时是 `<sys>:0`
    - 所以默认的 filter 对它们和 CPython 不同：`DeprecationWarning` 默认只在 `__main__` 里显示，`<sys>:0` 的不显示；同一条文字的警告在同一个位置只显示一次，所有 `<sys>:0` 的都算同一个位置，CPython 里不同行的会各显示一次。原生协程和生成器的 `throw(type, value, tb)` 的 `DeprecationWarning` 和多个 `coroutine '..' was never awaited` 就是这样（`coroutine_warnings` 用例）
  - 从调用方帧推断模块名的 API：三个参数的 `type(name, bases, dict)` 创建的类没有 `__module__`，`enum.Enum("C", "A B")` 这样的函数式 API 得到 `<unknown>`。`collections.namedtuple` 找不到帧时用 `__main__`，bin 模式的主模块碰巧是对的
  - 要修好需要在原生函数运行期间压入真正的解释器帧，CPython 没有公开 API（第 9 节）
- 原生的 `while` 循环每次迭代结尾（以及 `continue` 之前）调用 `py.check_signals()?`（`PyErr_CheckSignals`，M3d 加入，M4 从迭代开头移到结尾，traceback 的行号因此和 CPython 一样是最后执行的语句）。`for` 循环和推导式（M14 之后）用 `rt::iter(&xs)?` 代替 `xs.try_iter()?`，它在取第二个及以后的元素之前检查，循环体里不用再写这一行。`--traceback-lines` 时循环体结尾要把行号设回 `for` 那一行，`rt::iter` 的检查在那之后，报的会是 `for` 行，而 CPython 报循环体最后执行的语句（检查在回跳的 `JUMP_BACKWARD` 上），所以这时仍在设回行号之前写 `py.check_signals()?;`（`interrupt_for` 用例）。对应 CPython 在循环回跳处检查信号，所以 Ctrl-C 能打断只调用 C API 的循环，抛出 `KeyboardInterrupt`。它同时会运行已经排上的循环垃圾回收，和解释器的检查点一样。代价实测每次迭代 3 到 5 ns（release，`for i in range(n): total += i` 从 36 ns 到 42 ns，空循环从 6.3 ns 到 9.7 ns），只在几乎什么都不做的循环里明显。bin 模式下未捕获的 `KeyboardInterrupt` 打印 traceback，finalize 之后和 CPython 一样用 `SIGINT` 结束进程（shell 看到的是被中断，`subprocess` 看到的退出码是 `-2`）
- 原生循环里 GIL 不会像解释器那样定期切换，其他 Python 线程要等到原生代码调用会释放 GIL 的 API 时才能运行
- 直接调用的递归受 C 栈大小限制而不是 `sys.getrecursionlimit()`，`RecursionError` 的信息是 `Stack overflow (used N kB) ...` 而不是 `maximum recursion depth exceeded`
- 模块级代码里，`def` 执行之前的调用走 globals 查找（会得到 `NameError`）；但函数体里的直接调用不检查被调函数的 `def` 是否已经执行过
- `builtins.print` 等在参数求值之后才查找（Python 是先查找名字再求值参数），只有参数求值过程中替换了这个 builtin 才看得出区别
- 特化的 builtin（`len`、`isinstance`、`str`、`repr`、`bool`、`abs`、`type`、`iter`）直接调用对应的 C API，不在运行时查找 `builtins` 模块，所以 `builtins.len = ..` 这样的 monkeypatch 对它们无效。模块级重新绑定这些名字（`len = ..`、`def len`）则会被识别，不再特化
- 字面量每次求值都创建新对象，而 Python 的常量在同一段代码里共享。只有比较 `str`、大整数常量的 identity 时才看得出区别，例如 `d["k"] += 1` 里 `__getitem__` 和 `__setitem__` 收到的是两个相等但不同的 `"k"`
- f-string 中带转换但没有 spec 的字段（`{x!r}`）直接使用 `repr()`/`str()`/`ascii()` 的结果，CPython 对非精确 `str` 的结果还会再调用一次 `format`。只有 `__repr__`/`__str__` 返回带自定义 `__format__` 的 `str` 子类时才有区别
- `from x import y` 失败时 `ImportError` 的文字和 CPython 一样（M6 起包括循环 import 的"partially initialized module"和模块名遮蔽标准库的提示，§7）。模块级 import 传给 `__import__` 的 `locals` 是 `None`（CPython 传模块的 dict）
- warning（例如 `DeprecationWarning`）的输出里没有源码行
- 对自身模块的修改（第 7 节）只识别常见写法。识别不到的有：`importlib.import_module(__name__)`、`__import__(__name__)`、`getattr(sys, "modules")` 等其他取得模块对象的方式，经过参数、容器、属性传递的模块对象，单个模块的 ext 模式下按模块的真实名字 `import` 自己，`exec`/`eval` 的命名空间是别处得到的模块 dict，项目以外的模块里的修改（没有一起转换的测试里的 `mock.patch("app.f")`），以及项目内用别的写法做的修改（模块名是算出来的字符串等）。这些情况下直接调用看不到替换
- 函数对象是 `Function` 而不是 `types.FunctionType`：`isinstance(f, types.FunctionType)` 和 `inspect.isfunction(f)` 为 False，`__code__` 只描述签名，没有字节码，`co_firstlineno` 是 1（`dis`、`inspect.getsource` 等依赖字节码或源位置的工具无法使用）。`__closure__` 存在，内容是真实的 cell。`inspect.signature`、`functools.wraps`、pickle 能正常使用（第 6 节）。函数类型本身的 `type(f).__module__` 是实例属性 `__module__` 的 getset descriptor 而不是字符串（PyO3 的类是 heap type，类型的 `__module__` 从类型的 `__dict__` 里读，被实例的 getter 占了），`type(f).__name__` 是 `function`（M10 检查 skime 的函数是否原生时发现）
- `del f.__defaults__` 等删除元信息属性的操作抛 `AttributeError`，CPython 是把它设为 None。赋值为 None 的行为一致
- 函数类型是每个扩展模块自己的：一个进程里 import 了两个 steelix 生成的扩展模块（ext 模式）时，各自带一份 runtime，`type(a.f) is type(b.f)` 为 False，`isinstance(a.f, type(b.f))` 也是 False。生成器表达式的类型、`NativeLoader` 等 runtime 的类同样如此。bin 模式和一个项目的所有模块共用一个 runtime，没有这个问题
- 注解由源码文本 `eval` 得到。STRING 格式对非字符串字面量的注解直接返回源码文本，CPython 由 AST 重新生成，空白等写法可能不同；函数注解在 FORWARDREF 格式下创建的 `ForwardRef` 没有 `owner`（类注解的有）。`__annotate__` 对 FORWARDREF、STRING 格式直接返回结果，CPython 编译器生成的 `__annotate__` 只支持 VALUE，其余格式由 `annotationlib` 用假 globals 实现，通过 `annotationlib.get_annotations` 使用时没有区别
- 直接调用时省略的参数，不可变字面量的默认值直接写在调用处，所以别的模块给这个函数的 `__defaults__`/`__kwdefaults__` 赋值时直接调用看不到，这和直接调用看不到 monkeypatch 是同一类差异。本模块里的赋值会被识别，这时改为读函数对象（第 6 节）。字面量每次调用都创建新对象，见上面关于常量 identity 的一条
- 其余省略的参数从 globals 里当前的函数对象读取。如果别的模块把这个名字换成了别的对象（monkeypatch，或者 `sys.modules[__name__].f = g`），直接调用照常执行原来的 Rust 函数，读默认值时抛 `TypeError: f() was replaced by <function g at 0x..>, but transpiled code calls the original directly and can not take the default of its parameter 'b' from the replacement`。CPython 会调用替换后的函数，要做到一样，直接调用就得在每次调用前检查 globals，失去了直接调用的意义，所以只保证报错清楚。M3c 之前这里会 panic默认值被删掉（`__defaults__ = None`）后直接调用缺参数时，`TypeError` 只列出第一个缺少的参数，其余参数也已经求值
- 模块级的重复定义（同名的第二个及以后的顶层 `def`）保留为 Python（§13）。模块级复合语句里的 `def`（`if` 里的条件定义、循环和 `try` 里的定义，M8）是 `exec_module` 里的 fn item，执行到 `def` 时绑定到 globals，调用一律经过 globals
- 生成器表达式的对象（§8）不是 `types.GeneratorType`，而是 runtime 的 `generator` 类：`collections.abc.Generator`、`Iterator` 的 `isinstance` 为 True，`inspect.isgenerator(g)` 和 `isinstance(g, types.GeneratorType)` 为 False。`gi_frame`、`gi_code`、`gi_yieldfrom` 是 `None`，所以 `inspect.getgeneratorstate` 对还没开始的生成器返回 `GEN_CLOSED`（CPython 是 `GEN_CREATED`），开始之后的状态一致。`throw()` 抛出的异常的 traceback 里没有 `<genexpr>` 那一帧
- 原生的协程和生成器函数（§8 "协程和生成器"，M13，`tests/cases/coroutines.py` 检查前两条）。M12 之前生成器函数和 `async def` 保留为 Python，是 CPython 自己的对象，现在转换后的程序里它们都是原生的，下面几条因此适用于所有转换过的生成器和协程
  - 协程是 runtime 的 `coroutine` 类型，不是 `types.CoroutineType`：`asyncio.iscoroutine`、`isinstance(c, collections.abc.Coroutine)` 为 True，`inspect.iscoroutine(c)` 为 False，生成器同理（`inspect.isgenerator` 为 False）。`cr_frame`、`cr_code`、`cr_await`、`cr_origin`（生成器的 `gi_frame`、`gi_code`、`gi_yieldfrom`）都是 `None`，所以 `inspect.getcoroutinestate` 对没开始的协程返回 `CORO_CLOSED`，`sys.set_coroutine_origin_tracking_depth` 不起作用，按 `cr_await` 链打印任务栈的工具（`asyncio` 的 `Task.print_stack`、`python -m asyncio ps`）看不到原生协程里面。Cython 的协程有同样的限制
  - GC 能看到暂停的协程和生成器的模块 globals、捕获的变量、参数、正在处理的异常、`await`/`yield from` 委托的对象和 traceback 帧对象（§8），经过它们的循环和 CPython 一样被回收并 `close()`。看不到的是函数体自己的代码跨过暂停持有的东西：局部变量、语句的临时值、`except` 块绑定的异常（`except E as e` 的 `e`，以及 `rt::Caught` 保存的之前的异常）、`with` 块的上下文管理器和 `__enter__` 的值、`finally` 里待重新抛出的异常。被它们引用的对象（以及从这些对象能到达的一切）不会被回收，经过它们的循环也就不会被回收，`finally` 也不会执行，直到循环被别的方式打断。例如暂停在 `except KeyError as e:` 块里的协程，`e` 的 traceback 引用帧对象，帧对象引用模块的 globals，于是模块的 globals 和挂在上面的这个协程在退出时都不会被回收；CPython 的帧里的局部变量对 GC 可见，会回收。lowering 先绑定被 await 的表达式（§8），所以被等待的对象的临时值不会跨过暂停。同一条语句里先于 `await` 求值的操作数（`f(a) + await g()` 里 `f(a)` 的结果）和直接 await 的实参（`await f(g())` 里 `g()` 的结果，CPython 里它们分别在值栈上和被调用的协程的帧里，GC 都看得到）跨过暂停，和局部变量一样看不见
  - 正在处理的异常（§8）：函数体自己的异常在暂停前后保存和恢复，`except` 里 `sys.exception()` 跨过 `await` 不变，也不会泄漏给调用方，`throw` 进来的异常以它为 `__context__`（没有时不设置，不用调用方的），这些和 CPython 一样。不同的是看到调用方的异常的地方（`coroutines`、`generator_close` 用例的 `KNOWN_DIFFERENCES`）
    - 跨过暂停的 `except` 块结束时恢复它开始时正在处理的异常，那是开始那一次 poll 的调用方的（或者没有），CPython 是这一次 poll 的调用方的。例如在调用方的 `except ValueError` 里恢复一个暂停在 `except KeyError` 块里的协程，块结束后 `sys.exception()` 是 `None` 而不是那个 `ValueError`，此后抛出的异常也就没有它作为 `__context__`
    - 直接委托（`rt::delegate`）的内层和外层共用一份异常状态，内层自己没有正在处理的异常时看到外层的（`sys.exception()`、新异常的 `__context__`，包括 `close()` 送进内层的 `GeneratorExit`），CPython 的内层看到的是调用方的。`rt::await_` 往委托对象里 `throw` 时，委托对象看到的也是外层的
  - 原生的 `Coroutine.__await__()` 在协程暂停时就抛 `RuntimeError: coroutine is being awaited already`，这样 `await c` 和 CPython 一样；CPython 是在 `await` 里检查，直接调用暂停的 CPython 协程的 `c.__await__()` 不会报错
  - 递归深度和同步函数的直接递归（本节前面的一条）一样只受 C 栈限制（§8），`yield from`、`await` 的链比 `sys.getrecursionlimit()` 深也能运行，太深时 `RecursionError` 的信息是 `Stack overflow (used N kB) ...`（`coroutines` 用例只比较异常类型）
  - 改写后的代码里 await 的 Rust future（例如 tokio 的）见 §8
  - `cr_frame` 是 `None`，所以 `repr(asyncio.Task)` 里没有 `running at 文件:行`，`Task.get_stack()` 是空的（`coroutine_warnings` 用例）
  - 直接委托（§8 的 `rt::delegate`）的内层在 `close()` 时两次都忽略 `GeneratorExit`（第一次成为外层的 `RuntimeError`，第二次是释放时的终结器）时，`sys.unraisablehook` 的消息是 `Exception ignored while closing generator`，后面没有对象的 repr，因为内层没有生成器对象（`generator_close` 用例）
  - `throw()` 的参数在最外层就检查并变成异常对象。参数不合法（`g.throw(42)`）时直接抛给调用方，CPython 在有委托对象时先交给委托对象：不是生成器的委托对象收到原来的参数，是生成器的在委托处（`yield from`）抛出 `TypeError`。同样因为只有这一个异常对象，经过保留为 Python 的生成器再委托给不是生成器的对象时，后者的 `throw` 收到的是实例而不是原来的参数
- 模块级只有一个参数的 `exec(src)`、`eval(src)` 在原生代码里显式传入模块的 globals（`exec(src, globals())`），和 CPython 在模块级的行为一样。函数里的要读写局部变量，所在的函数保留为 Python
- 模块级不带参数的 `globals()`、`locals()`、`vars()` 都是模块的 globals 字典，`dir()` 是排好序的名字列表（`m.dir()?`），和 CPython 一样。函数里不带参数的 `locals()`、`vars()`、`dir()` 要读局部变量，所在的函数保留为 Python
- 项目（§7，M6）的差异
  - 项目模块的 `__loader__` 是 `steelix_runtime.NativeLoader`，不是 `SourceFileLoader`：有 `name`、`path`、`get_source`、`get_filename`、`is_package`、`get_data`、`get_resource_reader`（M7a，`importlib.resources` 和 `pkgutil.get_data` 能用）和 `get_code`（M7a），没有 `source_to_code`、`path_stats` 等。`get_code` 返回的不是模块的字节码，而是一行调用原生代码的代码，`dis` 之类看字节码的工具看不到模块的内容
  - 经过 `get_code` 运行的模块（`python/bin/python -m pkg.cli`、`runpy.run_module`）出错时，traceback 里在 `<frozen runpy>` 的帧和原生的 `<module>` 帧之间多一帧这行代码（同一个文件的第 1 行）
  - 附带的解释器只在项目目录在 `sys.path` 里（或者包的目录在父包的 `__path__` 里）时用转换过的模块，和 CPython 找源文件的条件一样，但只按路径比较，不看目录里实际有没有源文件，目录本身不存在也可以（`--no-sources` 的分发里转换过的包可能没有目录）。路径先变成绝对路径，存在的部分解析符号链接，其余部分按字面规范化（`.`、`..`）。`sys.path` 里项目目录前面有同名模块的目录时，转换过的模块仍然优先
  - finder 放在 `PathFinder` 前面，项目模块总是优先。CPython 里如果程序自己在 `sys.path` 的 `sys.path[0]` 前面插入了含有同名模块的目录，会 import 到那个目录里的模块
  - import 失败的 traceback：经过 `builtins.__import__`（import 语句）时 importlib 的帧和 CPython 一样被去掉；经过 `importlib.import_module` 时 CPython 保留 importlib 的帧，转换过的模块少一帧 `<frozen importlib._bootstrap_external>` 的 `exec_module`（`NativeLoader.exec_module` 是原生方法，没有帧）
  - `-m` 运行时 traceback 开头没有 `<frozen runpy>` 的两帧
- 运行和分发（§11，M7）的差异
  - `sys.argv[0]` 总是脚本的绝对路径（和 `__file__` 相同），相当于 `python /绝对路径/app.py`。用相对路径运行 `python app.py` 时 CPython 的 `sys.argv[0]` 是 `app.py`，只有直接打印 `sys.argv[0]` 时看得出来。入口脚本是符号链接时，`__file__` 是链接指向的文件，CPython 是链接本身的路径
  - 分发之后项目目录是 `dist/app/src/`，不再是原来的目录名。代码里用 `os.path.dirname(__file__)` 找数据文件不受影响，但依赖目录名本身（比如打印它）的输出会变
  - `sys.executable` 是 Python 解释器而不是程序：开发构建是构建用的 venv 的 python，分发里是指向程序的 `python/bin/python`。multiprocessing 的子进程由 `sys.meta_path` 最前面的 `steelix_runtime.SpawnFinder` 改成启动程序本身，用户自己调用 `multiprocessing.set_executable` 会覆盖它。子进程里的 `__mp_main__` 就是执行生成代码的那个模块对象，CPython 是把 `runpy` 的结果复制到一个新的模块对象里
  - 名为 `python`、`python3`、`python3.N` 的程序会被当成 Python 解释器运行
  - 分发的程序忽略 `PYTHON*` 环境变量和用户的 site-packages（§11，设置 `STEELIX_PYTHON_ENV=1` 恢复），相当于 `python -E -s app.py`，`sys.flags.ignore_environment`、`sys.flags.no_user_site` 是 1。附带的解释器读取环境变量，但不加用户的 site-packages
  - `PYTHONSAFEPATH`（`-P`）：CPython 运行 `python -P app.py` 时 `sys.path` 里没有脚本所在的目录，项目里的模块只有在别的路径上才能 import。程序总是把项目目录放在 `sys.path[0]`，转换过的模块也总是由 finder 提供，所以开发构建设置了 `PYTHONSAFEPATH` 时仍然能 import 项目模块，`sys.flags.safe_path` 是 1 但 `sys.path[0]` 仍然是项目目录。分发里默认忽略这个变量
  - 开发构建的 multiprocessing 子进程靠环境变量 `STEELIX_MP_CHILD` 识别（§11），父进程 import `multiprocessing.spawn` 之后启动的所有进程都会继承这个变量
  - `--no-sources` 的分发里，traceback 没有源码行，`NativeLoader.get_source` 返回 `None`
  - 分发带上项目里包目录下的所有文件（§11），项目顶层的数据文件（不在包里的）要用 `--add-data` 加进去
  - 分发里的 `.pyc` 是 unchecked-hash 的，修改了分发里标准库或 site-packages 的 `.py` 之后要删掉对应的 `__pycache__` 才会生效
- 混合模式（§13）的差异：跟在 `;` 后面的保留语句，第一行在 traceback 里的列位置会偏；ext 模式下文件名用模块的 `__file__`，也就是扩展模块的路径，traceback 显示不出源码行
- 模块级注解（§13 的"模块注解"）：`__annotate__` 的结果和 CPython 一样，剩下的差异是
  - 一段保留代码执行期间，`__annotate__` 和 `__conditional_annotations__` 是这一段自己的（只有这一段的注解），执行完才换回整个模块的。只有同一段代码里读注解（或者它调用的函数读模块的 `__annotations__`）时才看得出来
  - `__annotate__` 里注解的列位置：每条注解从第 0 列开始，原来缩进的（`if` 里的）或者跟在 `;` 后面的注解求值出错时，traceback 里 `^` 的位置会偏。`__annotate__` 函数对象每段保留代码执行时会重新绑定一次，比较它的 identity 时和 CPython 不同
  - 有 `from __future__ import annotations` 时，CPython 在模块开头创建的 `__conditional_annotations__` 要等第一段带注解的保留代码执行时才有（这种模式下它不被使用）
- Rust 的 panic（手工改写的代码里的 bug，或者 steelix 的 bug）在 bin 模式下和未捕获的 Python 异常一样处理：打印 `pyo3_runtime.PanicException` 的 traceback，退出码是 1，而不是 Rust 默认的 101。生成的代码和 runtime 不应该因为 Python 一侧可见的状态 panic，这类错误都转成 Python 异常
- 嵌套函数的调用总是经过函数对象（性能差异，语义相同）
- 类（第 10 节）的差异
  - `dataclass(slots=True)` 重新建类时，只更新 Python 函数和 property 的 `__class__` cell（它用 `isinstance(f, types.FunctionType)` 挑选方法）。类里有用到 `super()` 或 `__class__` 的 property 时，同一个 cell 被更新，所有方法都正确；没有的话，原生方法里的零参数 `super()` 仍然指向原来的类，抛出 `TypeError: super(type, obj): obj (instance of P) is not an instance or subtype of type (P).`（`tests/cases/classes_dataclass_slots.py`）。别的重新建类的装饰器也是一样
  - 原生的 class 语句不经过 `builtins.__build_class__`，替换它（`builtins.__build_class__ = ..`）只对保留为 Python 的类起作用，和特化的 builtin 一样
  - `__classdictcell__`：CPython 在类体里有 `def` 或注解时总是创建这个 cell，steelix 在第一个 `def` 执行时才创建。只有类体里的 `def` 都没有执行（在不成立的 `if` 里）时，类的 `__dict__` 里少一个 `__classdictcell__`
  - 类体里有保留为 Python 的语句（§13）时，运行时在每段保留代码执行前后读写命名空间的 `__module__`、`__qualname__`、`__firstlineno__`、`__static_attributes__` 几个键，把保留代码自己设置的值改回来。`__prepare__` 返回自定义 mapping 时能看到这些多出来的操作
  - 多行的 class 语句（多个装饰器、多行的 bases）在建类时出错，模块帧只显示 `class` 所在的一行，和上面"多行语句只显示取值所在的一行"是同一个差异

## 13. 不支持特性的处理：混合模式（M3c）

目标是 CPython 能运行的应用都能转换，并且行为一致。steelix 还不能降级的部分保留为 Python 源码，在模块的 globals 里执行。整个程序先跑起来，再逐步扩大原生转换的范围，覆盖率报告（下面）告诉我们下一步该支持什么。

- 默认就是混合模式。`steelix transpile --no-fallback` 恢复以前的行为：遇到不支持的语法就报错，诊断信息带 `文件:行:列` 和特性名
- 粒度是模块作用域里的语句。模块的作用域就是 globals 字典，所以不支持的模块级语句（带注解的赋值、`import *`、容器字面量里的 `*` 展开等）可以就地从源码执行，求值顺序和 Python 一致。包含不支持语法的顶层 `def`（异步生成器、多个 `for` 的生成器表达式、函数里的类、带捕获的 OR 模式等，协程和生成器里保留的写法见 §8 "协程和生成器"，`match` 的见 §8 "match 语句"）整个保留为 Python，嵌套函数一起保留
- 模块级的 `if`、`for`、`while`（包括 `if __name__ == "__main__":`）和 `match`（M14）会拆开（M3d）：只有体里的某些语句不支持时，只保留这些语句，`if`/循环/`match` 本身和其余语句仍是原生代码。拆分是递归的，体里的复合语句同样处理。不拆的情况：不支持的构造在条件、循环变量、被迭代的表达式或者 `case` 的模式和 guard 里，整个语句保留；保留的语句里有跳出原生循环的 `break`/`continue`（单独编译是 `SyntaxError`），整个循环保留，这是最简单而且正确的做法，保留的循环自己的 `break`/`continue` 不受影响。保留的语句之间不用同步任何东西，它们读写的都是 globals
- 类按方法拆开（M5，方法级的混合模式）：类本身原生创建，只有含不支持语法的方法和类体语句保留为 Python，在类的命名空间里执行，见下面的"类体里保留的语句"。类体里的 `if`/`for`/`while` 同样拆开
- 原生函数里的一部分语句保留为 Python（更细的粒度）暂不做：那需要在 Rust 局部变量和 Python 代码之间同步局部变量。记录保留哪些语句的数据结构（`frontend/fallback.py` 的 `FallbackPlan`）按语句节点索引，现在已经放进模块级复合语句里的语句，以后可以直接放进函数里的语句

### 判定

- `frontend/subset.py` 遍历整个模块，记录每个不支持的构造（特性名、位置、所在的最外层函数，即模块级 `def` 或类的方法），按顶层语句分组。遇到不支持的节点不停下，继续检查它的子节点，所以一个函数里所有的阻碍都会被统计。类的方法和模块级 `def` 一样统计
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
        rt::def!(m, double(x))?;
        // mixed_mode.py:35, kept as Python: yield at 38:13
        rt::exec_python!(
            m,
            line = 35,
            r#"
def evens(limit):
    for n in range(limit):
        if n % 2 == 0:
            yield n
"#
        )?;
        rt::def!(m, use_counter(start))?;
```

拆开的 `if` 里保留的语句保持原来的缩进（节选自 `tests/golden/mixed_nested.rs`）：

```rust
        if m.global("sys")?.attr("argv")?.len()? > 5 {
            m.set_global("mode", "many")?;
            // mixed_nested.py:12, kept as Python: unpacking in a display at 12:15
            rt::exec_python!(
                m,
                line = 12,
                r#"
    labels = [*"many"]  # kept as Python, the `if` is native
"#
            )?;
```

- raw string 从 `r#"` 的下一行开始，`line = 17` 是第一行在 Python 文件里的行号。源码里出现 `"#` 时自动加更多的 `#`
- 语句之间、保留的语句前面的注释仍然生成 Rust 注释；保留的语句内部的注释留在源码里。复合语句和带装饰器的语句总是从行首开始；跟在 `;` 后面的简单语句只截取它自己
- 复合语句体里的语句保留原来的缩进，运行时在它前面一行补一个 `if True:` 再编译，所以列位置（traceback 里的 `^`）也和文件一致
- 模块有 `from __future__ import annotations` 时写成 `rt::exec_python!(m, line = 17, future_annotations, r#"..."#)`，编译时带上对应的 flag
- 生成的模块文件开头有一段模块文档，列出所有保留为 Python 的语句和原因，读代码的人一眼就能看出哪些已经是原生代码
- 检查器（§2）认识这个宏：参数必须是模块上下文（类体里是命名空间 `ns`）、`line = N`、可选的 `future_annotations` 或 `first_annotation = I`（见下面的模块注解）和 raw string，源码必须从新的一行开始，并且按这个行号（缩进的源码按上面的 `if True:`）能用 `compile()` 编译

运行时 `rt::python::exec_python` 在源码前面补 `line - 1` 个空行，用 `Py_CompileStringObject` 以模块的 `__file__` 为文件名编译（项目里的每个模块都由 finder 设置成原来的源文件路径，所以别的模块里保留的代码和原生函数合成的帧显示的也是各自的文件，M6），再用 `PyEval_EvalCode` 在模块的 globals 里执行（globals 和 locals 是同一个 dict，和模块级代码一样）。所以

- 保留的 `def` 和 `class` 定义的对象和 CPython 完全一样：`__module__` 来自 globals 的 `__name__`，`__qualname__`、`__code__.co_filename`、`co_firstlineno` 都是原来的，`inspect.getsource` 能读到源码
- 异常经过保留的代码时，traceback 里是原来的文件名和行号，并且能显示源码行
- 原生代码和 Python 代码共享同一个 globals：Python 代码调用原生函数（`rt::def!` 绑定在 globals 里），原生代码通过 `m.global(..)` 调用 Python 定义的函数和类，两个方向都可以

### 类体里保留的语句（M5）

类体里不能降级的语句（方法、类属性的赋值、类体里的 `if` 等）保留为 Python，生成的 `body` 在原来的位置用类的命名空间执行它们，前后的语句仍是原生代码。下面节选自 `tests/golden/classes_mixed.rs`：

```rust
            ns.set("native_before", 1)?;
            // classes_mixed.py:16, kept as Python: list comprehension at 16:15
            rt::exec_python!(
                ns,
                line = 16,
                r#"
    squares = [i * i for i in range(4)]
"#
            )?;
            ns.set("native_after", ns.get("squares")?.get_item(-1)?)?;
            // classes_mixed.py:22, kept as Python: list comprehension at 23:18
            rt::exec_python!(
                ns,
                line = 22,
                r#"
    def kept_greet(self):
        values = [x for x in super().items()]
        return f"kept:{super().greet()}:{values}:{__class__.__name__}"
"#
            )?;
```

运行时（`class.rs` 的 `exec_python`）把保留的源码当作类体编译，在它前面的几行补上外层类的 `class` 语句（嵌套类每层一行，名字取自 `__qualname__`），编译整段源码，取出最里层类体的代码对象，以模块的 globals 和类的命名空间为 locals 执行。所以

- 保留的方法和 CPython 编译出来的完全一样：`__qualname__`、`co_firstlineno`、traceback 里的文件名和行号都是原来的，私有名字按真实的类名改编
- 保留的方法里的 `super()` 和 `__class__` 用的是这段代码自己的 `__class__` cell。执行完运行时从命名空间里取走它放进去的 `__classcell__`、`__classdictcell__`，建类之后把类填进这些 cell，和原生方法的 cell 一样经过 metaclass 检查
- 这段代码自己会设置 `__module__`、`__qualname__`、`__firstlineno__` 和 `__static_attributes__`，执行完改回类的值（`__static_attributes__` 在建类前由运行时统一设置），差异见第 12 节
- 原生代码和保留代码共用同一个命名空间，互相读写类属性，调用彼此的方法

整个类保留为 Python 的情况：不支持的构造在类的头部（装饰器、bases、关键字）或者是类型参数（`class C[T]:`）；类体里有 `global`/`nonlocal` 声明，或者复合语句里有带注解的赋值，它们会改变整个类体的编译方式；要保留的语句里有带注解的赋值（注解属于整个类的 `__annotate__`，不能分段编译）。标准库里只有 `typing` 的 3 个带类型参数的类整个保留。

### 模块注解

CPython 3.14 把模块里所有的注解编译成一个 `__annotate__` 函数，在模块的第一条语句之前绑定，并用 `__conditional_annotations__` 集合记录哪些带注解的赋值已经执行过（按注解在模块里的序号），`__annotate__` 只返回执行过的。模块级的带注解赋值现在都保留为 Python，分在几段单独编译的代码里，每段都会绑定只有自己注解的 `__annotate__`，序号也从 0 开始。所以（M3d）

- `exec_module` 开头调用 `rt::module_annotate(m, &[(8, "A: int"), (16, "B: list[str]")])?`，把所有注解按原来的行号编译成模块的 `__annotate__`，和 CPython 一次编译整个模块得到的一样，`__conditional_annotations__` 是空集合
- 含有模块级注解的保留代码写成 `rt::exec_python!(m, line = 16, first_annotation = 1, r#"..."#)`，`first_annotation` 是这一段第一个注解在模块里的序号。执行完把这一段记下的序号加上它，并入模块的集合，再换回模块的 `__annotate__` 和集合
- 有 `from __future__ import annotations` 时没有 `__annotate__`，每条带注解的赋值往模块的 `__annotations__` 字典里加一项，几段代码共用同一个字典，只需要在开头创建它（CPython 也是在开头创建）
- 只有名字的注解（`x: int`）算数，`(x): int`、`a.b: int` 不进 `__annotate__`。剩下的差异见第 12 节

### 手工把保留的函数改成原生代码

保留为 Python 的 `def` 和原生函数的接口相同：都是 globals 里的一个名字，调用方都经过它。所以在 steelix 支持对应的语法之前，也可以手工把它改写成 Rust，步骤是

1. 在模块文件里写一个 `pub fn`，签名用适配层的约定：`fn name<'py>(m: &rt::Module<'py>, x: &Bound<'py, PyAny>, ..) -> PyResult<Bound<'py, PyAny>>`，`*args` 是 `&Bound<'py, PyTuple>`，`**kwargs` 是 `&Bound<'py, PyDict>`（照着同一个文件里生成的原生函数写）。函数体可以先照抄 steelix 对类似代码的生成方式，也可以直接按第 15 节写原生逻辑
2. 在 `exec_module` 里原来的位置，把 `rt::exec_python!(..)?;` 换成 `rt::def!(m, name(x, y = ..))?;`，签名写 Python 的参数列表，有默认值的参数用 `rt::def!` 的写法（同样照着生成的例子）。装饰器写在名字前面，`rt::def!(m, @functools.cache name(x))?`（§6，见 `tests/golden/decorators.rs`）
3. 删掉 raw string 里的 Python 源码和文件开头模块文档里对应的一行，跑端到端测试

注意两点：这样改写的函数仍然只能通过 globals 调用，其他原生函数里对它的调用仍是 `m.global("name")?.call1(..)`，要直接调用就把这些调用点也改成 `name(m, ..)?`，和第 15 节"调用方"一样；改写过的模块文件从此由人维护，不能再用 steelix 覆盖。

### 覆盖率报告

`steelix transpile` 结束时在 stderr 打印一段摘要，`-v` 列出每个不支持构造的位置，`-q` 不打印，`--report FILE` 写出 JSON（`coverage.py`），可以跨模块合并（`coverage.merge`），用来决定先支持什么

```
stdlib_calendar.py: native 50/72 (69%) functions, 42/42 (100%) module statements, 583/838 (70%) lines
  kept as Python by     functions  only  statements  occurrences
  list comprehension            8     8           2           13
  generator expression          8     8           0           12
```

- 项目（M6）按模块各打印一段，文件名是相对于项目目录的路径（`store/items.py`），最后是所有模块的合计和合并后的特性排名（`all 6 modules: ...`），JSON 是 `{"modules": [..], "total": {..}, "features": [..]}`。另外列出转换了哪些模块、哪些项目文件没有转换以及原因
- 统计单位：函数是模块级 `def` 和类的方法（嵌套函数算在外层函数里），语句是除 `def` 以外的顶层语句，另外按行数统计
- 每个特性的列：`functions` 是它出现在其中的函数个数（类出现时算类的所有方法），`only` 是只被它挡住的函数个数（支持它就能让这些函数变成原生代码），`statements` 是函数以外的顶层语句个数，`occurrences` 是出现次数
- JSON 里还列出每个保留的函数和语句以及挡住它的特性。真实应用的数据和据此提出的优先级见 `coverage.md`

## 14. 测试与构建

- **单元测试**：rustast 打印、作用域与绑定分析等纯 Python 逻辑
- **Golden 快照**：`tests/cases/*.py` 转换结果与 `tests/golden/*.rs` 对比，`--update-golden` 更新。rustfmt 是必须的，固定 style edition，避免快照抖动。golden 是默认的输出（没有行号 marker），`tests/golden/traceback_lines/` 另有 8 个用例（`fib`、`control_flow`、`try_except`、`classes_basic`、`async_with_for`、`generators`、`match_basic`、`match_async`，列表在 `tests/test_golden.py` 的 `TRACEBACK_LINES_CASES`）在 `--traceback-lines` 下的输出
- **`cargo check` 层**：只检查 golden 能编译，比端到端快。同时要求生成代码没有编译警告（crate 根上只放 `non_snake_case`、`unused_variables`、`unused_assignments` 三个 allow。最后一个是因为 Python 代码里常见赋值后不再读取的写法，`del x` 生成的 `x = None;` 也是这种情况），把可读性问题挡在快照阶段。同一层还对所有 golden 跑 `cargo clippy`（默认 lint 组，警告也算失败）。crate 根上另外 allow 几个 clippy lint：`never_loop`（Python 里 `for` 的第一轮就 `return`/`break` 是常见写法）、`approx_constant`（Python 源码里的 `3.14159` 本来就是那个字面量）、`if_same_then_else`（Python 两个分支写同样的代码是程序本身的事）、`too_many_arguments`（Python 函数的参数个数，M6 转换 `json` 时遇到）、`doc_lazy_continuation`（docstring 不是 Markdown，doctest 的 `>>>` 会被当成引用块），以及 M9 的 bool 局部变量（§5）带来的四个：`while_immutable_condition`（`while not done` 但循环里从不给 `done` 赋值，Python 程序靠 `break` 退出）、`eq_op`、`nonminimal_bool`、`overly_complex_bool_expr`（`a and a`、`a and not a` 这样多余的逻辑），以及 M10 转换 skime 时遇到的两个：`collapsible_if`（Python 里没有 `else` 的 `if` 套 `if`，照原样保留嵌套）、`module_inception`（`pkg/pkg.py` 这样和包同名的模块，例如 `skime/compiler/compiler.py`）。它们说的是 Python 程序本身，不是生成方式
- 所有用例生成到 `target/e2e/` 下同一个 cargo workspace 里一次构建，`CARGO_TARGET_DIR` 共用，PyO3 只编译一次；workspace 复用仓库的 `Cargo.lock`，离线也能解析到同一个 PyO3 版本
- rustfmt 的配置随包附带（`rustast/rustfmt.toml`，edition 与 style edition 都是 2024），用 `--config-path` 显式指定，不受运行目录影响，并复制到生成的 crate 里
- **端到端差分测试**：比较 `python case.py` 与生成的可执行程序的 stdout、退出码和 stderr。这些 crate 用 `--traceback-lines` 生成，有 traceback 时比较完整的 traceback（M4 起原生函数也有帧，第 9 节），只去掉原生帧没有的 `^^^` 行和 CPython 编译时的 `SyntaxWarning`；少数用例（直接调用的递归，§12）只比较异常类型（`# steelix-test: compare-exception-type-only`）。未捕获异常的用例另外用 `2>&1` 比较输出顺序。ext 模式只保留少量冒烟测试（`python -c "import case"`）。用 pytest marker `e2e` 区分
  - 文档化的差异里有 CPython 成功而生成的程序报错的情况（§12 里被替换的直接调用）。这类用例用 `# steelix-test: expect-error 消息` 注释写明期望的错误，测试检查输出是 CPython 输出的前缀，并且最后一行是这条消息
  - stdout 里文档化的差异（§12 里原生函数运行时没有解释器帧对 `logging`、`warnings` 的影响）写在 `tests/test_e2e.py` 的 `KNOWN_DIFFERENCES` 里：CPython 的哪一行在生成的程序里应该是什么样子（正则），其余行必须相同。差异消失时测试失败，提醒删掉这一项
  - 带 `# steelix-test: interrupt after the first line of output` 的用例会一直运行，测试在它输出第一行之后发送 `SIGINT`，两边都必须以 `KeyboardInterrupt` 结束，退出码相同
  - 默认模式（没有 marker）另外用一个小 workspace 构建几个有 traceback 的用例（`uncaught`、`tracebacks`、`exceptions_uncaught`、`with_stmt`），要求 stdout、退出码、最后一行相同，traceback 的帧（文件名和函数名）顺序和 CPython 一样，原生帧的行号是 `def` 行
  - 混合模式：保留为 Python 的代码的 traceback 帧必须和 CPython 的最后几帧完全相同（文件名、行号、源码行）。bin 模式下的 panic 必须以退出码 1 结束（用手写的模块文件替换生成的文件来触发）
- **真实应用**：`tests/apps/` 是从本机 CPython 3.14 标准库原样复制的命令行工具（calendar、base64、tabnanny、tokenize、ast、json.tool，文件开头注明来源和 PSF 许可），以混合模式和 `--traceback-lines` 转换，在几组命令行参数下与 CPython 运行同一份副本比较 stdout、退出码和 stderr（没有 traceback 时完全相同；有 traceback 时比较每一帧的文件名、行号、函数名和第一行源码，以及最后一行和 traceback 之前的输出）。它们的覆盖率数据见 `coverage.md`
- **项目**（M6，`tests/test_projects.py`）：`tests/projects/` 下是多模块的程序。`inventory` 是专门写的测试应用，覆盖带 re-export 的包、子包、相对 import、循环 import（包括 partially initialized 的报错）、import 的副作用顺序、模块间共享的模块级状态、跨模块的子类和异常、`importlib.import_module`（静态能找到的和 `--include` 的）、`importlib.reload`、非入口模块里的 `__name__ == "__main__"`、按名字 import 入口模块、跨模块替换函数、`--exclude` 的模块、子模块里保留为 Python 的生成器穿过别的模块的原生帧抛出异常，以及 `python -m store`。`stdlib_json` 是标准库 `json` 包的原样副本（加上来源和 PSF 许可的注释）和一个驱动脚本，它遮蔽了标准库的 `json`，C 加速模块 `_json` 照常在 import 时被选中，纯 Python 的 scanner、编码器另外显式调用；还以 `python -m json` 运行 `json/__main__.py`。`namespaces` 是命名空间包里的模块（嵌套的命名空间包、命名空间包里的普通包、相对 import），以及被标准库的 `json` 遮住的命名空间包 `json/`。`skime`（M10）是一个 Scheme 实现（MIT 许可，约 4800 行）的原样副本（M12 起加上它的测试目录 `tests/`，steelix 自己的 pytest 不收集它）加驱动脚本 `app.py`，把 `programs/` 下的 Scheme 程序喂给它的 REPL 或用嵌入 API 加载，覆盖尾递归、call/cc、宏、向量和字符串，以及从 VM 深处抛出的未捕获异常。每个项目生成的整个 `src/` 是 golden（`tests/golden/projects/`，M7 起 `main.rs` 里没有绝对路径，不用替换；skime 的有 1.5 万行，不做 golden），默认输出和 `--traceback-lines` 的输出都要求编译没有警告、clippy 干净，后者在几组参数下和 CPython 运行同一份源码比较 stdout、退出码和完整的 traceback（文件名、行号、源码行；`-m` 时去掉 CPython 的 `<frozen runpy>` 帧）。另有包形式的 ext 模式的冒烟测试（M12 起也以 `pkg/__init__.so` 运行 `python -m store`），`steelix test` 的测试（`inventory` 的 `store` 和 skime 自己的 108 个测试，§15），以及 `tests/test_project.py` 里模块发现、布局命名、跨模块替换识别和命令行的单元测试
- **属性读写索引**（M12，`tests/test_attrs.py`）：`tests/attrs/shapes` 是一个小项目，覆盖 `self`、继承的属性、嵌套函数里的 `self`、私有名字改编、import 别名、注解和构造推断出的接收者、不确定时当作未知的几种情况（局部变量重新绑定、`nonlocal`、模块里绑定两次的类名）、类属性的写入、动态访问、dataclass、`__slots__`、metaclass 和类体里的逻辑。索引的文本和几个类的 `#[pyclass]` 草稿是 golden（`tests/golden/attrs/`，`--update-golden` 更新），另外对 skime 的 `Context` 检查实例属性和未知接收者所在的模块，并把它的索引和草稿也做成 golden。草稿的桩在一个只依赖 PyO3 的 crate 里 `cargo check`（marker `cargo`），runtime 的 `Module::error` 有 Rust 单元测试
- **分发**（M7，`tests/test_dist.py`）：`tests/dist_apps/hello` 用到 json、`subprocess.run([sys.executable, "-c", ..])` 和一个第三方包，`tests/dist_apps/mp` 用 spawn 和 forkserver 的 `Pool.map`（模块级的原生函数、`__main__` 里定义的类的实例、在 worker 里抛异常）和 `ProcessPoolExecutor`。构建用的 venv 是 `target/e2e/dist-venv`，用 `python -m venv --without-pip` 创建，`site-packages` 里直接写入一个本地包 `greeting`（加上 dist-info），不需要网络，路径固定，cargo 缓存的依赖在两次测试之间仍然有效。`steelix build` 的结果在 `bwrap` 里运行：只挂载 `/usr`（和 `/lib`、`/bin` 等指向它的链接）、`/etc/ld.so.cache`、`/proc`、`/dev`、空的 `/tmp`，分发目录只读地挂在无关的 `/opt/program`，环境变量只有 `PATH`，看不到 `$HOME`、uv 的 Python、仓库和 venv（测试本身检查这一点）。在里面运行 hello（有源文件和 `--no-sources` 两种，比较输出和 traceback 的帧，有源文件时才有源码行）、json 项目的脚本和 `-m json`（`--no-sources`，和 CPython 比较 stdout、退出码和最后一行 stderr）、mp（`--no-sources`，和 CPython 的输出完全相同，worker 里也调用项目的包 `mptools` 的函数；另外以 `-m mptools.cli` 构建，`src/` 里没有 `mptools` 目录，spawn 和 forkserver 的子进程仍然能 import 转换过的子模块）、`tests/dist_apps/pkgapp`（`--no-sources`，包里的数据文件经过 `importlib.resources` 和 `pkgutil.get_data`，附带的解释器在项目目录里运行 `-m pkg.cli`，在别的目录里 import 不到项目模块，构建时对没有复制的顶层模块 `dyn.py` 给出警告）。M7a 加了几个回归测试：程序收到 multiprocessing 子进程的命令行时和 CPython 一样把它当作参数（分发和开发构建都测），分发在 bwrap 外面用假的 `HOME`（里面有 `usercustomize.py`）和 `PYTHONPATH` 等变量运行，输出不变，`STEELIX_PYTHON_ENV=1` 时生效，标准库和 site-packages 的 `.pyc` 头部是 unchecked-hash。另外检查程序里没有构建机器的路径、`--archive` 解压后的布局和链接，以及开发构建的 mp 在源文件删掉之后仍然正确。没有 `bwrap` 时这些测试跳过。dist 的组装、`project_files`（包里的数据文件）、没有复制的顶层模块的警告、`compile_tree`（`.pyc` 的校验方式、编译不过的文件、`compileall` 失败）、`inspect_python`（包括没有 `METADATA` 的 `.dist-info` 和 `include-system-site-packages = true` 的 venv）、rustflags 有单元测试，runtime 识别 multiprocessing 命令行和开发构建子进程标记的 Rust 单元测试也由 pytest 运行。ext 模式的测试另外用 `python -m store.__main__` 检查 `NativeLoader.get_code`
- **协程和生成器**（M13）：PoC 阶段用 `tests/test_poc_async.py` 测手写的 Rust（`tests/poc_async`），lowering 完成后这两者退役，同一个程序成为 e2e 用例 `tests/cases/coroutines.py`，由 lowering 生成，和别的用例一样比较 stdout、stderr 和退出码，golden 也一样要 clippy 干净。手写的文件如果留着，就得和生成的代码一起维护，又不再规定生成器不生成的东西。用例里 CPython 自己的几个协程和生成器写在 `exec` 的字符串里，用来和原生的混用（原来它们是保留为 Python 的驱动代码）。不同的是 `inspect.iscoroutine`、§12 记录的正在处理的异常的几处差异（`KNOWN_DIFFERENCES`）、递归太深时 `RecursionError` 的信息（只打印异常类型）和警告的行号，原生代码没有 Python 帧，所以用例打印警告时不打印行号（§12）。`async_basics`、`async_with_for`、`async_uncaught`、`async_kept`、`generators` 覆盖 lowering 的各条规则和保留为 Python 的写法
- **语料**：`tests/corpus/` 下是模仿真实小程序的脚本（文本处理、argparse、路径处理、JSON、图算法、计算器等），只用已支持的子集，但把很多特性组合在一起。要求能转换、编译没有警告，输出与 CPython 一致。`tests/cases/` 每个用例针对一个特性，语料用来发现组合起来才出现的问题
- **示例应用**：`tests/test_examples.py` 检查 `examples/*/generated` 与当前转换结果完全相同，并且 `generated` 和 `rewritten` 两个版本在几组命令行参数下的输出、退出码都与 CPython 一致，改写出来的原生代码的单元测试通过
- **生成代码检查器**（§2）在每次转换时运行，所以 golden、语料和 e2e 都经过它，`tests/test_verify.py` 另有针对每条规则的单元测试
- **`PYO3_PYTHON` 必须显式设置**。`.cargo/config.toml` 只对仓库内的构建生效，生成到仓库外的 crate 会悄悄用系统 Python 构建
- 生成的 `Cargo.toml` 依赖 `steelix-runtime` 的方式：在源码 checkout 里运行时（测试和开发）用 path 依赖，指向仓库里的 `crates/steelix-runtime`；从 PyPI 安装的 steelix 找不到这个目录，写成 crates.io 上的版本（`emit/crate.py` 的 `RUNTIME_VERSION`，现在是 `"0.2"`）。`--runtime-path` 可以显式指定。所以 runtime 必须和 PyPI 上的 steelix 同步发布到 crates.io，版本的前两位一致
- **发版**：PyPI 上的 `steelix` 和 crates.io 上的 `steelix-runtime` 一起发布，git tag 用 `v` 前缀（`v0.1.0`）。改了 runtime 的公开接口（生成代码用到的部分）时提升 `RUNTIME_VERSION`
- 源码映射：函数前的 `// file.py:LINE` 注释，异常的 traceback 里有原生函数的帧，行号是 `def` 行，`--traceback-lines` 时语句前有 `frame.line(N);`，行号精确到语句（第 9 节）
- **CI**（`.github/workflows/ci.yml`）：GitHub Actions 在 push 和 PR 时运行，`uv python install 3.14.9`、Rust 1.95.0（固定版本，和开发用的工具链一致，新版 clippy 增加的 lint 不会让 CI 突然失败，升级时两边一起改），`uv sync --locked` 之后跑 `ruff check`、`ruff format --check`、`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 和完整的 `uv run pytest`（含 e2e、cargo、dist）。uv 和 cargo（包括 `target/e2e`）都有缓存。分发测试需要 `bwrap`，CI 用 apt 安装，并且关掉 Ubuntu 24.04 对非特权 user namespace 的 AppArmor 限制（`kernel.apparmor_restrict_unprivileged_userns=0`），否则 `bwrap` 无法启动。测试不依赖仓库以外的固定路径：Python 来自 `.venv`（`uv sync` 创建），分发测试从解释器本身推出标准库和 site-packages 的位置，没有 `bwrap` 时跳过

## 15. 把函数改写成原生 Rust（M3b）

迁移的第 3 步是人逐个把生成的函数改写成地道的 Rust，程序的其余部分照常工作。还保留为 Python 的函数也可以手工改成原生代码，步骤见第 13 节。`examples/logsum` 是完整的例子：`generated/` 是转换的原样输出，`rewritten/` 把其中一个函数改写到第一步、两个函数改写成了原生 Rust，`tests/test_examples.py` 验证两者的输出都与 CPython 一致（除了下面说的一处有意的差异）。

### 两步走

生成的代码里，Python 的 list、dict 仍然是 libpython 的容器（`Bound<PyList>`、`Bound<PyDict>`），这是预期的起点。改写分两步：

1. **先改逻辑，数据留在 Python 容器里。** 函数体改成用 PyO3 的具体类型 API 直接操作原来的对象：`cast::<PyDict>()`、`cast::<PyList>()`、遍历、`get_item`、`set_item`，没有拷贝。参数和返回值仍是原来的 Python 对象，调用方和读取这些对象的其他函数都不受影响。例子里的 `tally` 就停在这一步：它返回的 dict 还被两个没改写的函数读取
2. **生产方和所有读取方都改写后，再把容器换成 Rust 类型。** 一个 dict 记录变成 `struct`、一个计数 dict 变成 `HashMap` 或 `Vec`，要等创建它的函数和所有读取它的函数一起改写。只改其中一个，就要在边界上来回转换，既慢又容易在转换处丢掉语义（顺序、键的类型）。例子里的 `parse_line` 和 `percentile` 做完了这一步，它们的数据在适配层里转换一次

类的两步见下面的"类的改写"。


### 转换放在哪里

生成的函数本身就是面向 Python 的适配层。它的签名 `(m: &rt::Module<'py>, x: &Bound<'py, PyAny>, ..) -> PyResult<Bound<'py, PyAny>>` 是一个契约，有两类调用方依赖它：`rt::def!` 注册的函数对象（Python 代码、key 函数、装饰器都经过它调用），以及生成代码里的直接调用。改写时不改这个签名，而是

1. 把逻辑写成一个新的普通 Rust 函数，参数和返回值用原生类型，不出现 `Bound`、`Python`、`PyResult`
2. 生成的同名函数只留下类型转换：用 `extract` 把参数转成原生类型，调用新函数，再用 `into_bound_py_any`、`PyDict::set_item` 等把结果转回 Python 对象

所以 `extract`/`into_pyobject` 只出现在适配层里，每个函数一个地方。

生成的函数体包在 `rt::frame!(m, "name", N, |_| { .. })` 里（§9），改写时按情况处理

- 函数自己的逻辑不再抛 Python 异常（错误由原生代码的 `Result` 表达，适配层只做转换）时，去掉这个包装，下面的 `percentile` 就是这样。这时 traceback 里没有这个函数的帧，异常显示在调用方的帧里
- 想在 traceback 里继续看到这个函数时，保留两行的包装 `rt::frame!(m, "name", N, |_| { .. })` 把适配层包起来，`N` 仍是 `def` 行，不用跟着改写调整
- 默认的输出里没有 `frame.line(N)` marker，改写时没有要同步的行号。用 `--traceback-lines` 生成的代码改写时，改过的部分直接删掉 marker 就行，留着过期的 marker 会让 traceback 报错的行号

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

多模块的项目（§7，M6）里每个 Python 模块有自己的 Rust 文件，原生代码同样放在它旁边：`store/items.py` 的是 `src/store/items/native.rs`，由 `src/store/items.rs` 开头的 `mod native;` 引入；包 `store/__init__.py` 的是 `src/store/native.rs`，由 `src/store/mod.rs` 引入（包里有叫 `native.py` 的子模块时换一个名字）。模块表在 `main.rs` 里，改写不需要动它。原生代码之间可以跨模块直接调用（`crate::store::items::native::parse`），它们不经过 Python，不受上面说的跨模块替换影响；生成的适配层之间的调用仍然经过模块属性（§7），不要改成直接调用适配层，否则别的模块对这个函数的替换就失效了。

没有为每个函数另外生成一个转发用的适配函数（比如注册 `count_words_py`，由它调用 `count_words`）。改写之前两者完全一样，每个函数多出一个空转发，生成代码变长；改写函数体的签名之后，生成代码里的直接调用全部编译失败，要逐个在调用处加转换。现在的约定下调用方不用动。也没有让 `rt::def!` 按 `FromPyObject`/`IntoPyObject` 自动转换参数和返回值（类似 `#[pyfunction]`），那样可以直接改函数签名，但转换藏在宏展开里，出错时报在宏里，直接调用同样会全部失败。

### 改写步骤

下面是第二步（换成原生类型）的步骤，第一步只需要第 1、4 步，改的是函数体本身。

1. 选一个函数，读它的 Python 源码和调用方，确定参数和返回值的原生类型。可能是 `None` 的值用 `Option`，固定键的 dict 可以变成 struct。按调用图从叶子往上改写最省事
2. 在原生模块里写新函数，顺手写单元测试
3. 把生成的函数体换成转换加调用。原来返回 dict 或 tuple 的，在适配层里按原来的键和顺序构造，Python 一侧看到的结构不变
4. 跑端到端测试和项目自己的测试（下一节），确认输出仍与 CPython 一致

转换要与原来的 Python 语义对齐，对不齐的地方要有意识地决定。例如 `extract::<u64>()` 对负数抛 `OverflowError`、对 float 抛 `TypeError`，原来的 Python 代码也许接受它们；Rust 的 `split_whitespace`、`parse` 和 Python 的 `str.split`、`int()` 在非 ASCII 空白、负号、带下划线的数字、非 ASCII 数字等边界上不同（例子里的 `parse_line` 有意把这些行当成格式错误，README 和测试里写明了）；`HashMap` 没有插入顺序，转回 Python 的 dict 顺序会变，如果下游依赖 dict 的顺序（直接遍历输出，或者排序时只比较部分键），就要用 `Vec` 或保持顺序的 map。

### 用项目自己的测试检查（M12）

改写时最有用的检查是项目本来就有的测试套件。M10 改写 skime 时只能另外生成一个 ext 模式的 crate，把改过的模块文件复制过去再跑 pytest。M12 起直接用 bin：

```
steelix transpile -m pkg --root . -o out/
steelix test out/ -- tests/ -x
```

`steelix test CRATE [pytest 的参数]` 用 `cargo build` 构建 crate（`PYO3_PYTHON` 是 `--python`，默认 `$PYO3_PYTHON`，再没有就是运行 steelix 的解释器，和 `steelix build` 一样；`--release` 用 release profile），然后在当前目录运行 `python -m pytest 参数`，这里的 `python` 是可执行文件旁边 `steelix-test/bin/python` 这个指向程序的链接，程序于是处在解释器角色（§11），是完整的 python。它只构建、不转换，改写过的模块文件不会被覆盖。

- **原生的 import**。解释器角色平时只在 CPython 能找到源文件时回答项目模块的 import（§7、§11），`steelix test` 设置 `STEELIX_NATIVE_IMPORTS=1`，让它和程序一样回答所有转换过的模块。测试在哪个目录运行、项目目录在不在 `sys.path` 上（项目通过 editable 安装找到、src 布局、`--import-mode=importlib`）都不影响，测试里 `import pkg` 得到的总是原生代码，`__file__`、`__spec__` 和从源码 import 一样指向项目目录里的文件。没有转换的项目模块照常从源码 import
- **检查**。`steelix test` 加载一个 pytest 插件 `steelix_native`（`src/steelix/pytest_native.py`，复制到 `steelix-test/plugins/`，经过 `PYTHONPATH` 加载，只用标准库和 pytest，目标环境里不需要装 steelix）。结束时它报告导入了多少个转换过的模块（`steelix: 24 of 24 transpiled modules imported natively from ..`），以及项目目录里从源码 import 的其它模块；某个转换过的模块的 loader 不是 `NativeLoader`（被测试或别的工具从源码重新加载），或者 `sys.meta_path` 上根本没有 `NativeImporter`（解释器不是生成的程序）时，测试全部通过也算失败，退出码 1。插件靠 `NativeImporter` 新加的只读属性 `modules`、`root` 知道模块表
- **pytest 从哪里来**。开发构建的解释器用构建时那个 venv 的 site-packages（§11），pytest 和项目的依赖要装在那里。只能用 `python -m pytest`，`pytest` 命令的 shebang 指向 CPython。`-m` 让当前目录成为 `sys.path[0]`，和平时一样
- **测试文件和 conftest.py**。它们一般不在模块表里（`steelix transpile` 只转换入口静态 import 到的模块），照常从源码 import，测试代码本身仍然是 Python。即使被转换了（例如 `--include` 了整个目录），pytest 的断言改写 hook 排在 `sys.meta_path` 最前面，也会先从源码加载它们（`--assert=plain` 时除外）。包里的测试（`pkg/tests/test_x.py`）按 `pkg.tests.test_x` import，`pkg` 是原生的，`pkg.tests` 没有转换时从源码 import
- **模块表**。只有转换过的模块是原生的。`steelix transpile app.py` 只转换入口脚本静态 import 到的模块，入口脚本自己只在被按名字 import 时才在模块表里，测试要 import 的模块不在里面时用 `--include` 加进来，插件在 `-v` 时列出没有被 import 的转换过的模块
- **没有覆盖的**。开发构建的 `sys.executable` 是 venv 的 python（§11），测试里 `subprocess.run([sys.executable, ..])` 和 multiprocessing 的 spawn、forkserver 子进程是 CPython，从源码 import 项目模块。monkeypatch 和原生代码的关系和程序一样（§7、§12）

不用 steelix 的命令也可以：`ln -s out/target/debug/app python && STEELIX_NATIVE_IMPORTS=1 ./python -m pytest tests/`，只是没有插件的检查。

验证（`tests/test_projects.py`）：`inventory` 的包 `store` 在一个和项目无关的目录里跑测试，项目目录不在 `sys.path` 上，测试检查 loader 是 `NativeLoader`，包里的测试文件也能运行，把转换过的模块换成从源码加载时运行失败。skime 自己的 108 个测试（`tests/projects/skime/tests`，skime 0.3.1 的原样副本）全部通过，结果和时间见 `m10-skime.md` 第 3 节。

### 调用方

- 生成代码里的直接调用不用改，它们仍然调用适配层，每次调用付出一次转换
- 两个函数都改写成原生代码后，原生函数之间直接调用，不经过适配层，也不需要 `m`
- 原生函数要调用还没改写的函数时，它就还不是纯 Rust。要么先改写被调用的函数，要么让它接收 `m: &rt::Module<'py>`，在调用处转换参数后调用对方的适配层
- 改变 Python 可见的签名（参数、默认值）时，要同时改 `rt::def!` 那一行和所有直接调用，和在 Python 里改函数签名是一样的工作

### 经过 Python 调用的函数

作为 key 函数传给 `sorted`、注册成回调、赋给别的变量或者被装饰（`@functools.lru_cache`）的函数，都经过 `rt::def!` 创建的函数对象调用。函数对象按 Python 的规则绑定参数，再调用生成的函数，也就是适配层。所以这些函数的改写方式完全一样，不需要额外处理。装饰器包装的仍然是函数对象，`functools.wraps` 看到的名字和 docstring 来自 `rt::def!` 那一行，不受改写影响。

嵌套函数和 lambda 也一样：生成的 fn item 就是适配层，捕获的变量（`&Bound` 或 `&rt::Cell`）在适配层里取出、转换后传给原生函数。

### 类的改写（M5）

每个类生成一个 mod（§10），改写也以 mod 为单位，两步和上面对应：

1. **方法逐个改写，实例状态留在 `__dict__` 里。** 方法的 fn 就是适配层，签名 `(m, slf, ..)` 和普通函数一样不改。方法体可以把属性读出来转成原生类型，调用原生模块里的函数，再写回 `slf.setattr(..)`。只要读写的属性名和值的类型不变，同一个类的其他方法、子类和 Python 代码都不受影响。`rt::class!` 那一行和 `body` 不用动。也可以手工把类里保留为 Python 的方法改成原生的：在 mod 里加一个 fn，把 `body` 里的 `rt::exec_python!(ns, ..)?;` 换成 `rt::def!(ns, name(slf as "self", ..))?;`，方法里用了 `super()` 时在 `body` 开头 `let __class__ = ns.class_cell();` 并按闭包捕获它（照着同一个文件里生成的方法写）
2. **所有方法都改写之后，再把实例换成 Rust 结构体（`#[pyclass]`）。** 实例的属性是方法之间共享的数据，和第二步换容器一样，要等读写它们的方法全部改写。`rt::class!` 的 `attributes = [..]`（`__static_attributes__`）列出了方法给 `self` 赋值的属性，就是结构体字段的起点。改写时 mod 里的方法变成 `#[pymethods]`，`exec_module` 里的 `rt::class!(..)` 换成 `m.set_global("Point", py.get_type::<Point>())`。适用条件见 §10：只继承 object 或原生类型，没有 metaclass 和类体里的逻辑；pickle、`copy`、`__dict__`、被 Python 类继承要另外实现（`__reduce__`、`#[pyclass(dict, subclass)]`）。这一步不自动生成，原因见 §10。`steelix attrs --pyclass` 可以写一份草稿作为起点，属性被别的模块读写时先看下面的"被多个模块共享的类"

和函数一样，`self.method()` 在生成代码里总是动态调用，改写成原生代码之后，同一个结构体的方法之间可以直接调用，不经过 Python。

第二步的具体约定（M8a，来自 M5 之后的 review）

- **文件位置。** 原生代码和函数一样放在 `src/<module>/native.rs`，由 `src/<module>.rs` 开头的 `mod native;` 引入。`#[pyclass]` 结构体本身要用 PyO3，放在 `src/<module>.rs` 里原来那个类的 mod 中，`native.rs` 只放不依赖 PyO3 的数据和逻辑，结构体的字段可以直接是 `native` 里的类型
- **类属性用 `#[classattr]`。** 类体里的常量（`UNITS = ("B", "KB")`、`limit = 10`）写成 `#[classattr] fn UNITS(py) -> ..`，Python 一侧通过类和实例都能读到，和原来一样。会被重新赋值的类属性（计数器之类）不能这样写，`#[classattr]` 只在建类时求值一次，这种属性留在类型的 `__dict__` 里（`type.setattr`）
- **没有可变状态的类用 `frozen`。** 方法只读字段、所有字段都在 `__init__` 里赋值一次的类写成 `#[pyclass(frozen)]`，`&self` 访问不用运行期借用检查，也不会有 `PyBorrowMutError`。字段会被改的用普通的 `#[pyclass]` 加 `#[pyo3(get, set)]`
- **Python 侧的形状要写明。** 字段的原生类型和 Python 一侧看到的值不一定一一对应。例如原来是 tuple 的属性存成 `Vec<String>` 或 `[f64; 2]` 时，`#[pyo3(get)]` 会转成 list，`p.coords == (1.0, 2.0)` 就变成 False，`hash(p.coords)` 直接报错。这类属性要手写 getter，每次重新构造 tuple（`PyTuple::new(py, &self.coords)`），setter 同样按原来接受的类型转换。改写前先列出每个属性在 Python 一侧的类型，改写后逐个对照
- **`__module__`、`__qualname__` 和 pickle。** `#[pyclass(module = "app", name = "Point")]` 写明模块名和类名，否则 `__module__` 是 `builtins`，`repr`、错误信息和 pickle 都会出错。嵌套类的 `__qualname__` 要和原来的 `Outer.Inner` 一致。pickle 和 `copy` 需要 `__reduce__`（返回类本身和构造参数），并且类要能通过 `module.qualname` 找到，也就是 `exec_module` 里照旧 `m.set_global("Point", ..)`
- **`__repr__` 要手写。** 原来没有定义 `__repr__` 的类，`repr` 是 `<app.Point object at 0x..>`，`#[pyclass]` 默认的也类似，但原来用 `@dataclass` 生成的 `Point(x=1, y=2)` 要手写 `__repr__` 照原样格式化（字段顺序、`repr()` 每个字段、嵌套对象调用它们自己的 `repr`）。`__eq__`、`__hash__`、排序这些 dataclass 生成的方法同样要手写，`__eq__` 遇到别的类型返回 `NotImplemented`

### 被多个模块共享的类（M12）

上面的第二步假定读写实例属性的都是类自己的方法。真实项目里不一定：M10 改写 skime 时，`Context`（`skime/ctx.py`）的 `ip`、`stack`、`env`、`parent`、`bytecode` 被 `insns.py`、`invoke.py`、`call_cc.py` 等模块直接读写，`push`、`pop` 也在这些模块里调用。改写这样的类之前，先用属性读写索引列出谁读写了它的哪些属性

```
steelix attrs -m skime --root . --class Context     # 一个类，文本
steelix attrs app.py --json                         # 所有类，JSON
```

选择程序的参数和 `steelix transpile` 一样（入口脚本或 `-m`，`--root`、`--include`、`--exclude`），分析的是同一组模块，不生成 crate。每个类列出子类、在类的方法以外读写它的函数个数和模块、在哪里构造它，然后是每个实例属性的每次读、写、增量赋值和 `del`（`file:line`、所在函数、接收者），以及类属性、在实例或类上用到的方法和动态访问

```
class shapes.Point  (shapes.py:7, top level)
  subclasses: shapes.Point3 (shapes.py:31)
  used outside its methods by 5 functions in 2 modules: app, shapes
  instance attributes
    label  (not in __static_attributes__)
      write  app.py:16  origin  o.label  [inferred: o = P(..)]
    x
      write      shapes.py:12  Point.__init__  self.x
      augmented  shapes.py:21  Point.move      self.x
      read       shapes.py:37  Point3.flat     self.x
      augmented  app.py:10     scale           p.x  [inferred: p: P]
```

只有接收者的类能保守地确定时，访问才记在这个类下面（`attrs.py`）

- 方法的 `self`（类体里函数的第一个参数，和 `__static_attributes__` 的规则一样，staticmethod 除外）、classmethod 的 `cls`，包括没有重新绑定它们的嵌套函数和 lambda。子类的方法读继承来的属性时记在定义它的基类下面
- 注解为这个类的参数（`p: Point`、`p: "Point"`、`Point | None`、`Optional[Point]`），以及每个绑定都是 `x = Point(..)` 或 `x: Point = ..` 的局部变量。注解不强制，这两种标成 inferred
- 类对象本身：`Point.count`、`cls.count`、`type(self).count`、`self.__class__.count`
- 名字只有在模块只绑定它一次（`class` 语句或 `from .. import`，可以经过 re-export），模块没有 `import *`、`globals()` 这类看不见的绑定，别的模块也没有给它赋值时，才认为是这个类。局部变量还有别的绑定、被嵌套函数 `nonlocal` 改写，都当作未知

其余接收者的类未知。属性名是某个项目类的实例属性的访问单独列在一节里，方法名同样另列一节，审查时只需要看这两节，不用 grep 整个项目。skime 的 `insns.py` 里 `ctx` 是没有注解的参数，`ctx.ip`、`ctx.push(..)` 都在这里，结果见 `m10-skime.md` 第 6 节。动态访问（名字不是字面量的 `getattr`/`setattr`/`hasattr`/`delattr`、`vars(x)`、`x.__dict__`、`object.__setattr__`）在接收者已知时记在类下面，否则按函数列出。`getattr(p, "x")` 这样的字面量名字按普通的读写处理。没有做的推断：属性的类型（`self.ctx.ip` 里 `self.ctx` 的类）、返回值的类型（`ctx = proc.ctx.clone()`），项目以外的代码（测试、插件）也看不到。

有了这张表，共享的类按下面的顺序改写

1. **用原生函数取代类的用途时，改写入口，保留类。** M10 的读取器只改写了 `parse`，逻辑所在的 `Parser` 类仍是生成的代码。索引显示 `Parser` 只在 `parse` 里构造，方法以外没有读写方，就能确定改写后 skime 自己不再用它，留给直接使用它的 Python 代码
2. **共享的状态先停在第一步。** 属性留在 `__dict__` 里，各个模块的读写方各自改成对原来的 Python 对象的具体操作，按索引逐个改。只要属性名和值的类型不变，没改写的读写方不受影响

    ```rust
    // op_push_local: `ctx.push(loc)`, inlined (next section)
    let stack = ctx.getattr(intern!(py, "stack"))?.cast_into::<PyList>()?;
    stack.append(loc)?;
    ```

3. **索引里的读写方全部改写之后，再做第二步。** 包括未知接收者那两节里确实是这个类的访问。改写前可以让 steelix 写一份 `#[pyclass]` 草稿

```
steelix attrs -m skime --root . --pyclass skime.ctx.Context > context.rs
```

草稿只是给人的建议：不接入构建，不自动生成，函数体都是 `todo!()`（不自动升级的原因见 §10）。文件开头列出索引发现的会阻碍或增加升级难度的事实：项目里的子类、object 以外的基类、metaclass、类装饰器（dataclass 等）、类体里 def 和常量赋值以外的逻辑、`__getattr__` 这类 hook、`__slots__`、动态访问和 `__dict__`/`vars` 的使用、被重新赋值的类属性、在类的方法以外赋值的属性，以及类的方法以外的读写方和同名的未知接收者。后面是结构体，字段来自实例属性（`__static_attributes__`、dataclass 的字段、`__slots__` 和在别处赋值的属性），类型是 `Py<PyAny>` 加 TODO，注释写明哪些函数读写它；`#[pymethods]` 里每个方法一个桩，带 Python 的签名和它取代的生成的 fn（`context::push` in `src/skime/ctx.rs`），`__init__` 变成 `#[new]`（注释列出构造它的地方），property、staticmethod、classmethod 换成对应的属性；dataclass 另有 `__init__`、`__repr__`、`__eq__` 的桩，常量类属性是 `#[classattr]`，最后是 pickle 和 `copy` 的提醒

```rust
// What blocks or complicates the upgrade:
// - names of its attributes are also used through receivers of unknown class: ..
//   `ip` 30x in skime.call_cc, skime.insns; ..
// - names of its methods are called through receivers of unknown class: `clone`,
//   `insert`, `pop`, `pop_n`, `push`, `top` (inlining them in the callers changes
//   what a subclass override does).

#[pyclass(module = "skime.ctx", name = "Context")]
pub struct Context {
    /// `self.ip`, assigned in Context.__init__, Context.clone, read in Context.clone,
    /// Context.__str__.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    ip: Py<PyAny>,
    ..
}

#[pymethods]
impl Context {
    /// `def push(self, val)` (line 27), generated as `context::push` in src/skime/ctx.rs.
    fn push<'py>(&self, py: Python<'py>, val: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }
}
```

桩本身能通过 PyO3 的类型检查（`tests/test_attrs.py` 检查这一点），可以从能编译的代码开始，逐个换成真正的字段类型和方法体。

### 适配层构造别的模块的类、抛项目自己的异常（M12）

- **构造别的模块的类。** 类是 globals 里的一个名字，名字是这个模块 import 它时的别名：skime 的 `parser.py` 写的是 `from ..types.pair import Pair as pair`，所以是 `m.global("pair")?.call1((car, cdr))?`，不是 `"Pair"`。照着 `exec_module` 里的 import 或者同一个文件里生成的代码写。模块没有 import 这个类时用 `py.import("skime.types.pair")?.getattr("Pair")?`，转换过的模块 import 到的就是原生模块。要构造很多个时在适配层开头取一次，放进一个结构体里传给转换代码（M10 的 `Objects::new(m)?`），不要放进 `static`，`importlib.reload` 和测试替换模块属性之后会过期（§7）

    ```rust
    struct Objects<'py> {
        pair: Bound<'py, PyAny>,
        symbol: Bound<'py, PyAny>,
    }

    impl<'py> Objects<'py> {
        fn new(m: &rt::Module<'py>) -> PyResult<Self> {
            Ok(Self { pair: m.global("pair")?, symbol: m.global("sym")? })
        }
    }
    ```

- **抛项目自己的异常类。** `m.error("ParseError", (message,))` 按 globals 里的名字（同样是 import 的别名）构造异常，返回 `PyErr`，可以直接用在 `map_err` 里。查找、构造失败或者得到的不是异常时，返回的就是那个错误，和 `raise` 一样。异常的 traceback 由它经过的原生帧补上，适配层保留 `rt::frame!` 时有这一帧。它不设置 `__context__`，在 `except` 块里抛异常时仍用生成代码的写法 `frame.raise(m.global("ParseError")?.call1((..,))?)`

    ```rust
    let datum = native::parse(&text).map_err(|err| match err {
        native::Error::Empty => m.error("ParseError", ("Nothing to be parsed.",)),
        native::Error::Syntax { line, message } => m.error("ParseError", (format!("{name}:{line} {message}"),)),
        native::Error::NotAFloat(text) => PyValueError::new_err(format!("could not convert string to float: '{text}'")),
    })?;
    ```

    不用这个 helper 的写法是 `PyErr::from_value`，查找失败的错误要另外处理，M10 的读取器就自己写了这样一个函数

    ```rust
    fn parse_error(m: &rt::Module<'_>, message: String) -> PyErr {
        match m.global("ParseError").and_then(|class| class.call1((message,))) {
            Ok(exception) => PyErr::from_value(exception),
            Err(err) => err,
        }
    }
    ```

    内置的异常照常用 `PyValueError::new_err(..)`

### 内联别的类的方法（M12）

生成代码里 `ctx.push(x)` 总是动态调用（§10）。M10 改写主循环时把 `Context` 的 `push`、`pop`、`top` 内联成对 `ctx.stack` 这个 list 的操作，省掉了 Python 的方法调用，代价是下面这些不再成立，每一条都要确认并写进注释

- **子类覆盖。** `Context` 的子类重写 `push` 时，内联的代码不调用它。索引的 `subclasses` 列出项目里的子类（`Context` 没有），项目以外的子类看不到
- **替换方法。** `Context.push = ..`、`mock.patch.object(Context, "push", ..)` 对内联的代码不起作用，和 §12 里跨模块替换函数一样。项目里对类属性的赋值索引会列出
- **实例上的同名属性和 hook。** Python 的 `ctx.push` 先查实例的 `__dict__`，还经过 `__getattribute__`、`__getattr__`
- **方法依赖的属性。** 内联代码直接读写 `stack`，这个属性名和它的类型从此也是两边的契约，改 `Context` 时要同时改这里。索引会把这些访问列在未知接收者那一节
- **帧和异常。** 空栈时 `list.pop` 抛的 `IndexError: pop from empty list` 和原来一样，但 traceback 里少了 `Context.pop` 那一帧

接收者不一定是这个类时（`ctx` 也可能是子类或者别的对象），可以只对确切的类型走内联的路径，其余照旧动态调用，语义就和原来一样

```rust
let context = m.global("Context")?;
if ctx.get_type().is(&context) {
    push(ctx, &loc)?; // inlined `Context.push`
} else {
    ctx.call_method1(intern!(py, "push"), (&loc,))?;
}
```

### Python 源码本身是生成的（M12）

skime 的 `insns.py`、`iset.py` 由 `iset_gen.py` 从 `iset.yml` 生成（文件开头写着不要手改），改写后 `insns/native.rs` 里的 `Insn` 枚举是第二份指令表。原则是只保留一个来源，其余的由它生成或者和它对照

1. 最好让原来的生成器同时写出 Rust：`iset_gen.py` 再生成一个 `src/skime/insns/table.rs`（枚举、`from_opcode`、标签），`native.rs` 用 `mod table;` 引入，开头同样写明由谁生成。改指令只改 `iset.yml`
2. 改不了生成器时，加一个两份表不一致就失败的检查，例如 `native.rs` 里读同一份 `iset.yml` 的单元测试

    ```rust
    #[test]
    fn opcodes_follow_iset_yml() {
        // The project is next to the crate (`steelix transpile -m skime -o out`).
        let yml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../skime/iset.yml"));
        let names: Vec<&str> = yml.lines().filter_map(|l| l.trim().strip_prefix("name: ")).collect();
        let native: Vec<&str> = (0..names.len()).map(|op| Insn::from_opcode(op).unwrap().name()).collect();
        assert_eq!(native, names);
    }
    ```

3. 改写过的模块文件由人维护（下一节）。生成的 Python 变了（`iset.yml` 加了指令）时，把它重新转换到别的目录，和改写过的文件对照合并，第 2 条的检查会提醒这一步

### 改写之后不再成立的东西

- monkeypatch：原生函数内部不再经过 Python，替换它读的全局变量（`mock.patch("app.LIMIT")`）或它调用的函数都对它不起作用。替换函数本身的效果不变，只影响经过 Python 的调用（第 12 节）
- 异常：原生函数里没有 Python 异常，失败用 `Option`/`Result` 表示，由适配层转回去。约定是适配层抛出和原来的 Python 代码相同类型的异常，调用方（生成的代码或者保留的 Python 代码）的 `except` 才能照常接住；确实要改变行为时要有意识地决定并写明。例子里的 `parse_line` 遇到不是数字的大小或时间时返回 `Err(native::NotANumber(text))`，适配层把它转成和 `int()` 一样的 `ValueError`（`invalid literal for int() with base 10: '12x'`），生成的 `load` 照常用 `except ValueError` 把这一行当成格式错误。panic 经过 PyO3 会变成 `PanicException`，不要用它表达预期中的错误
- 数值语义：Python 的 int 没有上限，换成 `u64`/`usize` 后溢出在 debug 构建里 panic，在 release 构建里回绕。浮点运算的顺序变了，结果可能在最后一位上不同
- 异步：改写过的协程只能 await Python 的 awaitable 和原生协程的 future。Rust 生态的 future（tokio 等）返回 `Pending` 之后不会被唤醒，因为驱动协程的是 Python 的 `send`，waker 什么都不做（§8 "协程和生成器"）。要用它们需要一个用 asyncio 的 future 当 waker 的桥接 helper，还没有做。另外两条规则
  - 跨过 `.await` 持有的值只能是 Python 对象或者 `Send` 的值。协程和 CPython 的一样可以在别的线程上恢复或释放（asyncio 的 `run_coroutine_threadsafe`、几个线程共用的生成器），runtime 因此把 future 声明为 `Send`（§8 的 unsafe），`Rc`、`MutexGuard`、绑定线程的句柄跨过 `.await` 就是未定义行为。编译器查不出来，runtime 也不检查：按线程检查分不清这种错误和正当的跨线程恢复，而让函数体声明自己绑定线程又要求写代码的人已经知道这条规则。需要这类值时在两个 `.await` 之间创建并用完
  - 不要用 `join!`、`select!` 这类组合子同时 await 两个由 Python 驱动的 future（`rt::await_`、`rt::yield_`、`rt::anext`、`frame.async_with`、原生协程的 future）。一次 poll 只有一个通道，两个都暂停时后一个交出的值覆盖前一个，前一个就丢了（它等的 asyncio future 不会交给事件循环），恢复时送进来的值也只给先被 poll 的那个。并发等待用 Python 的 `asyncio.gather` 等，再 `rt::await_` 它的结果
- 重新生成：改写过的模块文件从此由人维护，不能再用 steelix 覆盖。Python 源码有变化时，重新生成到别的目录，再把没有改写过的函数手工合并过来

## 16. 待定问题

1. 参数传递用 `&Bound` 还是 `Bound`。初版用 `&Bound`，如果生成的 clone 太多再调整
2. ABI 策略：按版本编译还是 abi3。runtime 里的 ffi 调用尽量限制在 limited API 内，保留 abi3 的可能
3. free-threaded（3.14t）：不支持，runtime 的名字表等依赖 GIL，在 `Py_GIL_DISABLED` 下直接编译报错（M11a，§6）
