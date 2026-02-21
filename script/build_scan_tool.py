#!/usr/bin/env python3
import argparse
import base64
import glob
import gzip
import json
import sys


def load_payload(filepath=None):
    if not filepath:
        files = sorted(glob.glob("captured-output/payloads/*.json"))
        if not files:
            raise ValueError("No payload found in captured-output/payloads/")
        filepath = files[-1]

    with open(filepath, "r") as f:
        data = json.load(f)
    body = data.get("request", {}).get("body", {})
    raw = base64.b64decode(body["base64"])
    gzip_start = raw.find(b"\x1f\x8b\x08")
    return gzip.decompress(raw[gzip_start:])


def main():
    parser = argparse.ArgumentParser(description="Gradle Build Scan Payload Analyzer")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # We will add subparsers in subsequent tasks

    args = parser.parse_args()


if __name__ == "__main__":
    main()
