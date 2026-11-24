#!/usr/bin/env python3
"""Download real CC/CC0/public-domain photos and build e2e folder sets.

Idempotent: files land in a cache keyed by checksum. Re-runs skip complete
objects. Details go to the cache log. Stdout is timestamped progress
(counts and byte totals only) plus a final SUMMARY line.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import html
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

UA = "LixaGalleryE2EHarness/1.0 (desktop app performance test; local; +https://github.com/santhosh-chinnasamy/lixa-gallery)"
COMMONS = "https://commons.wikimedia.org/w/api.php"
RAW_ROOTS = [
    "https://raw.pixls.us/",
    "https://raw.pixls.us/data/",
    "https://raw.pixls.us/data/Canon/",
    "https://raw.pixls.us/data/Nikon/",
    "https://raw.pixls.us/data/Sony/",
    "https://raw.pixls.us/data/Fujifilm/",
    "https://raw.pixls.us/data/Olympus/",
    "https://raw.pixls.us/data/Panasonic/",
    "https://raw.pixls.us/data/Hasselblad/",
    "https://raw.pixls.us/data/Leica/",
]
RAW_EXTS = {".cr2", ".cr3", ".nef", ".arw", ".raf", ".orf", ".rw2", ".dng"}
JPEG_TARGET = 200
RAW_TARGET = 60
JPEG_MIN = 12_000_000
JPEG_MAX = 48_000_000
RAW_MIN = 8_000_000
RAW_MAX = 80_000_000

LICENSE_OK = re.compile(
    r"(public\s*domain|\bcc0\b|\bcc[-\s]?by\b|creative\s+commons|\bpd\b|no\s*known\s*copyright|cc-zero)",
    re.I,
)
LICENSE_BAD = re.compile(
    r"(all rights reserved|non-?free|copyrighted free use|gfdl|attribution-noderivs only)",
    re.I,
)


def log(cache: Path, message: str) -> None:
    line = f"{time.strftime('%H:%M:%S')} {message}"
    cache.mkdir(parents=True, exist_ok=True)
    with (cache / "fetch.log").open("a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    # Terminal progress stays free of URLs and file lists.
    if "http://" in message or "https://" in message or "File:" in message:
        return
    print(line, flush=True)


def request(url: str, data: bytes | None = None, timeout: int = 90) -> bytes:
    headers = {"User-Agent": UA, "Accept": "*/*"}
    last_error: Exception | None = None
    for attempt in range(5):
        try:
            req = urllib.request.Request(url, data=data, headers=headers)
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return resp.read()
        except urllib.error.HTTPError as err:
            last_error = err
            if err.code not in {429, 500, 502, 503, 504}:
                raise
            time.sleep(2 + attempt * 3)
        except Exception as err:  # noqa: BLE001 - network retry
            last_error = err
            time.sleep(2 + attempt * 2)
    raise RuntimeError(f"GET failed {url}: {last_error}")


def commons_api(params: dict) -> dict:
    params = {
        **params,
        "format": "json",
        "formatversion": "2",
        "maxlag": "5",
        "origin": "*",
    }
    url = COMMONS + "?" + urllib.parse.urlencode(params)
    body = request(url, timeout=120)
    payload = json.loads(body.decode("utf-8", "replace"))
    if "error" in payload:
        code = payload["error"].get("code")
        if code == "maxlag":
            time.sleep(5)
            return commons_api(params)
        raise RuntimeError(str(payload["error"]))
    return payload


def strip_html(value: str) -> str:
    text = re.sub(r"<[^>]+>", " ", value or "")
    return re.sub(r"\s+", " ", html.unescape(text)).strip()


def license_ok(short: str, usage: str) -> bool:
    blob = f"{short} {usage}"
    if not LICENSE_OK.search(blob):
        return False
    if LICENSE_BAD.search(blob) and not LICENSE_OK.search(short or ""):
        return False
    return True


def load_manifest(path: Path) -> dict[str, dict]:
    rows: dict[str, dict] = {}
    if not path.exists():
        return rows
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            rows[row["source_url"]] = row
    return rows


def save_manifest(path: Path, rows: dict[str, dict]) -> None:
    fields = [
        "sha1",
        "kind",
        "bytes",
        "license",
        "author",
        "source_url",
        "cache_rel",
        "original_name",
        "ext",
    ]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows.values():
            writer.writerow({key: row.get(key, "") for key in fields})


def sha1_file(path: Path) -> str:
    digest = hashlib.sha1()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download_file(url: str, dest: Path, expected_sha1: str | None) -> int:
    if dest.exists() and dest.stat().st_size > 0:
        if expected_sha1 is None or sha1_file(dest) == expected_sha1:
            return dest.stat().st_size
    dest.parent.mkdir(parents=True, exist_ok=True)
    part = dest.with_suffix(dest.suffix + ".part")
    last_error: Exception | None = None
    for attempt in range(6):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=240) as resp, part.open("wb") as handle:
                got = 0
                reported = 0
                while True:
                    chunk = resp.read(1024 * 1024)
                    if not chunk:
                        break
                    handle.write(chunk)
                    got += len(chunk)
                    if got - reported >= 8_000_000:
                        reported = got
                        print(
                            f"{time.strftime('%H:%M:%S')} downloading {got // 1_000_000}MB",
                            flush=True,
                        )
            break
        except urllib.error.HTTPError as err:
            last_error = err
            part.unlink(missing_ok=True)
            if err.code == 429:
                wait = 20 + attempt * 15
                print(
                    f"{time.strftime('%H:%M:%S')} rate limited, sleeping {wait}s",
                    flush=True,
                )
                time.sleep(wait)
                continue
            if err.code not in {500, 502, 503, 504}:
                raise
            time.sleep(2 + attempt * 3)
        except Exception as err:  # noqa: BLE001
            last_error = err
            part.unlink(missing_ok=True)
            time.sleep(2 + attempt * 2)
    else:
        raise RuntimeError(f"download failed: {last_error}")
    if not part.exists():
        raise RuntimeError(f"download failed: {last_error}")
    if expected_sha1 is not None:
        got_sha = sha1_file(part)
        if got_sha != expected_sha1:
            part.unlink(missing_ok=True)
            raise RuntimeError("checksum mismatch")
    os.replace(part, dest)
    return dest.stat().st_size


def collect_jpeg_candidates(cache: Path, limit: int) -> list[dict]:
    """Return licensed large JPEG candidates. Does not download bytes."""
    found: list[dict] = []
    seen: set[str] = set()

    def consider(title: str, info: dict) -> None:
        if len(found) >= limit:
            return
        url = info.get("url") or ""
        size = int(info.get("size") or 0)
        mime = (info.get("mime") or "").lower()
        if not url or url in seen:
            return
        if "jpeg" not in mime and not url.lower().endswith((".jpg", ".jpeg")):
            return
        if not (JPEG_MIN <= size <= JPEG_MAX):
            return
        meta = info.get("extmetadata") or {}
        short = strip_html((meta.get("LicenseShortName") or {}).get("value", ""))
        usage = strip_html((meta.get("UsageTerms") or {}).get("value", ""))
        artist = strip_html((meta.get("Artist") or {}).get("value", "")) or info.get("user") or ""
        if not license_ok(short, usage):
            return
        seen.add(url)
        name = title.split(":", 1)[-1]
        found.append(
            {
                "title": title,
                "url": url,
                "size": size,
                "sha1": info.get("sha1") or "",
                "license": short or usage,
                "author": artist,
                "original_name": name,
                "ext": ".jpg",
                "kind": "jpeg",
            }
        )

    # Search finds photographs; allimages sweeps the size window.
    searches = [
        "filemime:image/jpeg filew:>4000 fileh:>3000",
        "filemime:image/jpeg incategory:Featured_pictures_on_Wikimedia_Commons",
        "filemime:image/jpeg filetype:bitmap",
    ]
    for query in searches:
        cont: dict = {}
        for _page in range(12):
            if len(found) >= limit:
                break
            params = {
                "action": "query",
                "generator": "search",
                "gsrsearch": query,
                "gsrnamespace": "6",
                "gsrlimit": "50",
                "prop": "imageinfo",
                "iiprop": "url|size|mime|sha1|user|extmetadata",
                "iiextmetadatafilter": "LicenseShortName|UsageTerms|Artist|LicenseUrl",
                **cont,
            }
            payload = commons_api(params)
            for page in payload.get("query", {}).get("pages", []):
                infos = page.get("imageinfo") or []
                if infos:
                    consider(page.get("title") or "", infos[0])
            cont = payload.get("continue") or {}
            if not cont:
                break
            time.sleep(0.3)
        log(cache, f"search candidates={len(found)} query={query[:48]}")

    cont = {}
    for _page in range(20):
        if len(found) >= limit:
            break
        params = {
            "action": "query",
            "list": "allimages",
            "aimime": "image/jpeg",
            "aiminsize": str(JPEG_MIN),
            "aimaxsize": str(JPEG_MAX),
            "aisort": "timestamp",
            "aidir": "descending",
            "aiprop": "url|size|mime|sha1|user|canonicaltitle",
            "ailimit": "50",
            **cont,
        }
        payload = commons_api(params)
        images = payload.get("query", {}).get("allimages", [])
        titles = [img.get("canonicaltitle") or img.get("name") for img in images]
        titles = [f"File:{name}" if name and not str(name).startswith("File:") else name for name in titles]
        titles = [t for t in titles if t]
        for chunk_start in range(0, len(titles), 20):
            chunk = titles[chunk_start : chunk_start + 20]
            if not chunk:
                continue
            info_payload = commons_api(
                {
                    "action": "query",
                    "titles": "|".join(chunk),
                    "prop": "imageinfo",
                    "iiprop": "url|size|mime|sha1|user|extmetadata",
                    "iiextmetadatafilter": "LicenseShortName|UsageTerms|Artist|LicenseUrl",
                }
            )
            for page in info_payload.get("query", {}).get("pages", []):
                infos = page.get("imageinfo") or []
                if infos:
                    consider(page.get("title") or "", infos[0])
            time.sleep(0.2)
        cont = payload.get("continue") or {}
        log(cache, f"allimages candidates={len(found)}")
        if not cont:
            break
        time.sleep(0.3)
    return found


def cache_jpeg(cache: Path, item: dict) -> dict | None:
    sha = item["sha1"] or hashlib.sha1(item["url"].encode()).hexdigest()
    dest = cache / "jpeg" / f"{sha}.jpg"
    try:
        nbytes = download_file(item["url"], dest, item["sha1"] or None)
    except Exception as err:  # noqa: BLE001
        log(cache, f"jpeg fail {err}")
        return None
    if not (JPEG_MIN <= nbytes <= JPEG_MAX + 2_000_000):
        return None
    got = item["sha1"] or sha1_file(dest)
    return {
        "sha1": got,
        "kind": "jpeg",
        "bytes": str(nbytes),
        "license": item["license"],
        "author": item["author"],
        "source_url": item["url"],
        "cache_rel": str(dest.relative_to(cache)),
        "original_name": item["original_name"],
        "ext": ".jpg",
    }


def crawl_raw_urls(cache: Path) -> list[str]:
    seen_pages: set[str] = set()
    raw_urls: list[str] = []
    queue = list(RAW_ROOTS)
    href_re = re.compile(r"""href=["']([^"'#]+)["']""", re.I)
    while queue and len(seen_pages) < 500 and len(raw_urls) < 400:
        url = queue.pop(0)
        if url in seen_pages:
            continue
        seen_pages.add(url)
        try:
            body = request(url, timeout=60).decode("utf-8", "replace")
        except Exception as err:  # noqa: BLE001
            log(cache, f"raw index fail {err.__class__.__name__}")
            continue
        for match in href_re.finditer(body):
            href = urllib.parse.urljoin(url, html.unescape(match.group(1)))
            if not href.startswith("https://raw.pixls.us/"):
                continue
            path = urllib.parse.urlparse(href).path
            ext = Path(path).suffix.lower()
            if ext in RAW_EXTS:
                raw_urls.append(href.split("?")[0])
            elif ext == "" or path.endswith("/"):
                if href not in seen_pages and len(queue) < 800:
                    queue.append(href)
        if len(seen_pages) % 25 == 0:
            log(cache, f"raw crawl pages={len(seen_pages)} urls={len(set(raw_urls))}")
    # Diverse extensions first.
    by_ext: dict[str, list[str]] = defaultdict(list)
    for url in dict.fromkeys(raw_urls):
        by_ext[Path(urllib.parse.urlparse(url).path).suffix.lower()].append(url)
    mixed: list[str] = []
    while any(by_ext.values()) and len(mixed) < 200:
        for ext in sorted(by_ext):
            if by_ext[ext]:
                mixed.append(by_ext[ext].pop(0))
    log(cache, f"raw crawl done pages={len(seen_pages)} urls={len(mixed)}")
    return mixed


def head_length(url: str) -> int | None:
    req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=40) as resp:
            length = resp.headers.get("Content-Length")
            return int(length) if length else None
    except Exception:  # noqa: BLE001
        return None


def cache_raw(cache: Path, url: str) -> dict | None:
    length = head_length(url)
    if length is not None and not (RAW_MIN <= length <= RAW_MAX):
        return None
    name = Path(urllib.parse.urlparse(url).path).name
    ext = Path(name).suffix.lower()
    # Provisional name; rename to sha1 after download.
    tmp = cache / "raw" / f"incoming-{hashlib.sha1(url.encode()).hexdigest()}{ext}"
    try:
        nbytes = download_file(url, tmp, None)
    except Exception as err:  # noqa: BLE001
        log(cache, f"raw fail {err.__class__.__name__}")
        return None
    if not (RAW_MIN <= nbytes <= RAW_MAX):
        tmp.unlink(missing_ok=True)
        return None
    digest = sha1_file(tmp)
    dest = cache / "raw" / f"{digest}{ext}"
    if dest.exists():
        tmp.unlink(missing_ok=True)
    else:
        os.replace(tmp, dest)
    return {
        "sha1": digest,
        "kind": "raw",
        "bytes": str(nbytes),
        "license": "CC0",
        "author": "raw.pixls.us",
        "source_url": url,
        "cache_rel": str(dest.relative_to(cache)),
        "original_name": name,
        "ext": ext,
    }


def link_or_copy(src: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        return
    try:
        os.link(src, dest)
    except OSError:
        data = src.read_bytes()
        dest.write_bytes(data)


def build_sets(cache: Path, sets: Path, rows: list[dict]) -> dict:
    jpegs = [cache / row["cache_rel"] for row in rows if row["kind"] == "jpeg" and (cache / row["cache_rel"]).exists()]
    raws = [cache / row["cache_rel"] for row in rows if row["kind"] == "raw" and (cache / row["cache_rel"]).exists()]
    if len(jpegs) < 20:
        raise SystemExit(f"need at least 20 cached jpegs, have {len(jpegs)}")

    def take_jpeg(index: int) -> Path:
        return jpegs[index % len(jpegs)]

    def take_raw(index: int) -> Path:
        return raws[index % len(raws)]

    # A: 2000 flat unique names.
    a_dir = sets / "A-flat-2000"
    for i in range(2000):
        link_or_copy(take_jpeg(i), a_dir / f"frame_{i:04d}.jpg")

    # D: 1000 flat, different names, overlapping sources (page cache shared).
    d_dir = sets / "D-flat-1000"
    for i in range(1000):
        link_or_copy(take_jpeg(i + 17), d_dir / f"roll_{i:04d}.jpg")

    # Small folder used by the export scenario so a move cannot relocate A/D.
    e_dir = sets / "E-export"
    for i in range(24):
        link_or_copy(take_jpeg(i + 3), e_dir / f"export_{i:04d}.jpg")

    # B: ~500 mixed, RAW+JPEG pairs share a basename.
    b_dir = sets / "B-mixed-500"
    pair_count = min(80, len(raws)) if raws else 0
    made = 0
    for i in range(pair_count):
        raw = take_raw(i)
        stem = f"pair_{i:04d}"
        link_or_copy(raw, b_dir / f"{stem}{raw.suffix.lower()}")
        link_or_copy(take_jpeg(i), b_dir / f"{stem}.jpg")
        made += 2
    extra = 0
    while made < 500:
        if raws and extra % 5 == 0:
            raw = take_raw(pair_count + extra)
            link_or_copy(raw, b_dir / f"solo_{extra:04d}{raw.suffix.lower()}")
        else:
            link_or_copy(take_jpeg(300 + extra), b_dir / f"solo_{extra:04d}.jpg")
        made += 1
        extra += 1

    # C: nested shoots.
    plan = {
        "shoot-01": 80,
        "shoot-02": 150,
        "shoot-03": 220,
        "day2/morning": 60,
        "day2/evening": 180,
        "misc/pickups": 50,
        "misc/overflow": 120,
    }
    cursor = 0
    for folder, count in plan.items():
        for i in range(count):
            src = take_jpeg(cursor)
            # Sprinkle a raw into larger folders when we have them.
            if raws and i == 0:
                raw = take_raw(cursor)
                link_or_copy(raw, sets / "C-nested" / folder / f"frame_{i:04d}{raw.suffix.lower()}")
            else:
                link_or_copy(src, sets / "C-nested" / folder / f"frame_{i:04d}.jpg")
            cursor += 1

    def count_files(path: Path) -> int:
        return sum(1 for p in path.rglob("*") if p.is_file())

    summary = {
        "jpeg_unique": len(jpegs),
        "jpeg_bytes": sum(p.stat().st_size for p in jpegs),
        "raw_unique": len(raws),
        "raw_bytes": sum(p.stat().st_size for p in raws),
        "raw_exts": dict(Counter(p.suffix.lower() for p in raws)),
        "set_A": count_files(a_dir),
        "set_B": count_files(b_dir),
        "set_C": count_files(sets / "C-nested"),
        "set_D": count_files(d_dir),
        "set_E": count_files(e_dir),
    }
    (sets / "summary.json").write_text(json.dumps(summary), encoding="utf-8")
    return summary


def rows_from_disk(cache: Path) -> list[dict]:
    rows: list[dict] = []
    for kind, folder in (("jpeg", cache / "jpeg"), ("raw", cache / "raw")):
        if not folder.exists():
            continue
        for path in sorted(folder.iterdir()):
            if not path.is_file() or path.name.endswith(".part"):
                continue
            if path.stat().st_size < 1_000_000:
                continue
            rows.append(
                {
                    "sha1": path.stem,
                    "kind": kind,
                    "bytes": str(path.stat().st_size),
                    "license": "CC0" if kind == "raw" else "see-manifest",
                    "author": "",
                    "source_url": path.name,
                    "cache_rel": str(path.relative_to(cache)),
                    "original_name": path.name,
                    "ext": path.suffix.lower(),
                }
            )
    return rows


def cmd_layout(cache: Path, sets: Path) -> None:
    rows = rows_from_disk(cache)
    summary = build_sets(cache, sets, rows)
    print(
        "SUMMARY "
        f"jpeg_unique={summary['jpeg_unique']} jpeg_bytes={summary['jpeg_bytes']} "
        f"raw_unique={summary['raw_unique']} raw_bytes={summary['raw_bytes']} "
        f"exts={summary['raw_exts']} "
        f"A={summary['set_A']} B={summary['set_B']} C={summary['set_C']} D={summary['set_D']} E={summary['set_E']}"
    )


def cmd_fetch(cache: Path, sets: Path) -> None:
    cache.mkdir(parents=True, exist_ok=True)
    manifest_path = cache / "manifest.csv"
    rows = load_manifest(manifest_path)
    have_jpeg = sum(1 for row in rows.values() if row["kind"] == "jpeg" and (cache / row["cache_rel"]).exists())
    have_raw = sum(1 for row in rows.values() if row["kind"] == "raw" and (cache / row["cache_rel"]).exists())
    log(cache, f"resume jpeg={have_jpeg} raw={have_raw}")

    if have_jpeg < JPEG_TARGET:
        candidates = collect_jpeg_candidates(cache, JPEG_TARGET + 40)
        todo = [item for item in candidates if item["url"] not in rows]
        log(cache, f"jpeg download queue={len(todo)}")
        with ThreadPoolExecutor(max_workers=2) as pool:
            futures = [pool.submit(cache_jpeg, cache, item) for item in todo]
            for future in as_completed(futures):
                row = future.result()
                if row is None:
                    continue
                rows[row["source_url"]] = row
                have_jpeg += 1
                save_manifest(manifest_path, rows)
                log(cache, f"jpeg cached={have_jpeg} bytes={row['bytes']}")
                if have_jpeg >= JPEG_TARGET:
                    break
        save_manifest(manifest_path, rows)

    if have_raw < RAW_TARGET:
        urls = [url for url in crawl_raw_urls(cache) if url not in rows]
        log(cache, f"raw download queue={len(urls)}")
        # Sequential enough to be polite, 3 at a time.
        with ThreadPoolExecutor(max_workers=3) as pool:
            futures = [pool.submit(cache_raw, cache, url) for url in urls]
            for future in as_completed(futures):
                row = future.result()
                if row is None:
                    continue
                rows[row["source_url"]] = row
                have_raw += 1
                save_manifest(manifest_path, rows)
                log(cache, f"raw cached={have_raw} ext={row['ext']} bytes={row['bytes']}")
                if have_raw >= RAW_TARGET:
                    break
        save_manifest(manifest_path, rows)

    save_manifest(manifest_path, rows)
    summary = build_sets(cache, sets, list(rows.values()))
    print(
        "SUMMARY "
        f"jpeg_unique={summary['jpeg_unique']} jpeg_bytes={summary['jpeg_bytes']} "
        f"raw_unique={summary['raw_unique']} raw_bytes={summary['raw_bytes']} "
        f"exts={summary['raw_exts']} "
        f"A={summary['set_A']} B={summary['set_B']} C={summary['set_C']} D={summary['set_D']} E={summary['set_E']}"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--sets", type=Path, required=True)
    parser.add_argument("--layout-only", action="store_true")
    args = parser.parse_args()
    try:
        if args.layout_only:
            cmd_layout(args.cache, args.sets)
        else:
            cmd_fetch(args.cache, args.sets)
    except Exception as err:  # noqa: BLE001
        # Keep stdout small even on failure.
        print(f"SUMMARY_ERROR {err.__class__.__name__}: {err}")
        raise


if __name__ == "__main__":
    main()
