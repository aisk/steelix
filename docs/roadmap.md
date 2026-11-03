# Roadmap

每个里程碑结束时都应该有能跑通的端到端用例。任务粒度大约是一次会话能完成的量，完成后勾选并在 `tests/cases/` 留下对应用例。设计依据见 `design.md`，括号里是对应章节。

接下来的执行顺序（根据 `coverage.md` 的数据和 M3c 之后的 review）：M3d ✅ → M4 ✅ → M5 ✅ → M8 ✅ → M8a ✅ → M6 ✅ → M9a ✅ → M7 ✅ → M7a ✅ → M9 ✅ → M10。

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
- [ ] ext 模式下 `python -m pkg`：M7a 给 `NativeLoader` 加了 `get_code`，剩下的问题是扩展模块 `pkg` 的 spec 不是包，runpy 不会去找 `pkg.__main__`
- [ ] 命名空间包（没有 `__init__.py` 的目录）里的模块现在不转换

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
- ext 模式下 `python -m pkg`（M6 遗留）仍然不行：`pkg` 是扩展模块，runpy 在它变成包之前就按普通模块处理。`python -m pkg.__main__` 现在可以

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

已有用例的 golden 从 18168 行变成 18138 行（新增的两个用例另有 815 行），`tests/golden/projects` 的模块从 7873 行变成 7860 行，`tests/apps` 的输出从 9419 行变成 9407 行，154 个标准库模块从 332793 行变成 332235 行。行数变化不大，主要的变化是返回值少了 clone、解包不再构造元组、bool 局部变量不再经过 `rt::bool` 和 `is_truthy`（标准库里 `rt::bool(` 从 1251 个减到 1048 个，`is_truthy` 从 5585 个减到 5440 个）。

## M10 真实项目验证

- [ ] 用一两个真实的小项目完整走一遍转换、`steelix build` 部署、按 §15 逐步改写，记录转换覆盖率和改写时遇到的阻碍，据此决定下一步

## 以后再说

基于类型注解的 typed lowering（`int` → `i64` 等）和符合条件的类自动升级为 `#[pyclass]`（M9 原来的计划，不做的原因是注解在运行时不强制，`def f(x: float)` 用 `3` 调用时结果必须是 int，要保持语义就要同时生成两套代码路径，见 §1、§10）、generator、async/await、match、自定义 metaclass、`exec`/`eval`、多继承的边角情况、abi3、free-threaded、`--dynamic-globals`、Windows/macOS 上的 bin 模式、重新生成后与手工改写的代码合并（预期用法是生成一次后直接部署和改写，有需求再做，可以在每个生成函数前后加标记，由工具只替换未改写的函数）
