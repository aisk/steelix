# 真实应用的转换覆盖率（M3c，M3d、M4、M5、M8 更新）

数据截至 M13（最后一节）。真实项目的数据见 `m10-skime.md`。

M3c 的最后一步是拿不是我们写的代码跑一遍混合模式（design.md §13），看哪些已经能原生转换、哪些还保留为 Python，再据此决定 M4、M5、M8 的先后。

## 做法

目标是本机解释器（CPython 3.14.9）标准库里能当脚本运行的命令行工具，原样复制到 `tests/apps/`，文件开头加三行注释注明来源和 PSF 许可。每个工具以 bin 模式转换（默认的混合模式），在 `tests/apps/data/` 下按几组命令行参数运行，与 CPython 运行同一份副本比较 stdout、退出码和 stderr（M3c 时有 traceback 只比较最后一行，因为原生函数在 traceback 里没有帧；M4 之后比较每一帧的位置和源码行，见 design.md §14）。这些都在 `tests/test_apps.py` 里，共 33 组命令行。

| 工具 | 来源 | 命令行 | 函数（原生/总数） | 顶层语句 | 行数 |
|---|---|---|---|---|---|
| `stdlib_calendar` | `Lib/calendar.py` | 8 | 10/72（14%） | 28/42（67%） | 200/838（24%） |
| `stdlib_base64` | `Lib/base64.py` | 5 | 14/28（50%） | 33/33（100%） | 177/548（32%） |
| `stdlib_tabnanny` | `Lib/tabnanny.py` | 5 | 1/17（6%） | 9/11（82%） | 27/318（8%） |
| `stdlib_tokenize` | `Lib/tokenize.py` | 4 | 8/22（36%） | 56/60（93%） | 144/545（26%） |
| `stdlib_ast` | `Lib/ast.py` | 5 | 5/23（22%） | 4/14（29%） | 186/634（29%） |
| `stdlib_json_tool` | `Lib/json/tool.py` | 6 | 0/2（0%） | 9/9（100%） | 25/109（23%） |
| 合计 | | 33 | 38/164（23%） | 139/169（82%） | 759/2992（25%） |

函数指模块级 `def` 和类的方法（嵌套函数算在外层函数里），顶层语句不含 `def`。33 组命令行的输出全部与 CPython 一致。表里是 M3d 之后的数字，和 M3c 相比只有 `stdlib_ast`、`stdlib_json_tool` 各多了一条原生的顶层语句（见下面的"M3d 的变化"）。

另外两组更大范围的检查，没有放进测试（太慢，或者输出带对象地址）：

- 标准库全部 154 个顶层模块（`Lib/*.py`）都能转换，生成的 crate `cargo check` 没有错误和警告（M3d 之后重新检查过）
- 其中另外 25 个能当脚本运行的模块（`this`、`symtable`、`dis`、`pyclbr`、`mimetypes`、`inspect`、`pickletools`、`shlex`、`pickle`、`py_compile`、`gzip`、`quopri`、`textwrap`、`tarfile`、`platform` 等）跑了近 40 组命令行，输出与 CPython 一致，只有 `repr` 里的对象地址不同

数据可以用 `uv run steelix coverage tests/apps/*.py --report report.json` 重新生成。

## 发现并修复的问题

混合模式本身在这些程序上没有出错，问题都出在原生转换的部分，而且都是编译失败，没有出现运行结果不同的情况。之前语料只用支持的子集，没有覆盖到这些写法。

1. 脚本文件名不是 Rust 标识符（`_sysconfigdata__linux_x86_64-linux-gnu.py`，应用里常见的 `my-tool.py`）时，生成的 `mod` 名不合法，rustfmt 报错。现在把非标识符字符换成 `_`（`tests/test_cli.py`）
2. `*x`、`**x` 实参是之后还要用的局部变量时，`rt::call!` 把它 move 掉了，编译失败（`types.prepare_class`）。宏改成借用（`tests/cases/star_args_locals.py`）
3. 参数和函数同名（`timeit.repeat(..., repeat=...)`）时，`rt::function!` 生成的闭包里参数遮蔽了函数本身，编译失败。宏在绑定参数之前先取出函数（`tests/cases/param_shadows_function.py`）
4. 推导式里的 `:=` 在模块级绑定的是全局变量，绑定分析没有计入。以前推导式不支持，所以没有暴露；混合模式下原生代码可能把这个名字当成 builtin（`tests/cases/mixed_mode.py`）

## 哪些特性挡住了函数

