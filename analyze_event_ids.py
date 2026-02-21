#!/usr/bin/env python3
"""Search for string anchors in the decompressed Gradle build scan payload."""

import base64
import gzip
import json
import sys
import glob


def load_upload_payload():
    """Find and load the latest binary upload payload file."""
    files = sorted(glob.glob("captured-output/payloads/*.json"))
    for f in files:
        d = json.load(open(f))
        if "/upload" in d["request"]["uri"] and "20260221_" in f:
            return f, d
    raise ValueError("No upload payload found")


def decode_body(body):
    """Decode the request body (base64 or plain string)."""
    if isinstance(body, dict) and "base64" in body:
        return base64.b64decode(body["base64"])
    return body.encode()


def find_gzip_offset(data: bytes) -> int:
    """Find the offset of the gzip magic bytes."""
    magic = b"\x1f\x8b\x08"
    idx = data.find(magic)
    if idx == -1:
        raise ValueError("No gzip magic bytes found")
    return idx


def decode_varint_at(data, pos):
    """Decode LEB128 at pos, return (value, bytes_consumed)."""
    result = 0
    shift = 0
    consumed = 0
    while pos + consumed < len(data):
        b = data[pos + consumed]
        consumed += 1
        result |= (b & 0x7F) << shift
        if (b & 0x80) == 0:
            break
        shift += 7
    return result, consumed


def search_string(data: bytes, target: str, context_before: int = 64):
    """Find all occurrences of target in data and show context bytes before each."""
    target_bytes = target.encode("utf-8")
    results = []
    start = 0
    while True:
        idx = data.find(target_bytes, start)
        if idx == -1:
            break

        before_start = max(0, idx - context_before)
        context = data[before_start:idx]

        results.append(
            {
                "offset": idx,
                "context_hex": context.hex(),
                "string": target,
            }
        )
        start = idx + 1
    return results


def main():
    path, payload = load_upload_payload()
    print(f"Loaded: {path}")

    body = decode_body(payload["request"]["body"])
    gz_offset = find_gzip_offset(body)
    print(f"Gzip starts at offset: {gz_offset}")

    decompressed = gzip.decompress(body[gz_offset:])
    print(f"Decompressed size: {len(decompressed)} bytes")

    # Write out for CLI analysis
    with open("/tmp/decompressed.bin", "wb") as f:
        f.write(decompressed)

    anchors = sys.argv[1:] if len(sys.argv) > 1 else ["Linux"]
    for anchor in anchors:
        hits = search_string(decompressed, anchor)
        print(f"\n=== Anchor: '{anchor}' ===")
        if not hits:
            print("  NOT FOUND")
        for hit in hits:
            print(f"  Offset: {hit['offset']}")
            # Format hex context nicely, space separated
            hex_str = hit["context_hex"]
            formatted_hex = " ".join(
                [hex_str[i : i + 2] for i in range(0, len(hex_str), 2)]
            )
            print(f"  Context hex: {formatted_hex}")


if __name__ == "__main__":
    main()
