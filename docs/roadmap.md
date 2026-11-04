# Roadmap

每个里程碑结束时都应该有能跑通的端到端用例。任务粒度大约是一次会话能完成的量，完成后勾选并在 `tests/cases/` 留下对应用例。设计依据见 `design.md`，括号里是对应章节。

接下来的执行顺序（根据 `coverage.md` 的数据和 M3c 之后的 review）：M3d ✅ → M4 ✅ → M5 ✅ → M8 ✅ → M8a ✅ → M6 ✅ → M9a ✅ → M7 ✅ → M7a ✅ → M9 ✅ → M10 ✅ → M11 ✅ → M11a ✅ → M11b ✅ → M12 ✅ → M13 ✅ → M14 ✅。

## M0 项目骨架 ✅

- [x] uv 初始化，Python 3.14.9，dev 依赖 pytest、ruff
- [x] Rust workspace 与 `steelix-runtime` crate（pyo3 0.30）
- [x] 目录规划、设计文档、任务拆解
- [x] 验证 PyO3 嵌入 libpython 可行
- [x] 设计 review，结论已合入 design.md

## M1 最小闭环：fib 跑通 ✅

目标是 `tests/cases/fib.py` 能转换、编译成嵌入 libpython 的可执行程序，输出与 `python fib.py` 一致。原计划只做 ext 模式，后来改成以 app 场景为主，bin 模式提前到 M1 完成，ext 模式保留为 `--mode ext`。

- [x] `rustast`：最小节点集合（fn、let、赋值、if、while、return、表达式语句、调用、方法调用、字面量、`?`）和打印器，rustfmt 必须可用，带单元测试
- [x] `frontend`：`parse.py`、`scope.py`（symtable），`bindings.py` 统计模块级名字绑定次数、检测 `import *`/`globals()`
- [x] 变量模型的最小版本：局部变量在函数开头声明，定值分析，不能确定时用 `Option`（§5）
- [x] 名字卫生：重命名表、关键字 `r#`、字面量规则（§4、§12）
- [x] runtime：`Function` 的最小版本（仅位置参数、`__call__`、`__get__`、可写的元信息）、`rt::def!`、`RecursionGuard`、`rt::int` 等字面量辅助（§6）
- [x] `lower`：常量、名字、赋值、二元运算、比较、`if`、`while`、`def`（仅位置参数）、`return`、函数调用、`print`
- [x] 调用图分析，递归函数加 `RecursionGuard`
- [x] `emit`：生成 crate（Cargo.toml、lib.rs），runtime 用 path 依赖
- [x] CLI `steelix transpile foo.py -o out/` 串起来
- [x] 测试框架：golden 快照与 `--update-golden`、`cargo check` 层、e2e 差分测试（显式设置 `PYO3_PYTHON`，共享 target 目录，pytest marker `e2e`）
- [x] bin 模式：`PyConfig` 初始化、以 `__main__` 执行、`Py_FinalizeEx`、退出码、rpath（§11）

## M2 核心语句与表达式 ✅

- [x] 按 Python 求值顺序生成临时变量，赋值先求值右边（§8）
- [x] `for` + `try_iter`（赋值给已声明的变量，只在循环体内使用的循环变量在每次迭代开头 `let`），`range` 特化留到后面
- [x] `break`/`continue`、循环 `else`（带标签 block）
- [x] 增量赋值，runtime 的 `rt::iadd` 等，目标只求值一次（§8）
- [x] 一元运算、`and`/`or` 值语义、条件表达式、`in`/`not in`/`is`/`is not`
- [x] 比较的值语义（`rich_compare`）与链式比较
- [x] list/tuple/dict/set 字面量、下标、切片、属性读写、`del`
- [x] 解包赋值 `a, b = ...`、`a, *rest = ...`
- [x] f-string（`format` + 拼接）
- [x] `global`、模块全局变量读写、`pass`、`assert`
- [x] 常用 builtins 映射（`len`、`isinstance`、`str`、`repr`、`bool`、`abs`、`type`、`iter`），其余走 `builtins` 动态查找。`int`、`range` 没有逐字等价的 PyO3 写法，暂不特化
- [x] 简单的 `import x`、`import x.y as z`、`from x import y`（标准库与第三方库）
- [x] bin 模式贴近 `python app.py`：`sys.argv[0]` 用脚本名，`sys.path[0]` 为脚本所在目录，设置 `__file__`

容器字面量里的 `*x`/`**d` 展开、`AnnAssign` 仍然报 `UnsupportedError`，分别留给 M8 和 M3。

## M3 函数与闭包 ✅

- [x] `Function` 完整签名绑定：默认参数（`def` 时求值，存 `__defaults__`）、关键字参数、`*args`、`**kwargs`、keyword-only、positional-only，`TypeError` 文字与 CPython 一致（§6）
- [x] 直接调用支持关键字参数和默认参数，省略的参数在调用时从函数对象读取（§6）
- [x] 调用侧的关键字参数、`*`/`**` 展开，按 CPython 的顺序求值（`rt::call!`，§8）
- [x] 嵌套函数、闭包、`nonlocal`（会改变的变量用 `rt::Cell` 作为参数传入，其余按值传入）
- [x] `lambda`
- [x] 装饰器（按 Python 语义执行，被装饰的名字退化为动态调用），验证 `functools.wraps`、`functools.lru_cache`
- [x] PEP 649 注解（保存源码文本，惰性求值，不影响生成的类型）
- [x] `__signature__`、`__reduce__` 等内省支持，`inspect.signature` 和 pickle 可用
- [x] docstring 转 `///`，注释保留，源位置注释
- [x] 函数内的带注解赋值（注解不求值，等同普通赋值）

模块级的带注解赋值（需要模块的 `__annotate__`，与类一起在 M5 做）、模块级的条件定义和重复定义、类型参数（`def f[T]()`）仍然报 `UnsupportedError`。

## M3a 修复与生成代码检查 ✅

来自 M2 之后的独立评估，以及 M3 验收时发现的问题。

- [x] 字面量容器（`PyDict`/`PyList`/`PyTuple`/`PySet`）作为 `get_item`、`set_item`、`del_item`、`len`、`contains` 等的接收者时先擦除成 `PyAny`。现在 `{"a": 1}[key]` 在 key 不存在时返回 None 而不是抛 KeyError，`len([1, 2])`、`[1, 2, 3][-1]` 编译失败。`Value` 的 `typed` 应该带上具体类型
- [x] 条件里的链式比较，中间操作数需要临时变量时（`if a < b < f() < d:`），后续比较要放进同一个嵌套 block，否则临时变量超出作用域
- [x] 循环 `else` 里对外层循环的 `break`/`continue` 需要带标签（rustc E0695）。循环节点加可选标签，`Continue` 加标签字段，维护循环栈
- [x] bin 模式下未捕获异常时，先 flush `sys.stdout` 再打印 traceback，与 CPython 的输出顺序一致
- [x] 生成代码检查器：在 Rust AST 上检查路径引用是否在作用域内、跳转标签是否正确、带类型的接收者是否调用了被遮蔽的方法，对所有 golden 运行
- [x] 一批"真实风格"的脚本作为语料（`tests/corpus/`），要求能转换、编译没有警告，输出与 CPython 一致。发现并修复了 `if`/`while`/`for` 头部的 lambda 的 fn item 被放进分支或循环体的问题
- [x] 生成的 crate 在 dev profile 下提高 `opt-level`，文档说明函数改写前不会比 CPython 快

## M3b 改写边界 ✅

验证迁移路线的第 3 步：人把某个生成的函数改写成普通 Rust，程序其余部分照常工作。

- [x] 把面向 Python 的适配层生成为可见、可编辑的函数，不再藏在 `rt::def!`/`rt::function!` 宏里（生成的函数本身就是适配层，宏只负责注册和把绑定好的参数传给它，§15）
- [x] 把 `py` 和 `globals` 合并成一个上下文参数 `m: &rt::Module<'py>`，同步更新所有 golden
- [x] 只绑定一次的 import 名字和常量，考虑不再每次用字符串查 globals（结论是不缓存值，只缓存驻留的名字字符串，§7）
- [x] 直接调用时不可变字面量的默认值直接写在调用处
- [x] `examples/` 下放一个小应用：转换后手工把其中一个函数改写成地道的 Rust（原生参数和返回类型），e2e 测试证明输出仍与 CPython 一致
- [x] 把改写约定写进 design.md（适配层、`extract`/`into_pyobject` 放在哪里、直接调用方如何调整）

## M3c 混合模式与真实应用 ✅

