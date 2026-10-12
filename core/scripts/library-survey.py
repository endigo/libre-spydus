#!/usr/bin/env -S python3 -I
"""Multi-library compatibility survey: the Rust core (`spydus-cli`) against a deterministic,
region-stratified sample of production Spydus libraries from Civica's directory.

    python3 -I core/scripts/library-survey.py            # from anywhere; needs cargo + python3
    python3 -I core/scripts/library-survey.py --sample AU=10,NZ=10 --json out.json
    python3 -I core/scripts/library-survey.py --dry-run   # print the sample, contact no library

For each sampled library it runs, through `spydus-cli` with SPYDUS_ORIGIN=<library origin>:
`search "<query>"`, then `availability` and `record` on the first format (in result order)
that has an availabilityUrl. A library whose catalogue has nothing for the query (an academic
or corporate collection without Harry Potter) is searched once more with --fallback-query,
marked `*` in the matrix, so the drill-down still runs. It prints a per-library matrix and a
per-region summary, and checks each library against the regional expectation:

  AU / NZ / US / OTHER   search -> availability -> record all succeed (end to end)
  UK                     HumanVerificationRequired (AWS WAF captcha), answered quickly
  SG                     a Network error within the connect timeout (servers unreachable)

The exit code is 1 if any library missed its expectation (live data: a library can be down),
2 on a setup error.

Sample: per region, the production libraries sorted by name are cut into k equal strata and
the middle library of each is taken, so the sample is spread across the alphabet and the same
on every run for the same directory. Test instances (isTest) are never sampled.

Safety (docs/rust-core-port.md section 3): public GETs only, one request at a time per library
and at most --jobs (<= 6) libraries at once. No login, no session, no POST. A verification
cookie is never used: SPYDUS_VERIFICATION_COOKIE is removed from the CLI's environment.
"""

import argparse
import json
import os
import statistics
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

CORE_DIR = Path(__file__).resolve().parent.parent
REGIONS = ["AU", "NZ", "US", "UK", "SG", "OTHER"]
DEFAULT_SAMPLE = "AU=6,NZ=4,US=4,UK=4,SG=3"
MAX_JOBS = 6
# The core's connect timeout (config::CONNECT_TIMEOUT) and per-request timeout.
CONNECT_TIMEOUT_S = 10
CLI_TIMEOUT_S = 150
# An AWS WAF answer must come back quickly: it is one request, never a parse of the page.
FAST_ERROR_S = 5
# Regions expected to fail, and how.
EXPECTED = {"UK": "human-verification", "SG": "network"}


def main():
    args = parse_args()
    cli = build_cli()
    libraries = load_directory(cli)
    sample = stratified_sample(libraries, args.sample)
    if not sample:
        sys.exit("survey: the sample is empty")

    print(f"survey: query {args.query!r}; directory {len(libraries)} production libraries "
          f"({region_counts(libraries)}); sample {len(sample)} ({region_counts(sample)}); "
          f"jobs {args.jobs}")
    if args.dry_run:
        for lib in sample:
            print(f"  {lib['region']:<6} {lib['name']} ({lib['origin']})")
        return
    started = time.monotonic()
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        results = list(pool.map(
            lambda lib: survey(cli, lib, args.query, args.fallback_query), sample))
    for result in results:
        result["verdict"] = verdict(result)

    print_matrix(results)
    print(f"  * no records for {args.query!r}; searched {args.fallback_query!r} instead")
    print()
    print_summary(results)
    unexpected = [r for r in results if not r["verdict"][0]]
    print()
    print(f"survey: {len(results) - len(unexpected)}/{len(results)} libraries as expected "
          f"({time.monotonic() - started:.0f}s)")
    for r in unexpected:
        print(f"  UNEXPECTED {r['region']} {r['name']} ({r['origin']}): {r['verdict'][1]}")
    if args.json:
        Path(args.json).write_text(json.dumps(results, indent=2) + "\n")
        print(f"survey: raw results written to {args.json}")
    sys.exit(1 if unexpected else 0)


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--query", default="harry potter")
    parser.add_argument("--fallback-query", default="history",
                        help="searched when --query finds no records (default %(default)s)")
    parser.add_argument("--sample", default=DEFAULT_SAMPLE,
                        help=f"REGION=COUNT,... (default {DEFAULT_SAMPLE}); "
                             f"regions: {', '.join(REGIONS)}")
    parser.add_argument("--jobs", type=int, default=MAX_JOBS,
                        help=f"libraries surveyed at once (1-{MAX_JOBS})")
    parser.add_argument("--json", metavar="PATH", help="also write the raw results as JSON")
    parser.add_argument("--dry-run", action="store_true",
                        help="print the sample only; no library is contacted")
    args = parser.parse_args()
    if not 1 <= args.jobs <= MAX_JOBS:
        parser.error(f"--jobs must be 1-{MAX_JOBS}")
    try:
        args.sample = {region.strip().upper(): int(count)
                       for region, count in (part.split("=") for part in args.sample.split(","))}
    except ValueError:
        parser.error("--sample must look like AU=6,NZ=4")
    unknown = set(args.sample) - set(REGIONS)
    if unknown:
        parser.error(f"unknown region(s) {', '.join(sorted(unknown))}")
    return args