`functions` 是该特性出现在其中的函数个数（类出现时算类的所有方法），`only` 是只被它挡住的函数个数，`statements` 是 `def` 以外的顶层语句个数，`occurrences` 是出现次数。

6 个工具合计：

| 特性 | functions | only | statements | occurrences |
|---|---|---|---|---|
| class definition | 81 | 50 | 28 | 28 |
| raise | 22 | 7 | 1 | 39 |
| try statement | 20 | 4 | 1 | 29 |
| list comprehension | 17 | 5 | 2 | 22 |
| generator expression | 12 | 2 | 0 | 16 |
| yield | 10 | 2 | 0 | 13 |
| with statement | 9 | 1 | 0 | 9 |
| assignment expression | 5 | 4 | 0 | 6 |
| 模块级条件定义 | 2 | 2 | 1 | 2 |
| yield from | 2 | 0 | 0 | 4 |
| dict comprehension | 1 | 0 | 0 | 1 |
| star import | 0 | 0 | 2 | 2 |

标准库 154 个模块合计（6466 个函数，原生 893 个，14%；M3d 之后的数字）：

| 特性 | functions | only | statements | occurrences |
|---|---|---|---|---|
| class definition | 4792 | 3145 | 719 | 747 |
| raise | 1297 | 200 | 6 | 2236 |
| try statement | 953 | 137 | 85 | 1424 |
| with statement | 245 | 29 | 2 | 276 |
| list comprehension | 201 | 22 | 14 | 274 |
| generator expression | 163 | 27 | 8 | 223 |
| async function | 149 | 1 | 9 | 25 |
| yield | 121 | 19 | 1 | 199 |
| 模块级条件定义 | 74 | 33 | 44 | 71 |
| assignment expression | 55 | 11 | 1 | 60 |
| unpacking in a display | 43 | 0 | 2 | 52 |
| yield from | 28 | 0 | 0 | 49 |
| dict comprehension | 23 | 3 | 10 | 39 |
| match statement | 17 | 2 | 0 | 18 |

## M3d 的变化

M3d 把模块级的 `if`/`for`/`while` 拆开（design.md §13）：只有体里的某些语句不支持时，只保留这些语句，其余仍是原生代码。原来这种情况整个复合语句保留为 Python，标准库里有 54 处，其中 42 处是 `if` 里的 `def`，48 个 `if __name__ == "__main__":` 里有 8 个受影响。

| | 函数 | 顶层语句 | 行数 |
|---|---|---|---|
| 6 个工具，M3c | 38/164（23.2%） | 137/169（81.1%） | 751/2992（25.1%） |
| 6 个工具，M3d | 38/164（23.2%） | 139/169（82.2%） | 759/2992（25.4%） |
| 标准库，M3c | 893/6466（13.8%） | 2643/3517（75.1%） | 23057/122951（18.8%） |
| 标准库，M3d | 893/6466（13.8%） | 2697/3517（76.7%） | 23540/122951（19.1%） |

- 54 处复合语句全部变成原生的（拆开之后的复合语句算原生的顶层语句），它们里面保留的语句仍按行数算 Python
- 原生函数的个数不变：`if` 里的 `def` 仍然保留为 Python（以后可以原生转换成不同名字的 Rust 函数，见最后一节）
- 特性表里 `functions` 一列有几处变小、`only` 有几处变大：拆开之前，`if` 里的函数会把同一个 `if` 里别的语句的阻碍也算上，现在只算它自己的
- 每个保留的 `def`/`class` 单独一个 `exec_python!`，不影响这些数字，只影响生成代码的样子
- 下面"各里程碑能带来多少"的预测是 M3c 的数据，没有重算，函数的数字不变，所以结论不受影响

## M4 的变化

M4 支持了 `raise`、`try/except/else/finally`、`with`（design.md §9），`except*`、`async with` 和在 handler 之外还要用 `except .. as` 名字的函数仍然保留为 Python。实际收益和下面 M3c 时的预测一致（6 个工具预测 37%，标准库预测 22%）。

| | 函数 | 顶层语句 | 行数 |
|---|---|---|---|
| 6 个工具，M3d | 38/164（23.2%） | 139/169（82.2%） | 759/2992（25.4%） |
| 6 个工具，M4 | 61/164（37.2%） | 139/169（82.2%） | 1439/2992（48.1%） |
| 标准库，M3d | 893/6466（13.8%） | 2697/3517（76.7%） | 23540/122951（19.1%） |
| 标准库，M4 | 1393/6466（21.5%） | 2750/3517（78.2%） | 39240/122951（31.9%） |