开头先处理 M3b 之后 Fable review 发现的问题。

- [x] 直接调用的目标被替换成另一个没有对应参数的函数时，`Function::default` 会 panic，改成抛 `TypeError`（CPython 会调用替换后的函数，直接调用做不到，§12）
- [x] `run_main` 兜住 panic，转成 traceback 并以退出码 1 结束，而不是 Rust 的 101
- [x] 不带参数的 `dir()`、`vars()`、`locals()` 现在报 `SystemError: frame does not exist`，按 `globals()` 的方式处理（模块级；函数里的保留为 Python）
- [x] logsum 示例：README 补全行为差异（负数、下划线、非 ASCII 数字也会被当成格式错误），`access.log` 加覆盖这些情况的行，测试明确检查改写版本的输出与 CPython 的差异
- [x] §15 改成两步走：先改逻辑但数据仍用 Python 容器（`cast::<PyDict>()` 等，没有拷贝），等某个容器的生产方和所有读取方都改写后，再整体换成 Rust 类型。logsum 的 `tally` 只做第一步作为对照。同时说明类的方法要以整个类为单位改写
- [x] §7 的措辞：保持全局变量动态查找是因为不影响可读性，而不是因为能完整支持 monkeypatch（直接调用本来就不支持）
- [x] 混合模式：不支持的函数和模块级语句保留 Python 源码，用 `rt::exec_python!` 在模块的 globals 里就地执行，保留原来的文件名和行号，默认开启，`--no-fallback` 恢复报错（§13）
- [x] 转换覆盖率统计：`steelix transpile` 打印摘要，`--report` 写 JSON，`steelix coverage` 合并多个模块（§13）
- [x] 真实应用：标准库的 6 个命令行工具（calendar、base64、tabnanny、tokenize、ast、json.tool）在混合模式下与 CPython 一致，放进 `tests/apps/`。修复了发现的 3 个编译问题，数据和建议的 M4、M5、M8 顺序见 `coverage.md`

## M3d 混合模式之后的小修 ✅

来自 M3c 之后的 Fable review，都有复现。

- [x] 模块级嵌套语句的细粒度保留：`if`/`for`/`while`/`if __name__ == "__main__"` 里只有某个子语句不支持时，只保留那个子语句，其余仍然原生（模块作用域就是 globals，不需要同步局部变量；注意被保留的子语句里的 `break`/`continue`）
- [x] 原生的 `while`/`for` 循环里调用 `PyErr_CheckSignals`，现在纯原生循环按 Ctrl-C 停不下来
- [x] 每个被保留的 `def`/`class` 单独一个 `exec_python!`，各自带源位置和原因，只合并连续的简单语句。§13/§15 补充如何把保留为 Python 的函数手工改成原生
- [x] 对自身模块的 `setattr`/`delattr`（`sys.modules[__name__]`、`__main__` 等）关闭直接调用，与 design.md §7 的描述一致。`exec` 同理
- [x] `rt::builtin` 改为查 builtins 的字典，而不是对 builtins 模块 `getattr`（现在 `__annotations__` 等名字会读到模块对象的属性）
- [x] 模块注解分散在多个保留块时合并，而不是只保留最后一块
- [x] 带参数的 `vars(obj)` 不再关闭整个模块的直接调用
- [x] §12 记录原生函数没有 Python 帧对 `logging`（`%(lineno)d`、`%(funcName)s`）和 `warnings` 的影响，`tests/apps` 加一个记录行号的用例（放在了 `tests/cases/frames_logging.py`，期望的差异写在 `tests/test_e2e.py` 的 `KNOWN_DIFFERENCES`）

另外修复了两个顺带发现的问题：原生代码里只有一个参数的 `exec(src)`/`eval(src)` 没有帧可用，报 `SystemError`，现在在模块级显式传入 globals，函数里的保留为 Python；模块级不会结束的原生循环后面不再生成 `Ok(())`（unreachable 警告）。bin 模式下未捕获的 `KeyboardInterrupt` 和 CPython 一样以 `SIGINT` 结束进程。

## M4 异常与上下文管理 ✅

- [x] `raise`、`raise ... from ...`、`from None`、裸 `raise`（包括 handler 调用的辅助函数里的），`__cause__`、`__context__`、`__suppress_context__`、`__traceback__` 与 CPython 一致
- [x] `try/except/else/finally`，`Flow` 枚举处理块内的 return/break/continue（§9）。只有一个表达式的 `try` 直接 `match` 结果，不用闭包；`finally` 覆盖 `return`、吞掉异常与 CPython 一致
- [x] 进入 handler 时设置当前处理异常的 RAII guard（`rt::Handling`），用 `logging.exception`、`traceback.print_exc()`、嵌套 handler 验证
- [x] `except (A, B) as e`（结尾隐式 `del e`）、裸 `except`。`except*` 保留为 Python
- [x] `with`（多个 item、元组目标、`__enter__`/`__exit__` 在类型上查找、吞掉异常、跳出 `with`）。`async with` 保留为 Python
- [x] 原生函数在 traceback 里的合成帧（类似 Cython：异常离开时用 `PyFrame_New` 和 `types.TracebackType` 补上帧，语句前的 `frame.line(N)` 记录行号），替代原来计划的 `add_note`。logging、warnings 看到的仍是最近的 Python 帧，没有公开 API 可以在运行期间压入解释器帧，记录在 §12
- [x] e2e 测试比较预期异常的类型和消息，并且比较完整的 traceback（去掉原生帧没有的 `^` 行）；`tests/test_apps.py` 比较每一帧的位置
- [x] 改写约定补充：改写后的适配层抛出的异常类型必须与原来一致。`examples/logsum` 的 `parse_line` 把原生的 `Err(NotANumber)` 转成 `ValueError`，生成的 `load` 用 `except ValueError` 接住

覆盖率的变化见 `coverage.md`：6 个工具的原生函数从 23% 到 37%，标准库从 14% 到 22%，和 M3c 时的预测一致。顺带把循环里的 `check_signals` 从迭代开头移到结尾，`KeyboardInterrupt` 的行号和 CPython 一样。

## M5 类 ✅

- [x] 动态建类，语义和 `__build_class__` 完全一样（metaclass、`__prepare__`、`__mro_entries__`、`__set_name__`、`__init_subclass__`、`__classcell__`、类体前后编译器生成的 `__module__`、`__qualname__`、`__firstlineno__`、`__static_attributes__`），类体里任意语句都是原生代码，装饰器、私有名字改编。每个类一个 Rust mod，class 语句是一个 `rt::class!` 调用（§10）
- [x] 方法级的混合模式：类原生创建，用了不支持语法的方法和类体语句保留为 Python 源码，按类体编译后在类的命名空间里执行，`super()`、`__class__`、私有名字、`__qualname__` 和行号都和 CPython 一样（§13）。整个类保留的情况只有头部不支持、类型参数、类体里的 `global`/`nonlocal` 和要保留的注解
- [x] 方法、`__init__`、实例属性（`slf.getattr`/`setattr`）、类属性
- [x] 继承（Python 定义的类、保留为 Python 的类、Exception、Enum、NamedTuple、TypedDict、`abc.ABC`、`threading.Thread`、`argparse.Action`，多重继承）
- [x] 零参数 `super()` 和 `__class__` 用真实的 `__class__` cell（不是原计划的 `super(Cls, self)`），类装饰器替换类时也和 CPython 一样
- [x] `staticmethod`、`classmethod`、`property` 按普通装饰器处理
- [x] 自定义异常类，在原生代码和保留的代码之间抛出、接住
- [x] PEP 649 注解（`__annotate__`、`__annotations__`、`typing.get_type_hints`），dataclass（含 `frozen=True`、`order=True`），`from __future__ import annotations`
- [x] 改写约定补充类的两步（§15）
- 函数里的 `class` 还不支持（所在的函数保留为 Python，标准库里 9 个函数）

覆盖率的变化见 `coverage.md`：6 个工具的原生函数从 37% 到 73%，标准库从 22% 到 90%，和 M3c 时按方法级混合模式的预测一致（71%、88%）。标准库里只有 3 个带类型参数的类整个保留为 Python。顺带修了 `rt::call!` 在被调用的是引用（`rt::local(..)?` 返回的 `&Bound`）时编译失败的问题（`tests/cases/call_ref_callee.py`）。

## M8a M5/M8 之后的修复 ✅

来自 M5 之后的 Fable review（前两条已复现）和 M8 的遗留。

