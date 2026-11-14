#!/usr/bin/env python3
"""Summarize tools/cadence.csv: per feed, intervals between distinct Last-Modified values."""
import csv, os, statistics
from collections import defaultdict
from datetime import datetime

CSV_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cadence.csv")


def main():
    stamps = defaultdict(list)
    statuses = defaultdict(lambda: defaultdict(int))
    with open(CSV_PATH) as f:
        for row in csv.DictReader(f):
            key = (row["house"], row["feed"])
            statuses[key][row["status"]] += 1
            if row["last_modified_utc"]:
                t = datetime.fromisoformat(row["last_modified_utc"]).timestamp()
                if not stamps[key] or stamps[key][-1] != t:
                    stamps[key].append(t)
    for key in sorted(statuses):
        ts = sorted(set(stamps[key]))
        iv = [b - a for a, b in zip(ts, ts[1:])]
        st = dict(statuses[key])
        if iv:
            iv.sort()
            p90 = iv[min(len(iv) - 1, int(0.9 * len(iv)))]
            print(f"house {key[0]} {key[1]:11} snapshots={len(ts)} median={statistics.median(iv):.0f}s p90={p90:.0f}s max={max(iv):.0f}s statuses={st}")
        else:
            print(f"house {key[0]} {key[1]:11} snapshots={len(ts)} (not enough data) statuses={st}")
    any304 = any("304" in s for s in statuses.values())
    print("If-Modified-Since honored (any 304):", any304)


if __name__ == "__main__":
    main()