M4 之后各个工具的函数：`stdlib_calendar` 14/72，`stdlib_base64` 21/28，`stdlib_tabnanny` 5/17，`stdlib_tokenize` 11/22，`stdlib_ast` 10/23，`stdlib_json_tool` 0/2。行数涨得比函数多，因为用了异常的函数通常比较长。33 组命令行的输出（包括 traceback 的每一帧）仍然全部与 CPython 一致，标准库 154 个模块转换后 `cargo check` 仍然没有错误和警告。

现在挡住函数的特性（`only` 是只被它挡住的函数个数）：

| 特性 | 6 个工具 functions | only | 标准库 functions | only |
|---|---|---|---|---|
| class definition | 81 | 55 | 4792 | 4304 |
| list comprehension | 17 | 5 | 201 | 49 |
| generator expression | 12 | 4 | 163 | 49 |
| yield | 10 | 4 | 121 | 36 |
| 模块级条件定义 | 2 | 2 | 74 | 59 |
| assignment expression | 5 | 5 | 55 | 14 |
| async function | 0 | 0 | 149 | 1 |
| except-as name used outside its handler | 0 | 0 | 1 | 1 |

类现在几乎是唯一的障碍：标准库里只被类挡住的函数有 4304 个，占剩下的 5073 个的 85%。

## M5 的变化

M5 支持了类（design.md §10），包括方法级的混合模式：类原生创建，只有用了不支持语法的方法或类体语句保留为 Python，在类的命名空间里执行（design.md §13）。实际收益和 M3c 时按方法级算的预测一致（6 个工具预测 71%，标准库预测 88%）。

| | 函数 | 顶层语句 | 行数 |
|---|---|---|---|
| 6 个工具，M4 | 61/164（37.2%） | 139/169（82.2%） | 1439/2992（48.1%） |
| 6 个工具，M5 | 120/164（73.2%） | 167/169（98.8%） | 2202/2992（73.6%） |
| 标准库，M4 | 1393/6466（21.5%） | 2750/3517（78.2%） | 39240/122951（31.9%） |
| 标准库，M5 | 5816/6466（89.9%） | 3447/3517（98.0%） | 102419/122951（83.3%） |

M5 之后各个工具的函数：`stdlib_calendar` 50/72，`stdlib_base64` 21/28，`stdlib_tabnanny` 16/17，`stdlib_tokenize` 18/22，`stdlib_ast` 15/23，`stdlib_json_tool` 0/2。类本身算顶层语句，被拆开的类算原生的，里面保留的方法按函数和行数算 Python。标准库里整个保留为 Python 的类只有 `typing` 里 3 个带类型参数的（`class C[T]:`）。33 组命令行的输出（包括方法的帧）仍然全部与 CPython 一致，标准库 154 个模块转换后 `cargo check` 仍然没有错误和警告，另外 25 个能当脚本运行的模块的输出也和以前一样，只有已经记录的差异（多行语句的源码行、对象地址）。

现在挡住函数的特性：

| 特性 | 6 个工具 functions | only | 标准库 functions | only |
|---|---|---|---|---|
| list comprehension | 13 | 13 | 180 | 147 |
| generator expression | 12 | 12 | 163 | 128 |
| yield | 10 | 9 | 121 | 95 |
| 模块级条件定义 | 2 | 2 | 74 | 59 |
| assignment expression | 5 | 5 | 55 | 43 |
| async function | 0 | 0 | 29 | 10 |
| yield from | 2 | 1 | 28 | 11 |
| unpacking in a display | 0 | 0 | 26 | 16 |
| dict comprehension | 1 | 1 | 23 | 11 |
| match statement | 0 | 0 | 17 | 9 |
| class definition in a function | 0 | 0 | 9 | 6 |
| type parameters | 0 | 0 | 5 | 5 |

剩下的基本是 M8 的范围（推导式、生成器表达式、`:=`）和 generator。函数里的 `class` 仍然让整个函数保留为 Python，标准库里只有 9 个函数。

## 各里程碑能带来多少

JSON 报告列出了每个保留的函数和挡住它的特性，可以算出支持某组特性之后哪些函数会变成原生的（挡住它的特性都已支持）。M4 算 `raise`、`try`、`with`，M5 算类，M8 算推导式、生成器表达式（按全部支持算，实际只计划部分支持）和 `:=`。generator、async、match 仍然保留为 Python。