- [x] body 不会失败的 `try`（如 `try: x = 1`）生成的 `match Ok(..)` 编译失败（E0282）：body 不会失败时只生成 body，handler 不可达；其余 `Ok(..)` scrutinee 写成 `Ok::<_, PyErr>(..)`（§5、§9，`try_infallible`）
- [x] 为求值顺序提前计算的临时变量会一直活到所在 Rust 块结束（模块级就是整个程序），`__del__`、weakref、`finalize` 的时机与 CPython 不同：最后一次使用时 move，仍然活着的给语句加块作用域（§8 `lower/temps.py`，`temp_lifetimes`）
- [x] `except <非异常类>` 抛出的 TypeError 缺少 `__context__`，行号是 `try` 行而不是 `except` 行：改成 `frame.matches(&err, LINE, || handler)`（§9，`except_bad_type`）
- [x] 给 M4 的分析补单元测试：`frontend/regions.py`、`lower/rustflow.py`、`_DefiniteAssignment` 里 handler/with/region 相关部分（`tests/test_frontend.py`、`tests/test_rustflow.py`，后者也测 `lower/temps.py`）
- [x] `bindings.py` 中六处"节点绑定了哪些名字"的实现合并成一个（`_bound_names`），顺带去掉重复的 `_COMPOUND`、break/continue 查找、`BOUND` 和没用的 `class_key`
- [x] 容器字面量里的展开（`[*a]`、`{**d}`）：runtime 的 `rt::list!`/`tuple!`/`set!`/`dict!`（§8，`displays_unpacking`）。标准库被它挡住的 25 个函数都变成原生的，函数覆盖率 96.4% → 96.7%，行 93.3% → 94.2%
- [x] walrus 局部变量第一次在顶层 `if` 条件里绑定时 clippy 报 `needless_late_init`：在那里 `let`（§5）。cargo 测试层对所有 golden 跑 clippy（§14）
- [x] §15 类改写补充：类属性用 `#[classattr]`，没有可变状态的类用 `frozen`，`native.rs` 放在 `src/<module>/` 下，元组之类的属性要明确 Python 侧的形状

## M6 多模块应用 ✅

- [x] 输入为入口脚本（或 `-m 包`）时，静态找出它 import 的项目内模块和包，连同入口生成一个 crate，每个 Python 模块一个 Rust mod，按包嵌套（`src/pkg/mod.rs`、`src/pkg/sub.rs`），名字不是 Rust 标识符时改名（§7）
- [x] runtime 里的 meta path finder 和每个模块一个 loader，子模块在第一次 import 时执行；`__file__`、`__spec__`、`__path__`、`__cached__` 等和从源码 import 一样，import 出错的 traceback 去掉 importlib 的帧（§7）
- [x] 项目内模块之间的引用、相对 import；`from x import y` 失败时的文字和 CPython 一样（循环 import、遮蔽标准库）
- [x] 入口模块被按名字 import 时是第二个模块对象；`python -m pkg`（`pkg/__main__.py`）的等价运行方式（§11）
- [x] `--include`（动态 import 的模块、目录）和 `--exclude`，没有转换的项目模块照常从源码 import，报告里列出原因
- [x] 跨模块的调用经过模块属性，不做跨模块的直接调用；识别项目内别的模块对函数的替换，被替换的函数不直接调用（§7）
- [x] 覆盖率报告按模块加项目合计（§13）
- [x] ext 模式：有项目模块时安装 finder；`--mode ext -m pkg` 生成包形式的扩展模块（§11）
- [x] 测试：多模块测试应用 `tests/projects/inventory`、标准库 `json` 包（也以 `python -m json` 运行），golden、clippy、与 CPython 比较完整 traceback 的 e2e
- [x] 发现的问题：Python 参数 `_m` 和上下文参数 `m` 触发 clippy 的 `duplicate_underscore_argument`，这时上下文参数改叫 `module`；类的 mod 和所在模块同名（`module_inception`）时加 `_`；crate 根上 allow `too_many_arguments` 和 `doc_lazy_continuation`（§14）
- [x] ext 模式下 `python -m pkg`：M7a 给 `NativeLoader` 加了 `get_code`，剩下的问题是扩展模块 `pkg` 的 spec 不是包，runpy 不会去找 `pkg.__main__`。M12 用 `pkg/__init__.so` 的安装方式解决
- [x] 命名空间包（没有 `__init__.py` 的目录）里的模块也转换（M14 之后）：命名空间包的 Rust mod 只声明子模块，包对象仍由 `PathFinder` 创建，finder 对这些模块检查父包的 `__path__`（同名的普通包在 `sys.path` 后面时它优先），分发里保留目录（§7，`tests/projects/namespaces`）

## M7 分发 ✅

- [x] 部署后的二进制不依赖源文件：multiprocessing 的 spawn 和 forkserver 子进程启动程序本身（`sys.meta_path` 上的 hook 在 `multiprocessing.spawn` 第一次 import 时调用 `set_executable`），程序识别子进程的 `-c` 命令行，把 `_fixup_main_from_path`、`_fixup_main_from_name` 换成执行原生 `__main__` 的版本。开发构建和分发都一样（§11）
- [x] `sys.executable` 仍然是 Python 解释器：开发时是 venv 的 python，分发里是指向程序的 `python/bin/python`，程序以这个名字运行时就是 python（`Py_RunMain`），`subprocess.run([sys.executable, ...])` 照常可用
- [x] Spike：附带 python-build-standalone 加 site-packages（采用）、静态链接 libpython、系统 Python、标准库 zip，测量了 hello 和 json 项目在几种标准库取舍下的体积、文件数和启动时间，结论写入 design.md §11
- [x] 运行时按相对于二进制的路径定位 Python 安装（`PyConfig.home`，rpath `$ORIGIN/python/lib`）和项目目录，不再写死构建时路径。`main.rs` 里只有 `steelix_runtime::script!("app.py")`，开发构建的路径由 build.rs 给出，golden 不再需要替换 `$ROOT`。`sys.argv[0]` 改成和 `__file__` 一样的绝对路径
- [x] `steelix build` 一条命令转换、`cargo build --release`、组装 `dist/<name>/`（程序、Python 运行时、构建解释器的 site-packages、可选的源文件、`--add-data`），`--archive` 打成 tar.gz，`--python` 选择解释器或 venv
- [x] 在干净环境里验证：测试用 `bwrap` 只挂载 `/usr` 和分发目录运行 hello、json 项目、`-m json` 和 multiprocessing（容器的 podman 在本机不可用，`bwrap` 起到同样的隔离作用）
- 只做了 Linux x86_64，macOS 和 Windows 需要的改动记在 §11。标准库 zip 暂不做（文件数少了，但启动慢 6 到 11 ms）

## M7a M6/M7 之后的修复 ✅

来自 M6、M7 之后的 review，都有复现（`tests/test_dist.py` 里的回归测试）。

- [x] 安全：M7 的程序只看命令行就当成 multiprocessing 子进程，`app -c "from multiprocessing.spawn import spawn_main; ..."` 会执行任意代码。现在程序的参数永远交给脚本，和 `python app.py -c ...` 一样。分发里子进程由 `python/bin/python`（解释器角色，本来就执行任意代码）启动，解释器识别子进程的命令行后安装原生的 `__mp_main__`；开发构建由父进程在环境变量 `STEELIX_MP_CHILD` 里写上程序的路径，程序只在命令行和标记都对上时作为子进程运行（§11）
- [x] 分发的程序和环境隔离：忽略 `PYTHON*` 环境变量（`use_environment = 0`），不加用户的 site-packages，`STEELIX_PYTHON_ENV=1` 恢复。附带的解释器读取环境变量，但像 venv 一样不加用户的 site-packages。顺带修了解释器角色的 `-X dev`、`-X utf8`、`-E` 不影响 pre-initialization 的问题（先用命令行 `Py_PreInitializeFromBytesArgs`）
- [x] `NativeLoader` 加上 `get_resource_reader`（`importlib.resources.files(pkg)`）、`get_data`（`pkgutil.get_data`）和 `get_code`（`runpy`、`python -m pkg.mod`），ext 模式同样受益（§7）
- [x] 分发复制包目录下的所有文件（数据文件），`--no-sources` 只去掉转换过的 `.py`；项目顶层既没转换也没复制的 `.py` 在构建时给出警告，建议 `--include`
- [x] `.pyc` 用 `unchecked-hash`（标准库、site-packages）和 `checked-hash`（`src/` 里从源码 import 的文件），丢掉修改时间的拷贝不会每次启动都重新编译；`compileall` 失败时构建失败，单个文件编译不过只警告，并行编译失败时改为单进程
- [x] 附带的解释器安装项目模块的 finder，只在 CPython 能找到源文件时回答（项目目录在 `sys.path` 里），`--no-sources` 的分发里 `subprocess.run([sys.executable, "-m", "pkg.cli"], cwd=项目目录)` 能用。比较路径时不要求目录存在：`--no-sources` 的分发里转换过的包没有目录，最初用 `canonicalize` 比较时 spawn 的子进程 import 不到 `pkg.sub`，Pool 不停地重启 worker（验收时发现，`tests/dist_apps/mp/mptools` 是回归测试）
- [x] 小修：`SpawnFinder` 完成任务后把自己从 `sys.meta_path` 去掉；子进程里原生 `__main__` 出错时恢复 `sys.modules["__mp_main__"]`；purelib 和 platlib 不同时 `.pth` 的警告不再重复；输出路径是文件时报 `DistError`；remap 参数追加到用户的 rustflags 后面而不是覆盖；`inspect_python` 能处理没有 `METADATA` 的 `.dist-info`；`--python` 是 `include-system-site-packages = true` 的 venv 时给出警告
- [x] §12 记录 `PYTHONSAFEPATH`（`-P`）的差异
- ext 模式下 `python -m pkg`（M6 遗留）仍然不行：`pkg` 是扩展模块，runpy 在它变成包之前就按普通模块处理。`python -m pkg.__main__` 现在可以（M12 起安装成 `pkg/__init__.so` 时 `python -m pkg` 也可以）