def build_cli():
    print("survey: building spydus-cli…", file=sys.stderr)
    build = subprocess.run(["cargo", "build", "--quiet", "--bin", "spydus-cli",
                            "--manifest-path", str(CORE_DIR / "Cargo.toml")])
    if build.returncode != 0:
        sys.exit(2)
    return CORE_DIR / "target" / "debug" / "spydus-cli"


def load_directory(cli):
    run = run_cli(cli, None, "libraries")
    if not run["ok"]:
        print(f"survey: spydus-cli libraries failed: {run['error']}", file=sys.stderr)
        sys.exit(2)
    return [lib for lib in run["json"] if not lib["isTest"]]


def stratified_sample(libraries, wanted):
    """Per region: k strata of the name-sorted list, the middle library of each."""
    sample = []
    for region in REGIONS:
        pool = sorted((lib for lib in libraries if lib["region"] == region),
                      key=lambda lib: (lib["name"].lower(), lib["origin"]))
        k = min(wanted.get(region, 0), len(pool))
        sample += [pool[(2 * i + 1) * len(pool) // (2 * k)] for i in range(k)]
    return sample


def run_cli(cli, origin, command, arg=None):
    """One `spydus-cli` call → {ok, json | error, kind, seconds}."""
    env = {k: v for k, v in os.environ.items()
           if k not in ("SPYDUS_ORIGIN", "SPYDUS_VERIFICATION_COOKIE")}
    if origin:
        env["SPYDUS_ORIGIN"] = origin
    argv = [str(cli), command] + ([arg] if arg is not None else [])
    started = time.monotonic()
    try:
        proc = subprocess.run(argv, env=env, capture_output=True, text=True,
                              timeout=CLI_TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return {"ok": False, "kind": "cli-timeout", "error": f"no answer in {CLI_TIMEOUT_S}s",
                "seconds": CLI_TIMEOUT_S}
    seconds = round(time.monotonic() - started, 1)
    if proc.returncode == 0:
        return {"ok": True, "json": json.loads(proc.stdout), "seconds": seconds}
    # "spydus-cli <command>: <SpydusError Display>"
    error = proc.stderr.strip().split(": ", 1)[-1]
    return {"ok": False, "kind": error_kind(error), "error": error, "seconds": seconds}


def error_kind(error):
    if error == "human verification required":
        return "human-verification"
    if error.startswith("network error: connect timed out"):
        return "network-timeout"
    if error.startswith("network error") and "dns error" in error:
        return "network-dns"
    for prefix, kind in [("network error", "network"), ("upstream error", "upstream"),
                         ("parse error", "parse"), ("forbidden origin", "origin-forbidden")]:
        if error.startswith(prefix):
            return kind
    return "other"


def survey(cli, lib, query, fallback_query):
    result = {key: lib[key] for key in ("name", "origin", "region", "country")}
    search = run_cli(cli, lib["origin"], "search", query)
    result["search"] = summarize_search(search)
    if search["ok"] and not search["json"]["records"]:
        result["mainSearch"] = result["search"]
        search = run_cli(cli, lib["origin"], "search", fallback_query)
        result["search"] = summarize_search(search)
    if not search["ok"]:
        return result
    url = next((fmt["availabilityUrl"] for record in search["json"]["records"]
                for fmt in record["formats"] if fmt.get("availabilityUrl")), None)
    result["availabilityUrl"] = url
    if url:
        result["availability"] = summarize_availability(run_cli(cli, lib["origin"],
                                                                "availability", url))
        result["record"] = summarize_record(run_cli(cli, lib["origin"], "record", url))
    return result


def region_counts(libraries):
    counts = {region: sum(1 for lib in libraries if lib["region"] == region) for region in REGIONS}
    return " ".join(f"{region} {n}" for region, n in counts.items() if n)


def strip(run):
    return {k: v for k, v in run.items() if k != "json"}


def summarize_search(run):
    out = strip(run)
    if run["ok"]:
        records = run["json"]["records"]
        share = lambda pred: round(100 * sum(1 for r in records if pred(r)) / len(records)) \
            if records else None
        out.update(
            total=run["json"]["totalResults"],
            records=len(records),
            title=share(lambda r: r.get("title")),
            author=share(lambda r: r.get("author")),
            formats=share(lambda r: r["formats"]),
            availUrl=share(lambda r: any(f.get("availabilityUrl") for f in r["formats"])),
            nextPage=run["json"]["nextPageUrl"] is not None,
        )
    return out


def summarize_availability(run):
    out = strip(run)
    if run["ok"]:
        a = run["json"]
        out.update(counts=a["counts"], copies=a["copies"], reserved=a["reserved"],
                   title=a["title"] is not None, recordUrl=a["recordUrl"] is not None)
    return out


def summarize_record(run):
    out = strip(run)
    if run["ok"]:
        r = run["json"]
        out.update(title=r["title"] is not None, details=len(r["details"]),
                   brn=r["brn"] is not None)
    return out


def verdict(result):
    """(as expected?, why not)."""
    region, search = result["region"], result["search"]
    expected = EXPECTED.get(region)
    if expected == "human-verification":
        if search["ok"] or search["kind"] != expected:
            return False, f"expected human verification, got {describe(search)}"
        if search["seconds"] > FAST_ERROR_S:
            return False, f"human verification took {search['seconds']}s"
        return True, ""
    if expected == "network":
        if search["ok"] or not search["kind"].startswith("network"):
            return False, f"expected a network error, got {describe(search)}"
        if search["seconds"] > CONNECT_TIMEOUT_S + 3:
            return False, f"the network error took {search['seconds']}s"
        return True, ""
    if not search["ok"]:
        return False, f"search {describe(search)}"
    if not search["records"]:
        return False, "search returned no records"
    if not result.get("availabilityUrl"):
        return False, "no format has an availabilityUrl"
    for step in ("availability", "record"):
        if not result[step]["ok"]:
            return False, f"{step} {describe(result[step])}"
    if not result["record"]["title"]:
        return False, "record has no title"
    return True, ""


def describe(run):
    return "ok" if run["ok"] else f"{run['kind']}: {run['error']}"


# --- output ----------------------------------------------------------------------------------

COLUMNS = [("region", 6), ("library", 30), ("search", 24), ("total", 6), ("recs", 4),
           ("title", 5), ("author", 6), ("fmts", 5), ("avUrl", 5), ("next", 4),
           ("availability", 20), ("record", 16), ("ok", 3)]


def print_matrix(results):
    print()
    print(row([name for name, _ in COLUMNS]))
    print(row(["-" * width for _, width in COLUMNS]))
    for r in sorted(results, key=lambda r: (REGIONS.index(r["region"]), r["name"].lower())):
        s = r["search"]
        cells = [r["region"], r["name"], status(s) + (" *" if "mainSearch" in r else "")]
        if s["ok"]:
            cells += [dash(s["total"]), s["records"], pct(s["title"]), pct(s["author"]),
                      pct(s["formats"]), pct(s["availUrl"]), "yes" if s["nextPage"] else "no"]
            if r.get("availabilityUrl"):
                a, rec = r["availability"], r["record"]
                cells += [availability_cell(a), record_cell(rec)]
            else:
                cells += ["- (no availUrl)", "-"]
        else:
            cells += ["-"] * 9
        cells.append("yes" if r["verdict"][0] else "NO")
        print(row(cells))


def print_summary(results):
    print("per-region summary")
    header = ["region", "libs", "search ok", "with recs", "fallback", "availability ok",
              "record ok", "as expected", "search s (median/max)", "errors"]
    widths = [6, 4, 9, 9, 8, 15, 9, 11, 21, 0]
    print(row(header, widths))
    print(row(["-" * w if w else "------" for w in widths], widths))
    for region in REGIONS:
        rs = [r for r in results if r["region"] == region]
        if not rs:
            continue
        searched = [r for r in rs if r["search"]["ok"]]
        drilled = [r for r in searched if r.get("availabilityUrl")]
        seconds = [r["search"]["seconds"] for r in rs]
        errors = {}
        for r in rs:
            for step in ("search", "availability", "record"):
                run = r.get(step)
                if run and not run["ok"]:
                    key = f"{step}:{run['kind']}"
                    errors[key] = errors.get(key, 0) + 1
        print(row([
            region, len(rs),
            f"{len(searched)}/{len(rs)}",
            f"{sum(1 for r in searched if r['search']['records'])}/{len(rs)}",
            sum(1 for r in rs if "mainSearch" in r),
            f"{sum(1 for r in drilled if r['availability']['ok'])}/{len(rs)}",
            f"{sum(1 for r in drilled if r['record']['ok'])}/{len(rs)}",
            f"{sum(1 for r in rs if r['verdict'][0])}/{len(rs)}",
            f"{statistics.median(seconds):.1f}/{max(seconds):.1f}",
            ", ".join(f"{k} x{n}" for k, n in sorted(errors.items())) or "-",
        ], widths))


def status(run):
    if run["ok"]:
        return f"ok {run['seconds']}s"
    return f"{run['kind']} {run['seconds']}s"


def availability_cell(run):
    if not run["ok"]:
        return status(run)
    if run["copies"] is not None:
        return f"ok copies={run['copies']}"
    return f"ok {len(run['counts'])} counts"


def record_cell(run):
    if not run["ok"]:
        return status(run)
    return f"ok {run['details']} rows" if run["title"] else "ok NO TITLE"


def pct(value):
    return "-" if value is None else f"{value}%"


def dash(value):
    return "-" if value is None else value


def row(cells, widths=None):
    widths = widths or [width for _, width in COLUMNS]
    out = []
    for cell, width in zip(cells, widths):
        text = str(cell)
        if width and len(text) > width:
            text = text[:width - 1] + "…"
        out.append(text.ljust(width) if width else text)
    return "  ".join(out).rstrip()


if __name__ == "__main__":
    main()
