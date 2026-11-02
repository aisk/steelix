# Roadmap

每个里程碑结束时都应该有能跑通的端到端用例。任务粒度大约是一次会话能完成的量，完成后勾选并在 `tests/cases/` 留下对应用例。设计依据见 `design.md`，括号里是对应章节。

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

## M3 函数与闭包

- [ ] `Function` 完整签名绑定：默认参数（`def` 时求值，存 `__defaults__`）、关键字参数、`*args`、`**kwargs`、keyword-only（§6）
- [ ] 调用侧的关键字参数、`*`/`**` 展开
- [ ] 嵌套函数、闭包、`nonlocal`（cell 作为参数传入）
- [ ] `lambda`
- [ ] 装饰器（按 Python 语义执行，被装饰的名字退化为动态调用）
- [ ] docstring 转 `///`，注释保留，源位置注释

## M4 异常与上下文管理

- [ ] `raise`、`raise ... from ...`
- [ ] `try/except/else/finally`，`Flow` 枚举处理块内的 return/break/continue（§9）
- [ ] 进入 handler 时设置当前处理异常的 RAII guard，handler 内裸 `raise`
- [ ] `except (A, B) as e`、裸 `except`
- [ ] `with`（多个 item）
- [ ] 异常经过时用 `add_note` 附加 Python 源位置
- [ ] e2e 测试比较预期异常的类型和消息

## M5 类

- [ ] 动态建类（`types.new_class` 语义），每个类一个 Rust mod（§10）
- [ ] 方法、`__init__`、实例属性、类属性
- [ ] 继承（含继承 Python 定义的类）、`super()` 降级为 `super(Cls, self)`
- [ ] `staticmethod`、`classmethod`、`property`
- [ ] 自定义异常类
- [ ] PEP 649 注解，验证 dataclass 可用

## M6 多模块应用与混合模式

- [ ] 输入为应用目录（入口脚本加项目内模块和包）时生成一个 crate，每个 Python 模块一个 Rust mod
- [ ] 包初始化安装 meta path finder，子模块惰性执行（§7）
- [ ] 项目内模块之间的引用、相对 import
- [ ] 混合模式：不支持的函数或类保留 Python 源码嵌入执行（§13）
- [ ] 转换覆盖率统计（报告每种不支持特性出现的次数）

## M7 分发

- [ ] Spike：对比附带 python-build-standalone + site-packages、静态链接 libpython 等方案，结论写入 design.md §11
- [ ] 运行时按相对于二进制的路径定位 Python 安装，不再写死构建时路径
- [ ] `steelix build` 一条命令产出可分发的目录或压缩包
- [ ] 在干净环境（容器）里验证可运行

## M8 推导式与其它

- [ ] list/set/dict 推导式（降级为循环，注意推导式有自己的作用域）
- [ ] 生成器表达式的有限支持（作为 `any`/`all`/`sum`/`list` 等的参数时物化成列表，注明语义差异），其余走混合模式
- [ ] 海象运算符 `:=`
- [ ] 热循环中插入 `PyErr_CheckSignals`

## M9 可读性与渐进式改写

- [ ] 基于类型注解的可选 typed lowering（`int` → `i64` 等），只在用户显式开启时启用
- [ ] 符合条件的类可选升级为 `#[pyclass]`（§10）
- [ ] 生成代码的清理 pass：去掉多余 clone、合并临时变量
- [ ] 用一两个真实的小项目做验证，记录转换覆盖率
- [ ] CI

## 以后再说

generator、async/await、match、自定义 metaclass、`exec`/`eval`、多继承的边角情况、abi3、free-threaded、`--dynamic-globals`、Windows/macOS 上的 bin 模式