## M8 推导式与其它 ✅

- [x] list/set/dict 推导式：和 CPython 3.12 起一样内联（PEP 709），降级为语句前面的循环，变量是循环体里的 `let`，不泄漏也不遮蔽外面的变量；第一个可迭代对象在外层求值，类体里的推导式看不到类的命名空间，dict 推导式先 key 后 value，错误报告在外层帧里出错的那一行（§8）
- [x] 生成器表达式是真正惰性的迭代器（不物化）：创建时只求值第一个可迭代对象，其余在每次 `next()` 时由一个 step fn 执行，有自己的 `<genexpr>` 帧，捕获的变量 late binding，`send`/`throw`/`close`、PEP 479、重入检查和 CPython 一样，参与垃圾回收（§8，runtime 的 `Generator`）。多个 `for` 的（标准库 3 个）和变量被内层函数捕获的保留为 Python。和 `types.GeneratorType` 的差异见 §12
- [x] 海象运算符 `:=`，包括推导式和生成器表达式里绑定外层函数或模块作用域的（PEP 572），定值分析和求值顺序都按 `:=` 处理
- [x] 顺带：模块级复合语句里的 `def`（条件定义等）原生转换为 `exec_module` 里的 fn item
- [x] generator 函数的可行性调研写进 §8：暂时保留为 Python，不开里程碑
- 容器字面量里的 `*x`/`**d` 展开仍然保留为 Python（标准库 26 个函数）

覆盖率的变化见 `coverage.md`：6 个工具的原生函数从 73% 到 93%（剩下的 11 个都是 generator），标准库从 90% 到 96%。

## M9a 默认去掉行号 marker ✅

M9 第一项提前做，并且改成默认行为。

- [x] 默认不生成 `frame.line(N)`，保留 `rt::frame!`（它同时承担异常协议），traceback 里原生帧的函数名和帧的顺序不变，行号是 `def` 行，`<module>` 是 1（§9）。marker 原来占 golden 非空行的 11.6%、转换后的标准库工具的 14.1%，并且改过的 Rust 里 marker 过期后会报一个很肯定的错行号
- [x] `--traceback-lines`（`CrateOptions.traceback_lines`）恢复语句级的行号。在 lowering 里实现（`at_line` 不输出 marker，行号记录照旧），不是 Rust AST 上的后处理。两种模式生成的代码结构相同，只差 marker、`|frame|`/`|_|`，以及去掉 marker 后 `else { if }` 合并成 `else if`（用所有用例、`tests/apps`、`tests/projects` 和 154 个标准库模块比较过）
- [x] 函数体不用 `frame` 时闭包参数写 `|_|`。`rt::frame!` 的闭包体总是带花括号，只有一个表达式时也是 `|_| { expr }`，避免 rustfmt 把长表达式拆在宏调用的几行里
- [x] e2e、`tests/apps`、`tests/projects` 用 `--traceback-lines` 构建，和 CPython 逐行比较的测试不变。另加默认模式的 e2e（帧的顺序和 CPython 一样，原生帧报 `def` 行），项目的默认输出也要求编译没有警告、clippy 干净
- [x] golden 重新生成，`tests/golden/traceback_lines/` 留几个用例的 marker 版本。golden 从 20493 行减到 18114 行，`tests/apps` 的输出从 11041 行减到 9419 行
- [x] `examples/logsum/rewritten` 里没改写的函数换成新的生成结果，§15 写明改写时怎么处理 `rt::frame!`

## M9 可读性 ✅

只做低成本、按构造就保持语义的改进。基于类型注解的 typed lowering 和自动升级 `#[pyclass]` 移到"以后再说"。

- [x] 先测量：在 golden、`tests/apps`、`tests/projects` 和 154 个标准库模块上统计 clone、临时变量、`as_any`/`into_any` 等写法。结论是大部分 clone 和临时变量都是必要的（参数是 `&Bound`、两个 Python 变量共享对象、求值顺序），数据和取舍见 §8
- [x] 返回或 yield 在同一个 fn 或闭包里声明的局部变量时 move 而不是 clone（`rustast/cleanup.py`）
- [x] 个数相同、没有 `*` 的解包赋值 `a, b = b, a` 写成 `[a, b] = [b.clone(), a.clone()]`，不构造元组再解包；`globals()` 不再 clone；装饰类的最外层装饰器在保存时调用，和函数一样
- [x] 只会是 `bool` 的局部变量是 Rust `bool`（§5，`frontend/booleans.py`），只认 Python 语义保证是 `bool` 的值（`not`、`is`、`in`、`isinstance`、`bool()`、字面量和它们的 `and`/`or`），比较运算不算。标准库 12087 个局部变量里有 128 个
- [x] 用例 `bool_locals`（富比较返回非 bool、`__bool__` 的调用时机、被捕获、可能未赋值、`try`/`with` 里赋值）和 `returned_locals`（move 之后 `__del__`、weakref 的时机不变）
- [x] CI：`.github/workflows/ci.yml`，ruff、cargo fmt、clippy 和完整的 pytest（§14）
- [x] 先声明后赋值的局部变量移到第一次使用之前，第一次使用就是赋值（或者每个分支最后都赋值的 `if` 链、`match`）时带值声明：`let kind = if .. { a } else { b };`（§8，较新的 clippy 报 `needless_late_init`）。局部变量按 `co_varnames` 的顺序声明，函数返回时的释放顺序和 CPython 一致（§5，剩下的差异见 §12。M14 之后的简化改成按第一次出现的顺序声明，见下面的"简化"）。用例 `late_init`、`drop_order`，golden 里的扫描测试保证不再出现可以合并的写法。标准库里合并了 308 个

已有用例的 golden 从 18168 行变成 18138 行（新增的两个用例另有 815 行），`tests/golden/projects` 的模块从 7873 行变成 7860 行，`tests/apps` 的输出从 9419 行变成 9407 行，154 个标准库模块从 332793 行变成 332235 行。行数变化不大，主要的变化是返回值少了 clone、解包不再构造元组、bool 局部变量不再经过 `rt::bool` 和 `is_truthy`（标准库里 `rt::bool(` 从 1251 个减到 1048 个，`is_truthy` 从 5585 个减到 5440 个）。

## M10 真实项目验证 ✅

用 skime（一个用 Python 写的 Scheme 实现，24 个模块，4582 行）完整走了一遍，详细记录和建议见 `m10-skime.md`。

- [x] 转换：不需要任何参数，函数 373/374 原生（剩下一个 generator），行 99.9%
- [x] 和 CPython 比较：8 个 Scheme 程序（REPL）输出、退出码相同，`--traceback-lines` 下完整 traceback 相同
- [x] skime 自己的 108 个 pytest 用例对 ext 模式的原生包全部通过（CPython 0.17 s，原生 0.43 s）
- [x] `steelix build` 的分发（61 MB，tar.gz 20 MB）在 `bwrap` 里运行正确，包括 `--no-sources`
- [x] 按 §15 改写读取器（第二步，纯 Rust 加单元测试）和 VM 主循环（第一步），记录了 10 个阻碍
- [x] 修复：复数字面量（`rt::complex`，用例 `complex_constants`），clippy 的 `collapsible_if`（用例 `nested_if`）和 `module_inception` 在 crate 根上 allow
- [x] `tests/projects/skime`：原样副本加驱动脚本和 Scheme 程序，和 CPython 比较，不做 golden（生成的 Rust 有 1.5 万行）
- 发现的主要问题是性能：解释器这类小函数密集的程序比 CPython 慢 6 倍，原因是 Python 调用原生函数的开销（没有 vectorcall）和属性名没有 intern（只改后者就快 1.8 倍）。下一步的建议按优先级排在 `m10-skime.md` 第 10 节