一个关键的区别是类里有不支持的方法时怎么办。如果仍然按顶层语句保留（现在的粒度），任何一个方法用了不支持的语法，整个类都保留为 Python；如果 M5 支持方法级的混合模式（类原生创建，个别方法从源码编译后放进类的命名空间），其余方法照样是原生的。两种算法都列出来：

| 支持的特性 | 6 个工具（方法级） | 6 个工具（整个类） | 标准库（方法级） | 标准库（整个类） |
|---|---|---|---|---|
| 现在 | 23% | 23% | 14% | 14% |
| M4 | 37% | 37% | 22% | 22% |
| M5 | 54% | 39% | 62% | 23% |
| M8 | 30% | 30% | 15% | 15% |
| M4 + M5 | 71% | 57% | 88% | 55% |
| M4 + M8 | 46% | 46% | 24% | 24% |
| M4 + M5 + M8 | 92% | 87% | 94% | 78% |

## 结论与建议的顺序

- 类是最大的障碍：6 个工具里一半的函数是方法，标准库里四分之三。这些工具偏库的风格，普通应用脚本里函数的比例会高一些，但只要用了 dataclass、自定义异常或者 argparse 的 `Action` 子类，同样会碰到
- 类之后是异常：`raise` 和 `try` 各自出现在一到两成的函数里，只被其中一个挡住的函数不多，所以要作为一组（M4）支持。异常和类大量重叠，M4 加 M5 的效果大于两者各自的收益之和，标准库上尤其明显（+74 个百分点，各自是 +8 和 +48）
- 推导式出现得不少，但单独支持它几乎没有收益（标准库只从 14% 到 15%），因为用推导式的函数几乎总还有类或异常
- M5 如果不做方法级的混合模式，收益会从 +48 个百分点降到 +9 个（标准库），因为大一点的类里总有一个方法用了 generator 或别的暂不支持的语法

建议的顺序：

1. **M4 异常**。范围比 M5 小，又是 M5 的前提（自定义异常类、`__init__` 里的 `raise`、方法里的 `with`），单独做完函数原生率从 23% 到 37%（标准库 14% 到 22%）
2. **M5 类，并把方法级的混合模式列为必做项**：类原生创建，不支持的方法保留为 Python 源码，编译后放进类的命名空间（`__qualname__`、`__module__`、`super()` 的 `__class__` cell 都要对）。M4 加上 M5 到 71%（标准库 88%）
3. **M8 推导式等**。放在最后，做完 M4、M5 之后它把剩下的大部分补上（92%，标准库 94%）。其中 list/dict/set 推导式和容器字面量里的 `*`/`**` 实现成本低，可以在 M5 进行中穿插；生成器表达式只做作为 `any`/`all`/`sum`/`join` 等参数的情况，其余走混合模式
4. 顺带：模块级的条件定义（`if` 里的 `def`，标准库里 74 个函数）。M3d 之后 `if` 本身已经是原生代码，只有 `def` 保留为 Python。以后可以把条件里的每个 `def` 原生转换成不同名字的 Rust 函数，代价不大，可以放进 M5 或 M8

## M8 的变化

M8 支持了 list/set/dict 推导式、生成器表达式（只有一个 `for` 的，真正惰性）和 `:=`（design.md §8），顺带把模块级复合语句里的 `def` 也转换成原生代码。

| | 函数 | 顶层语句 | 行数 |
|---|---|---|---|
| 6 个工具，M5 | 120/164（73.2%） | 167/169（98.8%） | 2202/2992（73.6%） |
| 6 个工具，M8 | 153/164（93.3%） | 167/169（98.8%） | 2858/2992（95.5%） |
| 标准库，M5 | 5816/6466（89.9%） | 3447/3517（98.0%） | 102419/122951（83.3%） |
| 标准库，M8 | 6231/6466（96.4%） | 3487/3517（99.1%） | 114724/122951（93.3%） |

M8 之后各个工具的函数：`stdlib_calendar` 66/72，`stdlib_base64` 28/28，`stdlib_tabnanny` 17/17，`stdlib_tokenize` 20/22，`stdlib_ast` 20/23，`stdlib_json_tool` 2/2。6 个工具里剩下的 11 个保留函数全是 generator，两条保留的顶层语句是 `import *`。33 组命令行的输出仍然全部与 CPython 一致，标准库 154 个模块转换后 `cargo check` 仍然没有错误和警告。另外 31 个不带参数就能当脚本运行、输出确定的标准库模块（`argparse`、`ast`、`dis`、`doctest`、`heapq`、`inspect`、`pickle`、`tarfile`、`tokenize`、`typing` 等）转换后的输出和 CPython 完全一样。

