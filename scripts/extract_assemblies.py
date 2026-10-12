"""Extract the observed .NET Android v3 assembly store without executing code."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

import lz4.block

parser = argparse.ArgumentParser()
parser.add_argument("store", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
source = args.store.read_bytes()
base = source.index(b"XABA")
data = source[base:]
magic, version, count, index_count, index_size = struct.unpack_from("<5I", data)
if version & 0xFFFF != 3:
    raise ValueError(f"Unsupported assembly store version: {version:#x}")
cursor = 20 + index_size
descriptors = [struct.unpack_from("<7I", data, cursor + i * 28) for i in range(count)]
cursor += count * 28
names = []
for _ in range(count):
    length, = struct.unpack_from("<I", data, cursor)
    cursor += 4
    names.append(data[cursor:cursor + length].decode("utf-8"))
    cursor += length
args.output.mkdir(parents=True, exist_ok=True)
manifest = []
for name, descriptor in zip(names, descriptors):
    _, offset, size, *_ = descriptor
    image = data[offset:offset + size]
    if len(image) != size:
        raise ValueError(f"Truncated assembly: {name}")
    compressed = image.startswith(b"XALZ")
    if compressed:
        _, _, uncompressed_size = struct.unpack_from("<3I", image)
        image = lz4.block.decompress(image[12:], uncompressed_size=uncompressed_size)
    if not image.startswith(b"MZ"):
        raise ValueError(f"Not a PE image: {name}")
    target = (args.output / name).resolve()
    if not target.is_relative_to(args.output.resolve()):
        raise ValueError(f"Unsafe assembly name: {name}")
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(image)
    manifest.append(dict(name=name, store_offset=base + offset, stored_size=size,
                         size=len(image), compressed=compressed,
                         sha256=hashlib.sha256(image).hexdigest()))
(args.output / "manifest.json").write_text(json.dumps(dict(
    source_sha256=hashlib.sha256(source).hexdigest(), payload_offset=base,
    version=version, assemblies=manifest), indent=2) + "\n")
print(f"Extracted {len(manifest)} assemblies")