## M11 低成本的性能改进 ✅

M10 发现解释器这类小函数密集的程序比 CPython 慢 6 倍。只做不改变语义、不影响生成代码可读性的改进，复杂的不做。详细的设计在 §6 "调用的开销"，数字在 `m10-skime.md` 第 7 节。

- [x] `Function` 支持 vectorcall（PEP 590），在创建第一个函数时补到类型对象上（PyO3 的 `#[pyclass]` 不能声明）。`__call__` 保留，两条路径共用一个绑定函数，报错文字不变。入口经过 PyO3 的 trampoline（错误、panic、attach 计数），并且自己检查递归深度（CPython 在 vectorcall 之前不检查）
- [x] 只有位置参数的函数用位置参数调用时不做绑定，直接把参数数组传给函数体，参数少于形参时从调用时的 `__defaults__` 补上
- [x] `Py_TPFLAGS_METHOD_DESCRIPTOR`：`obj.m()` 不再创建 bound method，取属性 `obj.m` 照样得到 bound method
- [x] 属性名驻留：生成代码用 `rt::Attributes` 的 `attr`/`set_attr`/`del_attr`/`method0`/`method1` 代替 PyO3 的 `getattr`/`setattr`/`delattr`/`call_method0`/`call_method1`，写法一样短，名字是 `'static` 的字面量，runtime 每个名字只建一次驻留字符串（§4）。`m.global`、`rt::builtin` 的缓存改成按地址索引、读取不加锁的表，`rt::call!` 的关键字参数、`with` 的特殊方法查找、`*args` 展开里的 `list.extend` 也用它
- [x] 用例 `call_protocols`：各种调用路径（vectorcall、`f.__call__`、`type(f).__call__`、bound method、`classmethod`/`staticmethod`、`super()`、`functools.partial`/`partialmethod`）、快速路径的边界（参数个数不对、调用时改 `__defaults__`、比形参多的 `__defaults__`）、关键字参数和 `**kwargs`、经过 Python 调用的递归、新线程里异常对象的释放时机
- [x] 测量：skime fib 22（release）从 2.93 s 到 1.10 s（CPython 0.48 s），skime 自己的 108 个测试对 ext 模式的原生包从 0.40 s 到 0.24 s（CPython 0.16 s），单次调用的数字见 `m10-skime.md`
- 没有做：参数不纯时的方法调用仍然先建 bound method（要在 runtime 里重写 `_PyObject_GetMethod`），`rt::call!` 的关键字参数仍然先建 dict，字符串常量不缓存，全局变量的 dict watcher 缓存（§7）

golden 里属性和方法调用的写法都变了（`slf.getattr("x")?` 变成 `slf.attr("x")?`），rustfmt 合并了一些行，已有用例的 golden 少了约 100 行。

## M11a M9 到 M11 之后的修复 ✅

来自 M9 到 M11 之后的 review，都有复现。

- [x] 死锁：`Function` 的元信息 setter 在持有 `Mutex` 时 drop 旧值，旧值的 `__del__` 读同一个函数的元信息就永远卡住，`__del__` 释放 GIL 时另一个线程读元信息也一样。现在先换出旧值，释放锁之后再 drop（`Function::replace`，`__clear__` 也一样），`str` 子类的 `__str__` 也不在锁里调用。检查了 runtime 里其余的锁和 `RefCell`，生成器表达式的名字 setter 和 `repr` 有同样的问题，一起改了（§6，用例 `metadata_del`，包括另一个线程在 `__del__` 期间读元信息）
- [x] 接住的异常释放得太晚：`frame.raise` 把异常存在 `Frame` 里，只有 `frame.catch` 取走它，handler 是 `pass`、只有 `continue`/`break`、内层 `raise` 被外层的空 handler 接住、`finally` 之后被外层的空 handler 接住时，异常（参数、traceback）要等函数返回才释放。现在 `Frame` 只记异常的地址和它 traceback 第一项的帧，不持有异常，生成的代码不变（§9，用例 `handler_release`，包括异常释放后新异常用了同一个地址的情况）
- [x] §12 记录几处释放时机的差异（M11a 之前就有，都和 CPython 比较过）：经过函数对象调用的参数按书写顺序释放，只在 `with` 里用到的 `as` 目标在 `with` 结束时释放，语句里的临时值活到语句结束，异常离开原生函数时局部变量立即释放（CPython 由 traceback 的帧持有）。都要改生成代码的结构才能修，没有做。另外记录两个 steelix 扩展模块各有自己的 `function` 类型，§6 说明类型的模块名 `steelix_runtime` 不能 import
- [x] free-threaded：驻留名字的表读取不加锁，只在有 GIL 时正确。runtime 加了 `build.rs`（`pyo3-build-config`），在 free-threaded 的构建上 `compile_error!` 并指向 §6。查找表满了之后的 `HashMap` 从每个线程一张改成全进程一张，同一个名字只泄漏一次
- [x] `rt::Attributes` 对所有的 `Bound<'py, T>` 实现，手写代码里的 `Bound<PyDict>` 等不用 `.as_any()`
- [x] `rt::function!` 的排版：签名里有 `slf as "self"` 并且一行放不下时 rustfmt 不处理整条语句，以前打印出来的缩进是从第 0 列算的。现在打印 Rust AST 时一直带着当前的缩进层级（包括 `mod` 和函数里的 fn），宏的参数按所在的层级缩进。多行的 docstring（函数的 `doc = ..`、类和模块的 `__doc__`）一行对一行打印，行尾用 `\` 续行，值不变（§4，用例 `docstrings`）
- 没有做：review 建议把不纯的参数先求值再写成 `o.method1("m", (args,))`，这会让参数在查找方法之前求值，property、`__getattr__` 能看出来（§8）

## M11b 类体和装饰器的写法 ✅

类体里每个方法要占五六行（`ns.set("__init__", rt::function!(ns, ..., qualname = "Vector.__init__")?)?`），带装饰器的 `def` 是一个有 `let decorator`、`let function` 的块。改成和 Python 一样的一行，语义不变。

- [x] `rt::def!` 的第一个参数也可以是类体的 `ns`，在类的命名空间里绑定名字（`ns.set`），`rt::def!` 绑定的名字是函数名本身，不再读 `__name__`（§6）
- [x] 方法的 `__qualname__` 由 runtime 推出：`rt::Namespace` 记着类的 qualname，`ns` 里定义的函数是 `<类>.<名字>`，嵌套类的方法是 `Outer.Inner.method`。`qualname = ".."` 只在不同的时候写（§10）。`from __future__ import annotations` 时方法以前用 `m` 定义（为了不创建 `__classdict__` cell），现在 `rt::class!` 带上 `future_annotations`，方法同样用 `ns`
- [x] Rust 名字和 Python 名字不同的函数写成 `label_ as "label"(..)`，和参数的 `slf as "self"` 一样，代替 `name = "label"`
- [x] 装饰器写在名字前面：`rt::def!(ns, @property length(slf as "self"))?`、`@label.setter`、`@functools.wraps` 这样的点号名字按定义处的规则读（`ns.get`、`m.global`），别的表达式写成 `@(expr)`。宏按 CPython 的顺序求值装饰器、默认参数，再从下到上应用。装饰器或默认参数需要临时变量、里面有 `:=`，或者 `--traceback-lines` 时 CPython 会报告不止一个行号（多个装饰器、可能出错的默认参数）时，保留原来一条语句应用一个装饰器的写法（§6）
- [x] `--traceback-lines` 时装饰器求值和应用出错报告装饰器自己的那一行，默认参数报告 `def` 那一行，和 CPython 一样。以前都报告 `def` 那一行（§12 原来的记录也不准确）
- [x] 嵌套函数的装饰器同样写在 `rt::function!` 里，模块级的 `def` 也是
- [x] 用例 `decorators_uncaught`（装饰器、默认参数和应用的顺序，property 的 getter/setter/deleter，classmethod/staticmethod，嵌套类，名字是 Rust 关键字的方法，每个函数的 `__qualname__`，点号名字和调用作为装饰器，局部变量和 `:=` 作为装饰器，函数里装饰器出错的 traceback，带和不带 `--traceback-lines`）和 `decorator_lines`（模块、类体、函数里装饰器求值、默认参数、应用出错时的行号）

runtime 的宏变了（`rt::def!` 的装饰器和 `as`、`rt::class!` 的 `future_annotations`），生成的代码要配合新版本的 `steelix-runtime`。

顺带：生成的 `Cargo.toml` 加了 release profile（`lto = "fat"`、`codegen-units = 1`），skime 的 fib 22 快约 8%，release 构建慢约一半（§11）。

## M12 改写流程 ✅

M10 改写 skime 时最主要的阻碍是改写过的模块要在 bin 和 ext 两个 crate 之间手工复制，只为了能用项目自己的 pytest 测原生代码。不做"一个 crate 同时产出 bin 和 ext"，改为直接用 bin 跑测试：程序以 `python` 的名字启动时就是完整的解释器，并且安装了原生 importer（M7a），`import` 项目模块得到的就是原生代码。

- [x] 提供用生成的程序以解释器身份运行项目测试的入口：`steelix test CRATE [pytest 参数]` 构建 crate，通过名为 `python` 的链接以解释器角色运行 `python -m pytest`，`STEELIX_NATIVE_IMPORTS=1` 让解释器回答所有转换过的模块的 import（项目目录不在 `sys.path` 上也一样），pytest 插件 `steelix_native` 报告原生导入的模块数，转换过的模块从源码 import 时运行失败（§15）
- [x] 用 skime 的 108 个测试验证这条路径，替代 M10 的 ext 复制脚本：测试目录（skime 0.3.1 的原样副本）放在 `tests/projects/skime/tests`，`tests/test_projects.py` 在一个和项目无关的目录里运行它们，108 个全部通过，24 个模块都是原生的（`m10-skime.md` 第 3 节）。`inventory` 的 `store` 另有测试覆盖包里的测试文件和从源码 import 的检查
- [x] 补充 §15：被多个模块共享读写的类（如 skime 的 `Context`）怎么改写，适配层怎样构造别的模块的类、抛项目自己的异常类（runtime 加了 `m.error("ParseError", (..,))`），内联别的类的方法时的语义变化，Python 源码本身是生成出来的时候怎么保持单一来源
- [x] 属性读写索引：列出每个实例属性被哪些函数读写，改写共享的类之前先看这张表。`steelix attrs`（选择程序的参数和 `transpile` 一样，`--class`、`--json`），接收者是 `self`/`cls`、注解为这个类的参数、只由构造这个类的调用绑定的局部变量和类对象本身时记在类下面，其余按属性名和方法名列出，动态访问按类或函数列出（§15）。在 skime 上的结果见 `m10-skime.md` 第 6 节
- [x] `#[pyclass]` 改写草稿（只在用户指定某个类时生成，不接入构建）：字段取自 `__static_attributes__`，类型先留成 `Py<PyAny>` 加 TODO，并且和属性读写索引一起报告这个类是否被继承、有没有动态 `setattr`/`getattr`、`__dict__`/`vars` 的使用、装饰器和 metaclass，作为 §15 第二步的起点。不做自动升级（§10）。`steelix attrs --pyclass NAME`，桩能通过 PyO3 的类型检查（`tests/test_attrs.py`）
- [x] ext 模式下 `python -m pkg`（M6 遗留）：改写流程不再需要它，但发布的库也可能有 `__main__`。扩展模块安装成 `pkg/__init__.so` 时 import 之前就是包，`runpy` 能找到 `pkg.__main__`，`init_extension` 据此把项目目录提到包目录外面（§11）
- [x] ext 模式退回次要用途（作为扩展库发布），不再参与改写流程：§11、§15、README 改成用 bin 的解释器角色跑项目自己的测试

