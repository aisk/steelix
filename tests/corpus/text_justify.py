"""Text formatter: wraps, fully justifies, centers and indents paragraphs."""

import textwrap

PARAGRAPHS = [
    "The quick brown fox jumps over the lazy dog. Pack my box with five dozen "
    "liquor jugs. How vexingly quick daft zebras jump!",
    "Sphinx of black quartz, judge my vow. The five boxing wizards jump quickly.",
    "Short.",
]


def justify_line(words, width):
    if len(words) == 1:
        return words[0].ljust(width)
    total = 0
    for w in words:
        total += len(w)
    gaps = len(words) - 1
    spaces, extra = divmod(width - total, gaps)
    out = ""
    for i, w in enumerate(words[:-1]):
        out += w + " " * (spaces + (1 if i < extra else 0))
    return out + words[-1]


def justify(text, width=30):
    words = text.split()
    lines = []
    current = []
    length = 0
    for word in words:
        if current and length + len(current) + len(word) > width:
            lines.append(justify_line(current, width))
            current, length = [], 0
        current.append(word)
        length += len(word)
    if current:
        lines.append(" ".join(current).ljust(width))
    return lines


def boxed(lines, pad=1):
    inner = max(map(len, lines)) + 2 * pad
    top = "+" + "-" * inner + "+"
    body = [top]
    for line in lines:
        body.append("|" + " " * pad + line.ljust(inner - 2 * pad) + " " * pad + "|")
    body.append(top)
    return body


def main():
    width = 28
    for n, para in enumerate(PARAGRAPHS):
        print(f"== paragraph {n + 1} ({len(para.split())} words) ==")
        for line in boxed(justify(para, width=width)):
            print(line)
    print(textwrap.fill(PARAGRAPHS[1], width=20, initial_indent="> ", subsequent_indent="  "))
    print(textwrap.indent("alpha\nbeta\n\ngamma", "  | "))
    print(textwrap.shorten(PARAGRAPHS[0], width=40, placeholder=" [...]"))
    print(
        textwrap.dedent("""
        def f():
            return 1
    """).strip()
    )
    title = "Report"
    print(f"[{title:^20}]")
    print(f"[{title:*>12}]")
    print(title.center(16, "=").upper())


if __name__ == "__main__":
    main()
