"""Decode strings passed to the observed Spydus.dll string helper (static only).

This is specific to the recorded build. It does not execute the assembly and
does not claim that every obfuscated string or dynamic route is recovered.
"""
import argparse
import hashlib
import json
import struct
from pathlib import Path

import dnfile
from dncil.cil.body import CilMethodBody
from dncil.cil.body.reader import CilMethodBodyReaderBytes

parser = argparse.ArgumentParser()
parser.add_argument("assembly", type=Path)
parser.add_argument("output", type=Path)
args = parser.parse_args()
pe = dnfile.dnPE(str(args.assembly))
records = []
errors = []
owners = {}
for t in pe.net.mdtables.TypeDef.rows:
    for m in t.MethodList:
        owners[id(m.row)] = str(t.TypeNamespace) + "." + str(t.TypeName)

def local_index(ins):
    return ins.operand.index if ins.operand is not None else int(str(ins.opcode).split(".")[-1])

def user_chars(offset):
    raw = pe.net.user_strings.__data__[offset:]
    if raw[0] & 0x80 == 0:
        length, prefix = raw[0], 1
    elif raw[0] & 0xC0 == 0x80:
        length, prefix = ((raw[0] & 0x3F) << 8) | raw[1], 2
    else:
        length, prefix = ((raw[0] & 0x1F) << 24) | (raw[1] << 16) | (raw[2] << 8) | raw[3], 4
    encoded = raw[prefix:prefix + length - 1]
    return struct.unpack("<" + "H" * (len(encoded) // 2), encoded)

for row, method in enumerate(pe.net.mdtables.MethodDef.rows, 1):
    if not method.Rva:
        continue
    try:
        instructions = CilMethodBody(CilMethodBodyReaderBytes(pe.get_data(method.Rva, 200000))).instructions
    except Exception as e:
        errors.append({"method_row": row, "error": str(e)})
        continue
    seeds = {}
    for pos, ins in enumerate(instructions[:-1]):
        op = str(ins.opcode)
        if op in ("ldc.i4", "ldc.i4.s"):
            value = ins.operand
        elif op.startswith("ldc.i4."):
            try:
                value = int(op.rsplit(".", 1)[-1])
            except ValueError:
                continue
        else:
            continue
        following = instructions[pos + 1]
        if str(following.opcode).startswith("stloc") and 0 <= value <= 255:
            seeds[local_index(following)] = value
    for pos, ins in enumerate(instructions[:-2]):
        if str(ins.opcode) != "ldstr":
            continue
        argument, call = instructions[pos + 1:pos + 3]
        if str(call.opcode) != "call" or call.operand.value != 0x06000E3D:
            continue
        if not str(argument.opcode).startswith("ldloc"):
            continue
        seed = seeds.get(local_index(argument))
        if seed is None:
            continue
        key = 869004468 + seed + 66 + 71
        decoded = []
        for char in user_chars(ins.operand.value & 0xFFFFFF):
            low = (char & 255) ^ key
            key += 1
            high = (char >> 8) ^ key
            key += 1
            decoded.append(((low & 255) << 8) | (high & 255))
        text = struct.pack("<" + "H" * len(decoded), *decoded).decode("utf-16le", errors="surrogatepass")
        records.append({"declaring_type": owners[id(method)], "method": str(method.Name),
                        "method_token": f"0x{0x06000000 + row:08x}", "il_offset": ins.offset,
                        "seed": seed, "text": text})
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps({"assembly_sha256": hashlib.sha256(args.assembly.read_bytes()).hexdigest(),
                                  "helper_token": "0x06000e3d", "records": records,
                                  "decode_errors": errors}, ensure_ascii=True, indent=2) + "\n")
print(f"Recovered {len(records)} string uses; {len(errors)} method decode errors")