## M13 协程和生成器 ✅

用 Rust 稳定版的 `async` 当状态机实现 Python 的协程和生成器函数，不用执行器，进展只来自 Python 调用 `send`/`throw`/`close`，生成的代码保持直线（§8 "generator 函数"的第 4 种做法和"协程和生成器"）。先做了 runtime 和手写的"生成代码"的 PoC，再做 lowering。

- [x] runtime：`rt::Coroutine`（`coroutine`）和生成器对象，`send`/`throw`/`close`、`__await__`（`coroutine_wrapper`）、`__iter__`/`__next__`，CPython 的报错文字（重入、重用、未开始时 `send` 非 None、PEP 479、ignored GeneratorExit），`close()` 的返回值（3.13）
- [x] `rt::await_`、`rt::yield_`、`rt::yield_from`：PEP 492 取 awaitable 的迭代器，PEP 380 的 `send`/`throw`/`close` 委托，通过 waker 找到这一次 poll 的通道
- [x] `rt::def!(m, async f(..))`、`rt::def!(m, gen f(..))`，函数对象的 `__code__` 带 `CO_COROUTINE`/`CO_GENERATOR`，协程的名字取自调用时的函数对象
- [x] `rt::frame!` 接受 `async` 闭包，未开始时 `throw` 的异常在帧里抛出；`frame.with_async`；协程自己的异常状态（`sys.exception()` 跨过暂停）
- [x] 终结：补上 `tp_finalize` 和先调用终结器的 `tp_dealloc`，释放时 `close()`，从没开始的协程发出 `never awaited` 警告，出错经过 `sys.unraisablehook`
- [x] `tests/test_poc_async.py`：asyncio（`sleep`、直接 await 原生 future、经过函数对象、await Python 协程、`gather`、取消时 `finally` 里再 await、经过原生和 Python 协程的取消、traceback）、手动驱动（`send`/`throw`/`close`、ignored GeneratorExit、重入、重用、PEP 479、await 非 awaitable、never awaited、`del` 和 GC 时的终结顺序）、生成器（迭代、`send`、`throw`、`close`、`yield from` 委托、`with` 里的生成器、递归的 `Box::pin`）、`inspect`/`asyncio` 的判断、不用 asyncio 的 trampoline。和 CPython 的差异记在 §12
- [x] PoC 的 review 修复（`tests/test_poc_async.py` 都和 CPython 比较）
  - [x] 让 GC 看到暂停的协程：协程对象持有模块、捕获的变量和参数（`Locals`，future 只借用），`rt::await_` 委托的迭代器和帧对象放在协程对象里（`Held`），`Frame` 借用 globals，加上 `tp_clear`。测试：退出时 globals 里从没开始的生成器、暂停的生成器和协程、处理过异常后暂停的协程，经过 `self` 的方法生成器和 `self.task = create_task(self.run())`。局部变量和语句的临时值仍然看不到（§12）
  - [x] 在 `except` 里暂停时 `throw`、`close`、取消送进来的异常的 `__context__`
  - [x] `RuntimeError: coroutine is being awaited already`（Python 的 `await` 和 `rt::await_`，原生的和 CPython 的协程）
  - [x] 递归深度：每次 poll `rt::frame!` 的函数体算一层 Python 递归（3.14 的 `py_recursion_remaining`），每次 poll 和 `send` 检查 C 栈，`yield from`、`await` 的链和 `Box::pin` 的递归在和 CPython 相同的深度抛 `RecursionError`（M14 之后的简化去掉了 Python 的递归计数，现在只检查 C 栈，见下面的"简化"）
  - [x] 文档：future 的 `Send` 前提和改写时的规则、`join!`/`select!` 的限制（§15）
  - [x] 原生生成器之间满足直接调用条件的 `yield from g(x)` 可以直接 await 内层的 future
- [x] lowering：`async def` 和含 `yield` 的函数生成 `async fn`（`rt::frame!` 的 `async` 闭包，`rt::def!`/`rt::function!` 的 `async`、`gen`，方法、装饰器、嵌套函数），`await`、`yield`、`yield from`，会暂停的 `try` 区域换成 `async` 块（`let result: PyResult<..> = async { .. }.await;`），会暂停的 `with` 换成 `frame.with_async`，被 await 的表达式会留下临时值时先用 `let` 绑定（§8 "GC 看得到的东西"），`--traceback-lines` 照常工作
- [x] 原生 `async def` 之间满足直接调用条件的 `await f(x)`、原生生成器之间的 `yield from g(x)` 直接 await，调用图里在同一个环上的加 `Box::pin`，别的调用经过函数对象
- [x] `async with`、`async for` 的 helper：`frame.async_with`、`rt::aiter`、`rt::anext`，报错文字和 CPython 3.14 一样
- [x] 嵌套的 `async def`、生成器和 lambda 的捕获，`nonlocal`（`async_basics`、`generators`）
- [x] 保留为 Python，记入覆盖率报告的特性名：异步生成器、异步推导式、推导式里的 `await`、lambda 里的 `yield`、`types.coroutine` 的生成器、`except` 子句和嵌套函数的装饰器、默认值里的 `await`（`async_kept`）
- [x] 用例：`coroutines`（PoC 的程序，现在由 lowering 生成，`tests/poc_async` 和 `tests/test_poc_async.py` 退役，§14）、`async_basics`、`async_with_for`、`async_uncaught`（也在默认行号下测）、`async_kept`、`generators`，`async_with_for`、`generators` 也有 `--traceback-lines` 的 golden
- [x] 覆盖率（`coverage.md`）：标准库的函数 96.7% → 99.1%，6 个工具 93.3% → 100%；skime 374/374（`m10-skime.md`）
- [ ] 改写后的代码 await Rust 生态 future 的桥接 helper（只在需要时）

