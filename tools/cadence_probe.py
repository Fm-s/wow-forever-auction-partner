#!/usr/bin/env python3
"""Log Blizzard Forever auction feed Last-Modified values to measure snapshot cadence.

Usage: python3 tools/cadence_probe.py [--hours 24] [--interval 300]
Writes tools/cadence.csv: polled_at_utc,house,feed,method,status,last_modified_utc
"""
import argparse, base64, csv, json, os, sys, time, urllib.error, urllib.parse, urllib.request
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CSV_PATH = os.path.join(ROOT, "tools", "cadence.csv")
HOUSES = (110, 113, 116)
FEEDS = ("items", "commodities")
API = "https://us.api.blizzard.com"


def read_env():
    env = {}
    with open(os.path.join(ROOT, ".env")) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                k, v = line.split("=", 1)
                env[k.strip()] = v.strip().strip('"')
    return env


def get_token(env):
    auth = base64.b64encode(f"{env['BLIZZARD_CLIENT_ID']}:{env['BLIZZARD_CLIENT_SECRET']}".encode()).decode()
    req = urllib.request.Request("https://oauth.battle.net/token", data=b"grant_type=client_credentials",
                                 headers={"Authorization": f"Basic {auth}"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)["access_token"]


def feed_url(house, feed):
    suffix = "/commodities" if feed == "commodities" else ""
    return f"{API}/data/wow/ruleset/1/auctions/{house}{suffix}?namespace=dynamic-forever-us&locale=en_US"


def request(url, token, method, ims):
    headers = {"Authorization": f"Bearer {token}", "Accept-Encoding": "gzip"}
    if ims:
        headers["If-Modified-Since"] = ims
    req = urllib.request.Request(url, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            if method == "GET":
                while r.read(1 << 20):
                    pass
            return r.status, r.headers.get("Last-Modified")
    except urllib.error.HTTPError as e:
        return e.code, e.headers.get("Last-Modified") if e.headers else None
    except (urllib.error.URLError, OSError) as e:
        print(f"network error: {e}", file=sys.stderr, flush=True)
        return "neterr", None


def to_utc(http_date):
    return parsedate_to_datetime(http_date).astimezone(timezone.utc).isoformat() if http_date else ""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--hours", type=float, default=24)
    ap.add_argument("--interval", type=int, default=300)
    args = ap.parse_args()
    env = read_env()
    token = None
    while token is None:
        try:
            token = get_token(env)
        except (urllib.error.URLError, OSError) as e:
            print(f"token error: {e}", file=sys.stderr, flush=True)
            time.sleep(60)
    last = {}
    method = {}
    new_file = not os.path.exists(CSV_PATH)
    end = time.time() + args.hours * 3600
    with open(CSV_PATH, "a", newline="") as f:
        w = csv.writer(f)
        if new_file:
            w.writerow(["polled_at_utc", "house", "feed", "method", "status", "last_modified_utc"])
        while time.time() < end:
            for house in HOUSES:
                for feed in FEEDS:
                    key = (house, feed)
                    m = method.get(key, "HEAD")
                    status, lm = request(feed_url(house, feed), token, m, last.get(key))
                    if status == 401:
                        try:
                            token = get_token(env)
                        except (urllib.error.URLError, OSError):
                            pass
                        status, lm = request(feed_url(house, feed), token, m, last.get(key))
                    if m == "HEAD" and status not in (200, 304):
                        method[key] = m = "GET"
                        status, lm = request(feed_url(house, feed), token, m, last.get(key))
                    if lm:
                        last[key] = lm
                    w.writerow([datetime.now(timezone.utc).isoformat(), house, feed, m, status, to_utc(lm)])
            f.flush()
            time.sleep(args.interval)


if __name__ == "__main__":
    sys.exit(main())
