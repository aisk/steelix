# M10 真实项目验证：skime

用 [skime](https://github.com/aisk/skime)（一个用 Python 写的 Scheme 实现，MIT 许可，版本 0.3.1）完整走了一遍 steelix 的流程：转换、和 CPython 比较、用项目自己的测试套件测原生代码、`steelix build` 部署、按 §15 手工改写两处。skime 是包 `skime`，24 个模块，转换的部分有 4582 行 Python（包里另有开发工具 `iset_gen.py`，没有被 import，不转换），包括编译器（`compiler/`）、宏展开（`macro.py`）、字节码 VM（`vm.py`、`insns.py`、`ctx.py`）和 140 个左右的原语（`prim.py`），没有第三方依赖。CLI 是 `python -m skime` 进入的 REPL，从 stdin 一行读一个表达式。

所有工作都在 skime 的副本上进行，`~/skime` 没有改动。

## 结论

- 转换一次就成功，不需要 `--include`、`--exclude` 或改源码。函数 373/374 原生（修复复数字面量之后），剩下的一个是 generator
- 行为和 CPython 一致：8 个 Scheme 程序（REPL 方式）的 stdout、退出码和 stderr 最后一行全部相同，`tests/projects/skime` 里 7 组命令行在 `--traceback-lines` 下和 CPython 的完整 traceback 相同
- skime 自己的 108 个 pytest 用例对 ext 模式的原生包全部通过
- `steelix build` 的分发在 `bwrap` 里运行正确，有源文件和 `--no-sources` 两种都和 CPython 一样
- 发现 steelix 的 3 个问题，都已修复（复数字面量、两个 clippy lint）
- 最大的问题是性能：解释器这类"很多小函数互相调用"的程序，生成代码比 CPython 慢 6 倍。原因是 Python 调用原生函数的开销和属性名没有 intern，与改写无关，见"性能"和建议第 1 条

## 1. 转换和覆盖率

```
uv run steelix transpile -m skime --root skime -o out --report report.json
```

| | 修复前 | 修复后 |
| --- | --- | --- |
| 函数 | 372/374（99.5%） | 373/374（99.7%） |
| 模块级语句 | 122/122 | 122/122 |
| 行 | 4093/4128（99.2%） | 4122/4128（99.9%） |

保留为 Python 的

- `Parser.parse_number`（`compiler/parser.py`）：复数字面量 `1j`（`num1 + sign2 * num2 * 1j`）。已经支持，见"steelix 的问题"
- `iter_list`（`prim.py`）：generator（`yield`）。和 `coverage.md` 里别的程序一样，generator 是剩下的唯一障碍

生成的 Rust 有 14851 行（`prim.rs` 4742 行、`compiler/compiler.rs` 2542 行、`macro.rs` 2456 行），是 Python 的 3.2 倍。干净构建（含 PyO3）开发模式 61 s，只改生成代码后重新构建约 10 s，release 54 s。转换本身 3 s。生成的 crate 编译没有警告，修复后 clippy 也是干净的。

## 2. 和 CPython 比较

CLI 只有 REPL，所以 Scheme 程序从 stdin 喂进去，一行一个表达式。8 个程序覆盖阶乘（大整数）、fib、尾递归（10 万层）、`do` 循环、call/cc（提前返回和重新进入保存的 continuation）、`define-syntax`/`syntax-rules`（`swap!`、递归的 `my-or`、`while`）、quasiquote、向量、字符串、字符、复数、`display`/`write`、各种 skime 错误（REPL 打印 `!>` 继续运行）以及最后从 VM 深处抛出的未捕获 `ZeroDivisionError`（退出码 1）。

结果：stdout（把对象地址 `0x..` 换掉之后）、退出码和 stderr 最后一行全部相同。默认模式下原生帧的 traceback 行号是 `def` 行，和预期一样（§9）。程序在 `-m skime` 之外运行时也没有差别（运行目录是 `/`）。

`tests/projects/skime` 里的驱动脚本 `app.py` 用 `--traceback-lines` 构建，在 7 组命令行下比较完整的 traceback，从 `app.py` 经过 `vm.py`、`form.py`、`insns.py`、`invoke.py` 到 `prim.py` 的 12 帧（文件名、行号、源码行）都和 CPython 相同。

## 3. 用 skime 的测试套件测原生代码

`steelix transpile --mode ext -m skime` 生成包形式的扩展模块，`libskime.so` 复制成测试目录里的 `skime.so`，旁边只放 `skime/scheme/prim.scm`（`vm.py` 按 `__file__` 读取的数据文件，目录里没有 `__init__.py`，所以 `PathFinder` 仍然先找到扩展模块），再复制 `tests/`。`conftest.py` 检查 `import skime` 确实是原生的

```python
assert isinstance(skime.__spec__.loader, importlib.machinery.ExtensionFileLoader)
assert type(skime.vm.__loader__).__name__ == "NativeLoader"
assert type(skime.vm.VM.run) is not types.FunctionType
```

并在结束时列出所有 `skime.*` 模块的 loader（24 个，`ExtensionFileLoader` 和 `NativeLoader`）。

| | 结果 | 时间（3 次中位数） |
| --- | --- | --- |
| CPython | 108 passed | 0.17 s |
| 原生（生成的代码，debug 和 release 一样） | 108 passed | 0.43 s |
| 原生（改写两处之后） | 108 passed | 0.33 s |

`test_repl.py` 用 `monkeypatch.setattr(builtins, "input", ..)` 替换 `input`，原生的 `main()` 照样看到替换（`input` 是普通的 builtin 查找），这类测试不受影响。skime 的开发依赖 pyyaml 只有 `iset_gen.py` 用，测试都不需要它，没有跳过任何测试。

M12 起不再需要 ext 模式和复制：skime 0.3.1 的测试目录放进了 `tests/projects/skime/tests`（原样副本），`steelix test` 用 bin 的程序以解释器角色运行它们（design §15）

```
uv run steelix transpile -m skime --root tests/projects/skime -o out
cp -r tests/projects/skime/tests /tmp/run/ && cd /tmp/run   # 和项目无关的目录
uv run --project ~/steelix steelix test ~/steelix/out -- -q
```

项目目录不在 `sys.path` 上（skime 没有安装到 steelix 的 venv 里），`import skime` 仍然得到原生代码，插件报告 `24 of 24 transpiled modules imported natively`。`tests/test_projects.py` 的 `test_skime_tests_run_natively` 用 `app.py` 构建的程序做同样的事（模块表一样是 24 个模块）。

| M12，`steelix test` | 结果 | pytest 报告的时间（3 次） |
| --- | --- | --- |
| CPython（`PYTHONPATH` 指向项目目录） | 108 passed | 0.14 s |
| 原生，debug（opt-level 1，PyO3 和 runtime 是 3） | 108 passed | 0.25 s |
| 原生，release（LTO） | 108 passed | 0.23 s |

干净构建（含 PyO3）debug 47 s，之后的 `steelix test` 只有 cargo 的检查和 pytest 本身。

检查原生时发现一个小差异：`type(f).__module__` 不是字符串而是 getset descriptor（PyO3 的类型是 heap type，实例的 `__module__` getter 占了类型 `__dict__` 里的位置），已记入 §12。

## 4. 部署

```
python -m venv --without-pip venv        # 干净的 venv，skime 没有依赖
uv run steelix build -m skime --root skime --python venv/bin/python
```

| | 体积 | 文件数 |
| --- | --- | --- |
| `dist/skime/` | 61.1 MB | 1160 |
| `--no-sources` | 60.9 MB | 1136 |
| `--archive`（`--no-sources`） | 19.8 MB | |
| 其中程序本身 | 2.8 MB | |
| 其中 libpython | 33 MB | |

和 `tests/test_dist.py` 一样在 `bwrap` 里运行（只挂载 `/usr`，分发目录只读），8 个程序的结果和 CPython 相同，`--no-sources` 的 traceback 没有源码行，其他一样。启动时间 22 ms，`python -m skime` 是 20 ms。

问题：包目录下的所有文件都会复制（M7a 的规则），所以只在开发时用的 `iset_gen.py`（import yaml）和 `iset.yml` 也进了分发。不影响运行，只是多余，`--exclude` 只管转换不管复制。

## 5. 按 §15 改写

改写在生成的 bin crate 的副本里进行（`git init` 之后提交生成的结果，改写是它上面的 diff），改动了两个模块文件，新增两个 `native.rs`。改写后 8 个 Scheme 程序仍然和 CPython 一致，108 个测试全部通过，`cargo test` 跑原生模块的 4 个单元测试，clippy 和 rustfmt 干净。

### 读取器：`compiler/parser.py` 的 `parse`，第二步

`parse(text, name)` 是模块里唯一的入口，CLI、`vm.eval_string` 和测试都经过它。它的逻辑在 `Parser` 类的 20 个方法里，生成的代码每读一个字符都要经过几次 Python 方法调用

```rust
// 生成的 Parser.parse_expr 里的嵌套函数
if rt::call!(slf.getattr("peak")?, idx = 1)?.eq("t")? {
    rt::call!(slf.getattr("pop")?, n = 2)?;
    return Ok(rt::bool(py, true));
}
```

改写把整个读取器写成 `src/skime/compiler/parser/native.rs` 里的纯 Rust（不用 PyO3，500 行，其中测试 80 行）。结果是一个 Rust 的 `Datum` 树，`parse` 适配层把它转成 skime 的 Python 对象（`Pair`、`Symbol`、`Character`、`Vector`，从模块的 globals 取，名字是 import 时的别名 `pair`、`sym`）

```rust
pub fn parse<'py>(
    m: &rt::Module<'py>,
    text: &Bound<'py, PyAny>,
    name: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "parse", 8, |_| {
        let datum = native::parse(&text.extract::<String>()?).map_err(|err| match err {
            native::Error::Empty => parse_error(m, "Nothing to be parsed.".into()),
            native::Error::Syntax { line, message } => match name.str() {
                Ok(name) => parse_error(m, format!("{name}:{line} {message}")),
                Err(err) => err,
            },
            native::Error::NotAFloat(text) => {
                PyValueError::new_err(format!("could not convert string to float: '{text}'"))
            }
        })?;
        Objects::new(m)?.build(&datum)
    })
}
```

```rust
// native.rs
pub enum Datum {
    /// The empty list, `None` in Python.
    Nil,
    Bool(bool),
    /// The digits of an integer, with a leading `-` if it is negative. Python
    /// integers have no size limit, so the adapter converts them with `int()`.
    Int(String),
    Float(f64),
    Complex(f64, f64),
    Str(String),
    Char(char),
    Symbol(String),
    List(Vec<Datum>, Box<Datum>),
    Vector(Vec<Datum>),
}
```

原生代码按 Python 的递归下降逐个方法对应（位置按字符计数，和 Python `str` 的下标一致），错误照 §15 的约定用 `Result` 表示，适配层抛出和原来一样的 `ParseError`（文字也一样）。用一个差分脚本在 CPython 和改写后的扩展模块上解析 3000 多个随机和手写的输入，比较结果的结构和异常，只有下面两类有意的差异

- 数字只认 ASCII。Python 的 `str.isdigit` 还接受 `٣`（读成 3）和 `²`（随后 `int()` 抛 `ValueError`，REPL 只接 skime 的 `Error`，于是整个 REPL 崩溃）。改写后它们是符号
- 不认识的 `#` 语法报 `ParseError: Unknown syntax '#x'`。Python 版本在顶层报"Expecting end of code"，在列表里（`(#x)`）死循环

另外整数超出 float 范围时 `1e400/2` 这样的除法得到 `inf` 而不是 `OverflowError`，写在 `as_float` 的注释里。

`Parser` 类没有改，仍是生成的代码，留给直接使用它的 Python 代码。

### VM 的主循环：`insns.py` 的 `run`，第一步

生成的主循环

```rust
pub fn run<'py>(m: &rt::Module<'py>, ctx: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "run", 277, |_| {
        let mut ctx = ctx.clone();
        let mut opcode: Bound<'py, PyAny>;
        let mut nctx: Bound<'py, PyAny>;
        while ctx.getattr("ip")?.lt(ctx.getattr("bytecode")?.len()?)? {
            opcode = ctx.getattr("bytecode")?.get_item(ctx.getattr("ip")?)?;
            nctx = m.global("INSN_ACTION")?.get_item(&opcode)?.call1((&ctx,))?;
            if has_tag(m, &opcode, &m.global("TAG_CTX_SWITCH")?)?.is_truthy()? {
                ctx = nctx.clone();
            }
            m.py().check_signals()?;
        }
        ctx.call_method0("pop")
    })
}
```

改写后指令是 `src/skime/insns/native.rs` 里的 Rust 枚举 `Insn`（25 个指令和 `TAG_CTX_SWITCH`），每条指令直接调用同一个文件里生成的 `op_*` 适配层，不再经过 `INSN_ACTION` 列表和 Python 的函数调用。上下文对象和字节码列表仍是 Python 对象

```rust
let mut ctx = ctx.clone();
loop {
    let bytecode = ctx.getattr("bytecode")?;
    let ip: usize = ctx.getattr("ip")?.extract()?;
    if ip >= bytecode.len()? {
        break;
    }
    let opcode: usize = bytecode.get_item(ip)?.extract()?;
    let Some(insn) = Insn::from_opcode(opcode) else {
        // `INSN_ACTION[opcode]`
        return Err(PyIndexError::new_err("list index out of range"));
    };
    let nctx = execute(m, insn, &ctx)?;
    if insn.switches_context() {
        ctx = nctx;
    }
    m.py().check_signals()?;
}
pop(&ctx)
```

同时把 `op_*` 里对 `Context.push`、`pop`、`top` 的方法调用内联成对 `ctx.stack` 这个 Python list 的操作（`push(ctx, &loc)?` 代替 `ctx.call_method1("push", (&loc,))?`），空栈时抛和 `list.pop` 一样的 `IndexError`。代价是 `Context` 的子类覆盖这些方法时，指令不会调用它们，写在注释里。

### 改写的收益

| fib 22（Scheme） | 时间 |
| --- | --- |
| CPython | 0.52 s |
| 生成的代码，release | 3.0 s |
| 改写两处之后，release | 2.15 s |
| 生成的代码，只把属性名换成 `intern!`（实验） | 1.64 s |

主循环的改写少了 30% 的时间，但改写的部分不是瓶颈，见下面的"性能"。

## 6. 改写时的阻碍

按遇到的顺序

1. **ext 和 bin 是两个 crate。** 要用 skime 自己的 pytest 测改写后的代码，只能把改过的模块文件手工复制进另外生成的 ext crate（写了一个脚本按 `git diff` 复制）。两个 crate 的模块文件完全相同，只差入口。M12 的 `steelix test` 直接用 bin 跑测试，解决了这个问题
2. **逻辑在类里，入口是函数。** `parse` 只有一行，真正的逻辑在 `Parser` 的方法里。§15 讲了函数怎么改、类的方法怎么改，没有讲"用一个原生函数取代整个类的用途"。这里选择改写 `parse`、保留类，`Parser` 的生成代码于是不再被 skime 自己使用
3. **适配层要构造别的模块的类的实例。** 用 `m.global("pair")`，名字是这个模块 import 时的别名（`from ..types.pair import Pair as pair`），要读 `exec_module` 才知道。§15 没有说，但照着生成代码写就对了
4. **抛项目自己的异常类没有现成的写法。** 生成代码用 `frame.raise(m.global("ParseError")?.call1((..,))?)`，适配层在 `map_err` 里没有 `frame`，要自己 `PyErr::from_value`，查找失败时还要处理第二个错误。一个 `rt` 的小工具（按全局名字构造异常）会更干净
5. **跨类的方法调用。** §15 说 `self.method()` 总是动态调用，但 `insns.py` 调用的是另一个类（`Context`）的方法。内联它们改变了"子类覆盖"的语义，§15 没有提到这种情况，需要自己决定并写明
6. **实例状态共享太广，第二步做不了。** `Context` 的属性（`ip`、`stack`、`env`、`parent`、`bytecode`）被 `insns.py`、`invoke.py`、`vm.py`、`call_cc.py`、`form.py` 读写。照 §15 第二步要等这些模块的所有读写方都改写才能换成 `#[pyclass]`，等于改写整个 VM。§15 的"生产方和所有读取方"在真实项目里可能横跨五个模块，需要工具列出"谁读写了这个类的哪些属性"
7. **Python 源码本身是生成的。** `insns.py`、`iset.py` 由 `iset_gen.py` 从 `iset.yml` 生成，改写后 Rust 的 `Insn` 枚举是第二份指令表，要手工保持同步。§15 没有讨论这类情况
8. **生成代码里的求值顺序痕迹。** `ctx.push(ctx.top())` 生成 `ctx.getattr("push")?.call1((ctx.call_method0("top")?,))?`，而 `ctx.push(x)` 是 `ctx.call_method1("push", (&x,))?`。前者是因为 Python 先取属性再求值参数，写法正确但第一次看会疑惑
9. **模块常量是全局查找。** `INSN_TAGS` 生成 25 行 `&m.global("TAG_CTX_SWITCH")?.bitor(m.global("TAG_CTRL_FLOW")?)?`。这是正确的（Python 里可以重新赋值），改写时直接换成 Rust 常量
10. **Python 和 Rust 的字符分类不同。** `str.isdigit`、`isspace`、`isalpha` 和 `char` 的方法在非 ASCII 字符上不一致（`isspace` 多了 U+001C 到 U+001F），要逐个决定。和 logsum 的 `int()` 一样，§15 的"转换要与原来的 Python 语义对齐"说得对，但每次都要重新查

没有遇到的：改写没有动到任何无关的生成代码，`main.rs` 和模块表不用改，调用方不用改，`rt::frame!` 保留两行包装就够了，`cargo test` 能直接测 `native.rs`（bin crate 也可以）。

## 7. 性能

性能不是这个里程碑的目标，但差距大到影响部署的判断。skime 的 VM 每执行一条指令要调用 5 到 10 个很小的 Python 函数，fib 22 在 CPython 里一共有 630 万次 Python 函数调用（`sys.setprofile` 统计，最多的是 `get_param`、`has_tag`、`Context.push`）。CPython 每次调用约 80 ns（含函数体），原生代码约 400 ns。

在扩展模块上测单个调用（`timeit`）

| | CPython | 原生 |
| --- | --- | --- |
| `ctx.top()` | 43 ns | 206 ns |
| `has_tag(1, 2)`（模块函数） | 40 ns | 143 ns |
| `Context(None, env, vm=vm)` | 256 ns | 992 ns |
| `VM()` | 126 us | 521 us |

两个原因

- **Python 调用原生函数的开销。** `Function` 只有 `__call__`（tp_call，每次构造参数元组和 PyO3 的参数解析，再按签名绑定到 `Vec`），没有 vectorcall，也没有 `Py_TPFLAGS_METHOD_DESCRIPTOR`，所以 `obj.method()` 每次都创建一个 bound method 对象。`ctx.top()` 的函数体很小，多出来的 160 ns 都在调用上
- **属性名没有 intern。** `ctx.getattr("ip")` 每次从 `&str` 新建一个 Python 字符串，再算哈希。把生成代码里所有字面量属性名（`getattr`、`setattr`、`call_method*`）换成 `pyo3::intern!` 之后，同一个程序从 3.0 s 降到 1.64 s，不用改别的。模块全局变量已经是 intern 的（`rt::interned`）

改写只能消除被改写部分内部的调用，Python 调用原生函数的开销留在所有还没改写的地方，所以要先从 runtime 和生成方式上解决。

### M11 之后

M11 按上面的两个原因做了几项低成本的改进（design §6 "调用的开销"）：`Function` 支持 vectorcall 和 `Py_TPFLAGS_METHOD_DESCRIPTOR`，只有位置参数的调用不做绑定，生成代码的属性名和方法名经过 `rt::Attributes` 使用驻留的字符串。

| skime | CPython | M10 | M11 |
| --- | --- | --- | --- |
| fib 22（release） | 0.48 s | 2.93 s | 1.10 s |
| 自己的 108 个测试（ext 模式，release） | 0.16 s | 0.40 s | 0.24 s |

fib 22 分步看：只换 runtime（vectorcall、method descriptor、快速路径，生成代码不变）是 2.11 s，再加上驻留的属性名是 1.10 s。去掉 method descriptor 是 1.20 s。

单次调用（`timeit`，扩展模块，release，`tests/cases` 风格的小模块，"循环"是原生函数里的 `while` 循环，每轮的时间包括循环本身）

| | CPython | M10 | M11 |
| --- | --- | --- | --- |
| Python 调用 `f(1, 2)` | 21 ns | 63 ns | 31 ns |
| Python 调用 `g(1)`（有 keyword-only 参数，走完整的绑定） | 27 ns | 86 ns | 78 ns |
| Python 调用 `c.m()` | 24 ns | 107 ns | 50 ns |
| Python 调用 `c.m2(1)`（用默认值补参数） | 26 ns | 107 ns | 56 ns |
| Python 调用 `C(1)`（`__init__` 是原生的） | 65 ns | 171 ns | 95 ns |
| 空循环 | 14 ns | 28 ns | 28 ns |
| 循环里 `c.m()`（原生到原生，经过属性） | 34 ns | 145 ns | 74 ns |
| 循环里 `o.f(i, i)`（`o` 是 `SimpleNamespace`） | 49 ns | 112 ns | 66 ns |
| 循环里 `C(i)` | 76 ns | 222 ns | 135 ns |
| 循环里直接调用 `f(i, i)` | 34 ns | 29 ns | 29 ns |

剩下的差距主要在 CPython 的特化（`LOAD_ATTR`、`LOAD_GLOBAL` 的 inline cache，`CALL_ALLOC_AND_ENTER_INIT`），原生代码每次都走通用的 `PyObject_GetAttr` 和 dict 查找。另外 `ctx.push(ctx.top())` 这样参数不纯的方法调用仍然先创建 bound method（design §6）。

## 8. 发现的问题

### steelix（已修复）

- **复数字面量。** `1j` 以前不支持，整个 `Parser.parse_number` 保留为 Python。现在生成 `rt::complex(py, 0.0, 1.0)`（runtime 的新函数，和 `rt::float` 对应），用例 `tests/cases/complex_constants.py`。`tests/test_lower.py` 里以前拿 `1j` 当"不支持的构造"的例子换成了 t-string 和多个 `for` 的生成器表达式
- **clippy `collapsible_if`。** Python 里没有 `else` 的 `if` 套 `if` 生成嵌套的 `if`，clippy 1.95 要求合并（`compiler.py` 两处、`prim.py` 一处）。保留 Python 的结构，在 crate 根上 allow，用例 `tests/cases/nested_if.py`
- **clippy `module_inception`。** `skime/compiler/compiler.py` 生成 `compiler/mod.rs` 里的 `pub mod compiler;`。模块名就是 Python 的文件名，同样在 crate 根上 allow，由 `tests/projects/skime` 覆盖

### steelix（没有修复，记录）

- `type(f).__module__` 是 getset descriptor（§12）
- 分发复制了包里只在开发时用的文件（`iset_gen.py`、`iset.yml`）
- ext 模式下 `python -m skime` 仍然不行（M6 遗留）。M12 起安装成 `skime/__init__.so` 时可以

### skime 自身（CPython 下同样出现，不是 steelix 的问题，供参考）

- `(define (q n) (if (= n 0) 0 (let () (q (- n 1)))))` 之后 `(q 1)` 抛出未捕获的 `IndexError: pop from empty list`（`if` 分支里 `let` 体内的尾调用），REPL 退出
- `(#x)` 让读取器死循环，顶层的 `#x` 报的是"Expecting end of code"
- `²` 这样 `isdigit` 为真但 `int()` 不接受的字符抛出 `ValueError`，REPL 退出
- 同一个 `begin` 或 REPL 里，顶层 `define` 不能引用后面才定义的函数（`Unbound variable`），互相递归的顶层函数写不出来

## 9. 测试

- `tests/projects/skime/`：skime 包的原样副本（加上 `LICENSE`），驱动脚本 `app.py`（把 `programs/*.scm` 喂给 REPL 的 `main()`，或者用嵌入 API `vm.load`、`vm.eval_string`，REPL 输出里的对象地址换成固定文字），5 个 Scheme 程序。`tests/test_projects.py` 在 7 组参数下和 CPython 比较 stdout、退出码和完整 traceback，并要求两种输出编译没有警告、clippy 干净
- 生成的 `src/` 有 1.5 万行，比现有全部 golden（单文件用例 1.8 万行，项目 7860 行）的一半还多，所以 skime 不做 golden（`Case.golden = False`）。行为由和 CPython 的比较保证，编译和 clippy 由构建保证。如果以后想锁定输出，可以只对几个有代表性的模块（`insns.rs`、`ctx.rs`、`compiler/parser.rs`）做 golden
- 加上 skime 之后 `tests/test_projects.py` 的 26 个用例共约 90 s

改写后的 crate 在 `scratchpad/m10/rewrite`（相对于生成结果的 `git diff`），暂不放进仓库。

## 10. 建议（按优先级）

1. **降低调用原生函数的开销（性能里程碑，M11 已做，见第 7 节）。** `Function` 实现 vectorcall（PEP 590），加 `Py_TPFLAGS_METHOD_DESCRIPTOR` 让 `obj.method()` 不创建 bound method，参数绑定在只有位置参数时走快速路径；生成代码里的字面量属性名用 intern 的字符串（`pyo3::intern!` 或 runtime 的缓存，要兼顾可读性，例如 `rt` 提供带缓存的 `attr`/`set_attr`/`call_method`）。目标是像 skime 这样的程序不再比 CPython 慢几倍，单是 intern 就有 1.8 倍
2. **一个 crate 同时出 bin 和 ext（M12 换了做法，用 bin 的解释器角色跑测试，见第 3 节）。** 生成的 crate 同时有 `[lib]`（cdylib）和 `[[bin]]`，或者 `steelix` 提供把一个 crate 的模块文件用于另一种模式的命令，让项目自己的测试套件能直接测改写后的代码。顺带写一份"用项目自己的 pytest 测原生代码"的做法（这次最有用的验证手段），并修好 ext 模式的 `python -m pkg`
3. **补充 §15。** 加上这次遇到的几种情况：用原生函数取代一个类的用途（保留还是删除类）、适配层构造别的模块的类的实例（按 import 的别名取 globals）、抛项目自己的异常类（并在 runtime 里加一个小工具）、内联别的类的方法时的语义变化、Python 源码本身是生成的时候怎么保持单一来源
4. **为第二步提供"属性读写索引"。** 静态列出项目里每个类的每个实例属性被哪些函数读写（`ctx.ip` 这类），作为 `steelix coverage` 或单独命令的输出。改写 `Context` 这样被五个模块共享的类之前，没有这张表只能靠 grep
5. **generator。** skime 里剩下的唯一一个保留函数，和 `coverage.md` 的结论一致
6. **分发的小问题。** 复制包目录时可以排除开发用的文件（`--exclude-data` 或按 `pyproject.toml` 的打包配置），`type(f).__module__` 返回字符串