runtime 的公开接口变了（`FunctionDef::body` 是 `function::Kind`，新的 `rt::Coroutine`、`rt::await_`、`rt::aiter`、`rt::anext`、`Frame::async_with` 等，`rt::Frame` 多了一个生命周期参数 `Frame<'a, 'py>`），生成的代码用到了这些，下次发版要提升 `steelix-runtime` 的版本和 `RUNTIME_VERSION`。

## M14 match 语句 ✅

`match` 原来整个保留为 Python（标准库里 17 个函数，为 3.10 以后写的应用会多一些，比如解析命令、处理 JSON 消息）。Rust 的 `match` 对不上，因为 Python 的模式全是运行时的检查（`isinstance`、`len()`、`Sequence`/`Mapping`、`__match_args__`、`getattr`、`.get()`、`==`/`is`）。对得上的是 let chains（Rust 1.88 起在 edition 2024 稳定）：每个 `case` 是一个 `if let .. && ..`，模式的每一层是链上的一节，和 Python 原文一行对一行（design §8 "match 语句"）。

```rust
// case ["go", direction] if direction in EXITS:
if let Some([item, direction]) = rt::match_seq(&subject)?
    && item.eq("go")?
    && m.global("EXITS")?.contains(&direction)?
{
    ..
// case Point(x=0, y=y):
} else if let Some([x, y]) = rt::match_class(point, &m.global("Point")?, 0, ["x", "y"])?
    && x.eq(0)?
{
    ..
}
```

- [x] runtime 的 helper（`matching.rs`），每个一次取出这一层的所有值（和 CPython 的 `MATCH_CLASS`、`MATCH_KEYS`、序列解包一样），返回 `Option<[Bound; N]>`，let 链再按深度优先、从左到右检查子模式，所以求值顺序和短路与 CPython 相同
  - `rt::match_seq`、`rt::match_seq_star`、`rt::match_seq_len`、`rt::match_seq_min`、`rt::item_from_end`：`Py_TPFLAGS_SEQUENCE`（str、bytes 不算）、`len()`，取值方式照 CPython 编译器按模式形状的选择（迭代或下标），自定义的 Sequence 能看出区别
  - `rt::match_map`、`rt::match_mapping`、`rt::match_keys`、`rt::match_rest`：`Py_TPFLAGS_MAPPING`、`.get(key, 哨兵)`、重复 key 的 `ValueError`、`**rest` 复制后删掉已匹配的 key，不是字面量的 key 在映射检查之后才求值
  - `rt::match_class`、`rt::match_instance`：`isinstance`（不是类型时 `TypeError`）、`__match_args__`（必须是 tuple，元素必须是 str，位置参数太多时报错）、关键字属性用 `getattr`（`AttributeError` 算不匹配）、`int(x)` 这类内置类型的位置参数匹配主体本身、重复属性的 `TypeError`，错误文字和 CPython 一样
- [x] 字面量用 `==`（主体在左边），`None`/`True`/`False` 用 `is`，`-1`、`1 + 2j` 当成常量，值模式（`Color.RED`）走到那一步才求值
- [x] 捕获的名字在整个模式成功之后、guard 之前按 CPython 的顺序绑定（guard 失败也已经绑定，`match` 之后能读到）。只在这个 case 里用到时直接用 `if let` 绑定的名字（`LocalsInfo.case_scoped`），否则先赋给局部变量再求值 guard，这时整个 `match` 是带标签的块，否则是 `if .. else if ..` 链
- [x] 不带捕获的 OR 模式写成 `||`（嵌套时用 `matches!`），带捕获的和有不会失败的分支的保留为 Python（覆盖率报告的特性名 `OR pattern with captures`、`OR pattern with an irrefutable alternative`）
- [x] 模块级、类体、协程和生成器里的 `match`，guard 里的 `:=`、生成器表达式和 lambda，`--traceback-lines` 的行号（出错的子模式和 guard 所在的行），模块级的 `match` 和 `if`/`for` 一样按 case 体拆开保留
- [x] 对照 CPython 3.14 `Python/codegen.c`、`Python/ceval.c` 的模式匹配部分逐条核对。用例 `match_basic`（命令解析、各个位置的星号、嵌套、guard 失败后仍然绑定、OR、AS、单例和 `==`、模块级和类体）、`match_classes`（`__match_args__`、dataclass、enum、内置类型）、`match_mappings`（`**rest`、值模式做 key、非 dict 的映射）、`match_order`（自定义的 Sequence/Mapping/`__eq__`/`__getattr__`/`__match_args__`/`__instancecheck__` 打印每一次调用和存名字的顺序）、`match_errors`（所有报错文字）、`match_async`、`match_uncaught`（也在默认行号下测）、`match_kept`、`match_release`（§12 的释放时机），`match_basic`、`match_async` 也有 `--traceback-lines` 的 golden
- [x] 原来用 `match` 当"不支持的写法"的测试换成 `type` 别名和 `try`/`except*`
- [x] 覆盖率（`coverage.md`）：标准库的函数 99.1% → 99.3%，行数 97.0% → 97.8%，17 个用 `match` 的函数全部原生，没有带捕获的 OR 模式；含 `match` 的 8 个标准库模块 `cargo clippy` 没有错误，也没有出自 `match` 的警告

runtime 的公开接口多了 `matching.rs` 的 11 个函数（`rt::match_seq` 等），生成的代码用到了它们，下次发版要提升 `steelix-runtime` 的版本和 `RUNTIME_VERSION`（和 M13 的变化一起）。

## M13/M14 之后的修复 ✅

M13、M14 之后的独立 review 发现的问题。

- [x] 原生函数之间直接委托（`yield from g(x)`、`await f(x)`）时 `close()` 和 `throw(GeneratorExit)` 不符合 CPython 的 `_gen_throw`：内层吞掉 `GeneratorExit` 之后外层会继续执行，内层又暂停时 `RuntimeError` 从外层的 `close()` 出来而不是在外层的委托处抛出。直接委托改写成 `rt::delegate(g(m, &x)).await?`（递归的是 `rt::delegate(Box::pin(..))`），先关闭内层，再在外层抛出 `GeneratorExit` 或关闭内层的错误，多层委托每层各做一次，`__context__` 也和 CPython 一样（design §8 "直接 await 和递归"，`generator_close` 用例比较直接委托和经过变量的委托）
- [x] 超过 12 个位置参数的 `print(..)` 和 `raise ValueError(..)` 生成的代码编译不过（特化绕过了参数个数的检查），带关键字参数的调用也一样。现在和一般的调用一样保留为 Python（`many_arguments` 用例）
- [x] `gen.throw(ExcType)` 给不是生成器的委托对象的 `throw` 传的是实例，CPython 传原来的参数
- [x] 原生的 `generator`、`coroutine`、`coroutine_wrapper`、生成器表达式的对象和 `__annotate__` 的 `__doc__` 是 Rust 的文档注释，改成和 CPython 一样
- [x] 报错信息里的类型名是 `steelix_runtime.generator`（`cannot pickle 'steelix_runtime.generator' object`），改成 `generator`，类型的 `__module__` 和 CPython 一样是 `builtins`
- [x] design §12 补上原生帧的警告位置（`<sys>:0`）带来的后果（默认 filter 不显示 `DeprecationWarning`，多个 never awaited 只显示一次）和 `repr(asyncio.Task)` 没有 `running at`（`coroutine_warnings` 用例）

runtime 的公开接口多了 `rt::delegate`，生成的代码用到了它，和 M13、M14 的变化一起发版。

## 简化（M14 之后的架构 review）

