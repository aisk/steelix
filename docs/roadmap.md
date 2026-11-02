# Roadmap

每个里程碑结束时都应该有能跑通的端到端用例。任务粒度大约是一次会话能完成的量，完成后勾选并在 `tests/cases/` 留下对应用例。设计依据见 `design.md`，括号里是对应章节。

接下来的执行顺序（根据 `coverage.md` 的数据和 M3c 之后的 review）：M3d → M4 → M5 → M8 → M6 → M7 → M9。M6 多模块应用视真实应用的需要可以提前。

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

## M3d 混合模式之后的小修

来自 M3c 之后的 Fable review，都有复现。

- [ ] 模块级嵌套语句的细粒度保留：`if`/`for`/`while`/`if __name__ == "__main__"` 里只有某个子语句不支持时，只保留那个子语句，其余仍然原生（模块作用域就是 globals，不需要同步局部变量；注意被保留的子语句里的 `break`/`continue`）
- [ ] 原生的 `while`/`for` 循环里调用 `PyErr_CheckSignals`，现在纯原生循环按 Ctrl-C 停不下来
- [ ] 每个被保留的 `def`/`class` 单独一个 `exec_python!`，各自带源位置和原因，只合并连续的简单语句。§13/§15 补充如何把保留为 Python 的函数手工改成原生
- [ ] 对自身模块的 `setattr`/`delattr`（`sys.modules[__name__]`、`__main__` 等）关闭直接调用，与 design.md §7 的描述一致。`exec` 同理
- [ ] `rt::builtin` 改为查 builtins 的字典，而不是对 builtins 模块 `getattr`（现在 `__annotations__` 等名字会读到模块对象的属性）
- [ ] 模块注解分散在多个保留块时合并，而不是只保留最后一块
- [ ] 带参数的 `vars(obj)` 不再关闭整个模块的直接调用
- [ ] §12 记录原生函数没有 Python 帧对 `logging`（`%(lineno)d`、`%(funcName)s`）和 `warnings` 的影响，`tests/apps` 加一个记录行号的用例

## M4 异常与上下文管理

- [ ] `raise`、`raise ... from ...`
- [ ] `try/except/else/finally`，`Flow` 枚举处理块内的 return/break/continue（§9）
- [ ] 进入 handler 时设置当前处理异常的 RAII guard，handler 内裸 `raise`
- [ ] `except (A, B) as e`、裸 `except`
- [ ] `with`（多个 item）
- [ ] 原生函数在 traceback 里的合成帧（类似 Cython：出错路径上用 `PyFrame_New` + `PyTraceBack_Here`，文件名和行号取自生成器已知的源位置），替代原来计划的 `add_note`
- [ ] e2e 测试比较预期异常的类型和消息
- [ ] 改写约定补充：改写后的适配层抛出的异常类型必须与原来一致，加 e2e 用例在改写过的函数外面 `except` 原来的异常类型

## M5 类

- [ ] 动态建类（`types.new_class` 语义），每个类一个 Rust mod（§10）
- [ ] 方法级的混合模式：类原生创建，用了不支持语法的方法保留为 Python 源码放进类的命名空间（`coverage.md`：不做的话 M5 在标准库上的收益从 +48 降到 +9 个百分点）
- [ ] 方法、`__init__`、实例属性、类属性
- [ ] 继承（含继承 Python 定义的类）、`super()` 降级为 `super(Cls, self)`
- [ ] `staticmethod`、`classmethod`、`property`
- [ ] 自定义异常类
- [ ] PEP 649 注解，验证 dataclass 可用

## M6 多模块应用

- [ ] 输入为应用目录（入口脚本加项目内模块和包）时生成一个 crate，每个 Python 模块一个 Rust mod
- [ ] 包初始化安装 meta path finder，子模块惰性执行（§7）
- [ ] 项目内模块之间的引用、相对 import

## M7 分发

- [ ] 部署后的二进制不能依赖源文件：`multiprocessing` 的子进程现在会用 venv 的 python 重新执行原来的 `.py`，源文件挪走就失败。需要让子进程使用二进制本身（`multiprocessing.set_executable`、`__spec__` 等），或者明确要求源文件随二进制一起部署
- [ ] Spike：对比附带 python-build-standalone + site-packages、静态链接 libpython 等方案，结论写入 design.md §11
- [ ] 运行时按相对于二进制的路径定位 Python 安装，不再写死构建时路径
- [ ] `steelix build` 一条命令产出可分发的目录或压缩包
- [ ] 在干净环境（容器）里验证可运行

## M8 推导式与其它

- [ ] list/set/dict 推导式（降级为循环，注意推导式有自己的作用域）
- [ ] 生成器表达式的有限支持（作为 `any`/`all`/`sum`/`list` 等的参数时物化成列表，注明语义差异），其余走混合模式
- [ ] 海象运算符 `:=`

## M9 可读性与渐进式改写

- [ ] 基于类型注解的可选 typed lowering（`int` → `i64` 等），只在用户显式开启时启用
- [ ] 符合条件的类可选升级为 `#[pyclass]`（§10）
- [ ] 生成代码的清理 pass：去掉多余 clone、合并临时变量
- [ ] 用一两个真实的小项目做验证，记录转换覆盖率
- [ ] CI

## 以后再说

generator、async/await、match、自定义 metaclass、`exec`/`eval`、多继承的边角情况、abi3、free-threaded、`--dynamic-globals`、Windows/macOS 上的 bin 模式、重新生成后与手工改写的代码合并（预期用法是生成一次后直接部署和改写，有需求再做，可以在每个生成函数前后加标记，由工具只替换未改写的函数）
