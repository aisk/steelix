"""A todo-list CLI that parses sys.argv with argparse, falling back to a demo command list."""

import argparse
import sys

DEMO = [
    ["add", "buy milk", "--priority", "2"],
    ["add", "write report", "--priority", "1", "--tag", "work"],
    ["add", "call mom"],
    ["done", "2"],
    ["add", "fix bike", "--tag", "home", "--priority", "3"],
    ["list", "--all"],
    ["list", "--tag", "home"],
]


def build_parser():
    parser = argparse.ArgumentParser(prog="todo")
    sub = parser.add_subparsers(dest="cmd")
    add = sub.add_parser("add")
    add.add_argument("title")
    add.add_argument("--priority", type=int, default=5)
    add.add_argument("--tag", default=None)
    done = sub.add_parser("done")
    done.add_argument("ident", type=int)
    lst = sub.add_parser("list")
    lst.add_argument("--all", action="store_true")
    lst.add_argument("--tag")
    return parser


def make_store():
    items = []
    next_id = 1

    def add(title, priority=5, tag=None):
        nonlocal next_id
        item = {"id": next_id, "title": title, "priority": priority, "tag": tag}
        item["done"] = False
        items.append(item)
        next_id += 1
        return next_id - 1

    def mark_done(ident):
        for item in items:
            if item["id"] == ident:
                item["done"] = True
                return True
        return False

    def query(show_all=False, tag=None):
        result = []
        for item in sorted(items, key=lambda it: (it["priority"], it["id"])):
            if item["done"] and not show_all:
                continue
            if tag is not None and item["tag"] != tag:
                continue
            result.append(item)
        return result

    return add, mark_done, query


def run(commands):
    parser = build_parser()
    add, mark_done, query = make_store()
    for argv in commands:
        ns = parser.parse_args(argv)
        if ns.cmd == "add":
            ident = add(ns.title, priority=ns.priority, tag=ns.tag)
            print(f"added #{ident}: {ns.title}")
        elif ns.cmd == "done":
            ok = mark_done(ns.ident)
            print(f"done #{ns.ident}" if ok else f"no such item #{ns.ident}")
        elif ns.cmd == "list":
            rows = query(show_all=ns.all, tag=ns.tag)
            print(f"{len(rows)} item(s):")
            for item in rows:
                mark = "x" if item["done"] else " "
                tag = f" [{item['tag']}]" if item["tag"] else ""
                print(f"  [{mark}] {item['id']:>2} p{item['priority']} {item['title']}{tag}")


def main():
    args = sys.argv[1:]
    if args:
        run([args])
    else:
        run(DEMO)


if __name__ == "__main__":
    main()