现在挡住函数的特性：

| 特性 | 6 个工具 functions | only | 标准库 functions | only |
|---|---|---|---|---|
| yield | 10 | 9 | 121 | 102 |
| async function | 0 | 0 | 29 | 10 |
| yield from | 2 | 1 | 28 | 12 |
| unpacking in a display | 0 | 0 | 26 | 25 |
| match statement | 0 | 0 | 17 | 15 |
| await | 0 | 0 | 12 | 0 |
| class definition in a function | 0 | 0 | 9 | 9 |
| star import | 0 | 0 | 8 | 2 |
| except-as name used outside its handler | 0 | 0 | 6 | 6 |
| type parameters | 0 | 0 | 5 | 5 |
| eval() without a namespace in a function | 0 | 0 | 4 | 4 |
| generator expression with several for clauses | 0 | 0 | 3 | 3 |
| locals() in a function | 0 | 0 | 3 | 3 |
| call with more than 12 arguments | 0 | 0 | 2 | 2 |
| exec() without a namespace in a function | 0 | 0 | 1 | 1 |

只被 `yield` 或 `yield from` 挡住的函数，标准库里 129 个，6 个工具里 11 个，可行性分析见 design.md §8 的"generator 函数"，结论是暂时保留为 Python。剩下的最便宜的是容器字面量里的 `*`/`**` 展开（只被它挡住的 25 个函数，M5 之后是 16 个，多出来的函数原来同时被推导式挡住）。`except-as name used outside its handler`、`eval()`/`locals()` 这几项是 lowering 时才发现的（design.md §13），以前所在的函数先被推导式挡住，到不了 lowering，所以数字比 M5 时多。

## M13 的变化

M13 原生转换了生成器函数和 `async def`（design.md §8 "协程和生成器"），包括 `await`、`yield`、`yield from`、`async with`、`async for`。异步生成器、异步推导式、推导式里的 `await` 等少数写法仍然保留为 Python。"之前"是 M12 结束时的数字（M8 之后 M10 支持了复数字面量等，所以和上一节的 M8 略有不同）。

| | 函数 | 顶层语句 | 行数 |
|---|---|---|---|
| 6 个工具，M12 | 153/164（93.3%） | 167/169（98.8%） | 2858/2992（95.5%） |
| 6 个工具，M13 | 164/164（100%） | 167/169（98.8%） | 2990/2992（99.9%） |
| 标准库，M12 | 6255/6466（96.7%） | 3488/3517（99.2%） | 115837/122951（94.2%） |
| 标准库，M13 | 6406/6466（99.1%） | 3489/3517（99.2%） | 119229/122951（97.0%） |

6 个工具的函数全部是原生的，剩下的两条顶层语句是 `import *`。标准库 154 个模块转换后 `cargo check` 和 `cargo clippy` 都没有错误，clippy 的警告和以前一样只有文档注释的缩进（docstring 原样变成 `///`）、几处 bool 赋值和单个分支的 `match`，没有出自协程和生成器的。33 组命令行的输出仍然全部与 CPython 一致。

现在挡住函数的特性：

| 特性 | 6 个工具 functions | only | 标准库 functions | only |
|---|---|---|---|---|
| match statement | 0 | 0 | 17 | 17 |
| class definition in a function | 0 | 0 | 9 | 9 |
| star import | 0 | 0 | 8 | 7 |
| except-as name used outside its handler | 0 | 0 | 7 | 7 |
| type parameters | 0 | 0 | 5 | 5 |
| eval() without a namespace in a function | 0 | 0 | 4 | 4 |
| generator expression with several for clauses | 0 | 0 | 3 | 3 |
| locals() in a function | 0 | 0 | 3 | 3 |
| call with more than 12 arguments | 0 | 0 | 2 | 2 |
| asynchronous generator | 0 | 0 | 2 | 1 |
| exec() without a namespace in a function | 0 | 0 | 1 | 1 |
| yield in a lambda | 0 | 0 | 0 | 0 |

M12 时 `yield`（121 个函数，只被它挡住的 102 个）、`async function`（29、10）、`yield from`（28、12）、`await`（12、0）、`async with`（1、0）这几项都没有了。`yield in a lambda` 只出现在 `_collections_abc` 的一条顶层语句里（`generator = type((lambda: (yield))())`），两个异步生成器也在 `_collections_abc` 和 `types` 里，都是为了取得类型对象。剩下最多的是 `match`（M14）。