原则：程序很少依赖的实现细节（确切的递归深度、函数返回时局部变量之间的释放顺序、委托时内层看到的是谁的异常）不追求和 CPython 完全一致，对齐的代价大时用简单的实现，差异记在 design §12，e2e 用例用 `KNOWN_DIFFERENCES` 或只比较异常类型记录下来。程序依赖的语义（异常类型、副作用顺序、`close`/`finally`、`except E: await x; raise F` 的 `__context__`）仍然和 CPython 一样。

- [x] 去掉协程和生成器的 Python 递归计数（`Depth`、按 3.14 布局读写的 `PyThreadState`、`build.rs` 的 `python_recursion`、`rt::frame!` 的 async 版本外面的 `poll_fn`）。每次恢复函数体只在 `Core::resume` 检查一次 C 栈，直接委托的链在 `rt::delegate` 每层检查一次，太深时是 C 栈的 `RecursionError`，debug 和 release 都不崩溃（`coroutines` 用例的 10 万层链，只比较异常类型）。`coroutine.rs` 少了 4 个 unsafe 块，per-item 的生成器从约 80 ns 到约 73 ns（release，`yield from` 两层从约 127 ns 到约 102 ns）
- [x] 局部变量不再按 `co_varnames` 排序声明（`lower/function.py` 的排序、`declare_late` 的限制、`ModuleContext.varnames`、`frontend/classinfo.py` 里取 `co_varnames` 的部分；编译模块仍然用来取 `__static_attributes__`），按第一次出现的顺序声明，`declare_late` 可以越过别的声明合并 `let x = if ..`。标准库里没有值的声明从 3774 个变成 3746 个。释放顺序的差异记在 §12（`drop_order` 用例的 `crossing`）
- [x] 简化协程里的异常状态：保留函数体自己的异常在暂停前后的保存和恢复（`State::Suspended` 的 `handled`）和 `throw` 进来的异常的 `__context__`，去掉让直接委托的内层、`throw` 进委托对象时看到调用方的异常的机制（`Channel` 的 `caller`、`raise_in_body`、`as_caller`、`rt::delegate` 的 `outer`/`inherited`、`rt::Handling` 里协程相关的字段、`coroutine::polling()`）。`rt::Handling` 和同步代码一样无条件恢复，暂停在 `except` 里的函数体在自己的 poll 以外被释放时由 `Body` 保存和恢复线程的状态（`coroutines` 用例的 `stubborn_in_except`）。`coroutine.rs` 少了 4 个 unsafe 块。差异记在 §12（`coroutines`、`generator_close` 用例的 `KNOWN_DIFFERENCES`），上一节"`__context__` 也和 CPython 一样"现在只对外层自己的异常成立
- [x] ~~生成器表达式改用协程的机制，删掉 `generator.rs`~~，不做。先测了每个元素的开销（release，LTO）：`sum(x * 2 for x in xs)` 用现在的 step 约 35 ns，同样的函数体写成原生生成器约 63 ns（慢约 80%，CPython 约 22 ns），去掉 C 栈检查、异常状态的保存恢复和状态的 `Mutex` 也只到约 59 ns，开销分散在每次恢复的固定成本里。超过了 30% 的界限，保留 step（design §8 生成器表达式的否决方案）。两个同名的 pyclass 只改了 Rust 的名字：生成器表达式的是 `rt::GenExpr`，Python 的类名仍然是 `generator`
- [x] 拆分 runtime 的大文件：`coroutine.rs` 拆成 `coroutine/`（`state`、`channel`、`awaits`、`objects`、`finalize`），`lib.rs` 拆出 `names.rs`（驻留的名字）、`values.rs`（字面量、slice、f-string）、`macros.rs`（`rt::function!`、`rt::def!`），`rt::` 的路径不变
- [x] 拆分大文件：`frontend/bindings.py` 拆成 `module_bindings.py`（模块级绑定）和 `locals.py`（局部变量的定值分析）；`lower/expr.py` 拆出 `calls.py`（调用）、`defs.py`（`def`/`lambda` 的函数对象）和它们共用的 `values.py`（`Kind`、`Value`）。有了 `values.py`，`comprehension.py` 里为避开循环 import 写在函数内的 import 也挪到了模块顶部
- [x] `frame.catch` 和 `rt::Handling` 合成一个 RAII 的值 `rt::Caught`，`except` 块里只写 `let e = frame.catch(err);`（没有 `as` 时是 `_caught`）。`Caught` deref 成 `Bound`，实现了 `IntoPyObject`、`IntoException`，生成的代码里 `e` 的用法不变，golden 净少约 115 行。`rt::Handling` 仍然用在 `finally`、`with` 的 runtime 里
- [x] 文档：§12 里不存在的 `--dynamic-globals` 改成"以后可以加"，§14 的 `traceback_lines/` golden 列表，§2 的流水线和 §3 的目录结构补上漏掉的模块、子命令和 runtime 文件，`CLAUDE.md` 是本地文件，不在仓库里

## 性能的候选项（未排期）

只做语义不变的。

- [x] ~~字符串常量缓存~~，不做。在 skime 的读取器上测（release，`parse` 一段 30 万字符的生成的 S 表达式）：生成的代码 622 ms（CPython 261 ms），把 `parser.rs` 里的 `.eq("..")` 和 `rt::str(py, "..")` 都换成 `pyo3::intern!` 之后 620 ms，约 1%。读取器的字面量几乎都是单字符，CPython 对 Latin-1 的单字符字符串本来就有缓存，新建它们不分配内存；差距在 Python 方法调用的开销上（`slf.method0("peak")` 等，M10 §7）
- [x] ~~`__slots__` 属性的 inline cache~~，不做。单次读取（release）：驻留名字的 `getattr` 约 10 ns，检查 `tp_version_tag` 之后按 member descriptor 的偏移直接读约 1 ns。但 skime 的 fib 22 只有约 20 万次 slot 读取（`CallValue.value`、`CallFrame.context`，热点的 `Context` 没有 `__slots__`），一共省约 1.6 ms，占 1.10 s 的 0.15%，不值得依赖非 limited API 的细节

- [x] 补默认参数时参数列表放在栈上（`SmallVec`），不再每次分配 `Vec`：skime 读取器约 3%，Python 侧调用有默认值的原生函数约 20 ns
- [x] `rt::call!` 只有关键字参数（没有 `*`/`**`）时直接 vectorcall，关键字名字的 tuple 每个调用点建一次：约 208 ns 到 124 ns（§6）
- [x] `for` 循环和推导式用 `rt::iter(&xs)?`，它在取后续元素之前检查信号，循环体里不再写 `py.check_signals()?;`，golden 净少约 180 行。`--traceback-lines` 时循环体结尾在把行号设回 `for` 行之前仍写这一行，让信号的异常和 CPython 一样报在循环体最后执行的语句上
- [ ] `Function` 加 `#[pyclass(immutable_type)]`，让 CPython 的 `LOAD_ATTR` 方法特化对原生函数生效（Python 侧方法调用约 5 到 10 ns），暂不做
- [ ] 没有返回值的函数返回 `PyResult<()>`，去掉 336 处函数末尾的 `Ok(rt::none(py))`（`rt::def!` 的包装要把 `()` 转成 None，成本中等）

## 发布

- [x] PyPI 发布 steelix 0.1.0，git tag `v0.1.0`
- [x] `steelix-runtime` 0.1.0 发布到 crates.io，从 PyPI 安装的 steelix 生成的 crate 可以直接构建（已验证）
- [ ] 0.2.0：版本号、`RUNTIME_VERSION = "0.2"` 和 tag `v0.2.0` 已就绪（runtime 的公开接口自 M13 起不兼容，所以不是 0.1.1），待发布到 PyPI 和 crates.io
- [ ] 以后每次发版：PyPI 和 crates.io 同步，`RUNTIME_VERSION` 与 runtime 的版本一致（design §14）

## 以后再说

generator 和 async/await 移到了 M13 并完成，match 移到了 M14 并完成。

基于类型注解的 typed lowering（`int` → `i64` 等）和符合条件的类自动升级为 `#[pyclass]`（M9 原来的计划，不做的原因是注解在运行时不强制，`def f(x: float)` 用 `3` 调用时结果必须是 int，要保持语义就要同时生成两套代码路径，见 §1、§10）、自定义 metaclass、`exec`/`eval`、多继承的边角情况、abi3、free-threaded、`--dynamic-globals`、Windows/macOS 上的 bin 模式、重新生成后与手工改写的代码合并（预期用法是生成一次后直接部署和改写，有需求再做，可以在每个生成函数前后加标记，由工具只替换未改写的函数）
