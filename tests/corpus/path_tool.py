"""Pure path manipulation utility: normalize, split and relativize POSIX paths."""

import posixpath

PATHS = [
    "/usr/local/lib/python3.12/site-packages/foo/__init__.py",
    "/home/user/../user/projects/./steelix/src/main.rs",
    "relative/dir/file.tar.gz",
    "/etc/nginx/sites-enabled/default",
    "//double//slashes///here/",
    "no_extension",
]


def describe(path):
    norm = posixpath.normpath(path)
    head, tail = posixpath.split(norm)
    root, ext = posixpath.splitext(tail)
    return {
        "norm": norm,
        "dir": head,
        "base": tail,
        "stem": root,
        "ext": ext or "(none)",
        "abs": posixpath.isabs(path),
        "depth": len(norm.strip("/").split("/")),
    }


def all_suffixes(name):
    parts = name.split(".")
    if len(parts) <= 1:
        return []
    out = []
    for p in parts[1:]:
        out.append("." + p)
    return out


def common_prefix(paths):
    abs_paths = []
    for p in paths:
        if posixpath.isabs(p):
            abs_paths.append(posixpath.normpath(p))
    if not abs_paths:
        return None
    return posixpath.commonpath(abs_paths)


def change_ext(path, new_ext):
    stem, _ = posixpath.splitext(path)
    return stem + new_ext


def main():
    for path in PATHS:
        info = describe(path)
        print(f"{path}")
        for key in ("norm", "dir", "base", "stem", "ext"):
            print(f"    {key:>5}: {info[key]}")
        flag = "absolute" if info["abs"] else "relative"
        print(f"    {flag}, depth {info['depth']}")
        sufs = all_suffixes(info["base"])
        if sufs:
            print("    suffixes:", ", ".join(sufs))
    print("common:", common_prefix(PATHS[:2]))
    print("common all:", common_prefix(PATHS))
    print(posixpath.relpath("/usr/local/bin/python", "/usr/share/doc"))
    print(posixpath.join("a", "b", "/c", "d"))
    print(change_ext("archive/data.json", ".yaml"))
    segments = PATHS[0].split("/")
    print(segments[-1], segments[-2], segments[1:3])
    print("/".join(segments[:-3]))


main()
