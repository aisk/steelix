"""Aggregate a sales CSV (embedded) into per-region and per-product reports."""

import csv
import io
import statistics

DATA = """region,product,units,price
north,widget,12,2.50
south,gadget,3,10.00
north,gadget,7,10.00
east,widget,20,2.25
south,widget,5,2.50
east,gizmo,1,99.99
north,gizmo,2,95.00
west,widget,0,2.50
west,gadget,11,9.50
"""


def load_rows(text):
    reader = csv.DictReader(io.StringIO(text))
    rows = []
    for rec in reader:
        rec["units"] = int(rec["units"])
        rec["price"] = float(rec["price"])
        rows.append(rec)
    return rows


def group_totals(rows, key):
    totals = {}
    for row in rows:
        k = row[key]
        if k not in totals:
            totals[k] = 0.0
        totals[k] += row["units"] * row["price"]
    return totals


def format_table(title, totals, *, currency="$"):
    lines = [title, "-" * len(title)]
    grand = 0.0
    for name, amount in sorted(totals.items(), key=lambda kv: kv[1], reverse=True):
        lines.append(f"{name:<10}{currency}{amount:>10.2f}")
        grand += amount
    lines.append(f"{'TOTAL':<10}{currency}{grand:>10.2f}")
    return "\n".join(lines)


def write_summary(rows):
    out = io.StringIO()
    writer = csv.writer(out, lineterminator="\n")
    writer.writerow(["product", "orders", "units"])
    seen = {}
    for row in rows:
        prod = row["product"]
        orders, units = seen.get(prod, (0, 0))
        seen[prod] = (orders + 1, units + row["units"])
    for prod in sorted(seen):
        orders, units = seen[prod]
        writer.writerow([prod, orders, units])
    return out.getvalue()


def main():
    rows = load_rows(DATA)
    active = []
    for row in rows:
        if row["units"] == 0:
            print(f"skipping empty order in {row['region']}")
            continue
        active.append(row)
    print(format_table("By region", group_totals(active, "region")))
    print()
    print(format_table("By product", group_totals(active, "product"), currency="EUR "))
    print()
    print(write_summary(active), end="")
    units = []
    for row in active:
        units.append(row["units"])
    print(f"mean units: {statistics.mean(units):.2f}")
    print(f"median units: {statistics.median(units)}")
    biggest = max(active, key=lambda r: r["units"] * r["price"])
    print(f"biggest order: {biggest['region']}/{biggest['product']}")


if __name__ == "__main__":
    main()
