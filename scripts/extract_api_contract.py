#!/usr/bin/env python3
"""Recover declared Refit contracts and DTO metadata without executing the app.

Run: .private/tools/venv/bin/python scripts/extract_api_contract.py
Dependency: dnfile. Inputs are private; outputs contain definitions, never values.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import uuid
import base64
from urllib.parse import urlsplit, parse_qsl

import dnfile

ROOT = Path(__file__).resolve().parents[1]
CLIENTS = {
    "IAuthClient", "IBookClient", "ILibrariesClient", "ILogoClient",
    "ILogOutClient", "INotificationClient", "ISelfIssueClient", "IStyleClient",
    "IValidateSessionClient",
}
PRIMITIVES = {
    0x01: "System.Void", 0x02: "System.Boolean", 0x03: "System.Char",
    0x04: "System.SByte", 0x05: "System.Byte", 0x06: "System.Int16",
    0x07: "System.UInt16", 0x08: "System.Int32", 0x09: "System.UInt32",
    0x0A: "System.Int64", 0x0B: "System.UInt64", 0x0C: "System.Single",
    0x0D: "System.Double", 0x0E: "System.String", 0x18: "System.IntPtr",
    0x19: "System.UIntPtr", 0x1C: "System.Object",
}


class Blob:
    def __init__(self, data):
        self.data, self.pos = data, 0

    def byte(self):
        result = self.data[self.pos]
        self.pos += 1
        return result

    def take(self, n):
        result = self.data[self.pos:self.pos + n]
        if len(result) != n:
            raise ValueError("Truncated metadata blob")
        self.pos += n
        return result

    def compressed(self):
        first = self.byte()
        if first < 0x80:
            return first
        if first < 0xC0:
            return ((first & 0x3F) << 8) | self.byte()
        if first < 0xE0:
            return ((first & 0x1F) << 24) | int.from_bytes(self.take(3), "big")
        raise ValueError("Invalid ECMA-335 compressed integer")

    def string(self):
        if self.data[self.pos] == 0xFF:
            self.pos += 1
            return None
        return self.take(self.compressed()).decode("utf-8")


class Metadata:
    def __init__(self, assembly):
        self.pe = dnfile.dnPE(str(assembly))
        self.md = self.pe.net.mdtables
        self.method_owner = {}
        for i, t in enumerate(self.md.TypeDef, 1):
            for m in t.MethodList:
                self.method_owner[m.row_index] = i
        self.attributes = defaultdict(list)
        for attr in self.md.CustomAttribute:
            ctor = attr.Type.row
            owner = ctor.Class if attr.Type.table.name == "MemberRef" else None
            if owner is None:
                name = self.name("TypeDef", self.method_owner[attr.Type.row_index])
            else:
                name = self.name(owner.table.name, owner.row_index)
            # Only decode attributes relevant to HTTP and JSON; avoid custom values.
            if not name.startswith("Refit.") and name not in {
                "System.Text.Json.Serialization.JsonPropertyNameAttribute",
                "System.Text.Json.Serialization.JsonIgnoreAttribute",
            }:
                continue
            sig = self.method_signature(ctor.Signature.value)
            blob = Blob(attr.Value.value)
            if blob.take(2) != b"\x01\x00":
                raise ValueError("Invalid custom attribute prolog")
            args = [self.attribute_value(blob, t) for t in sig[1]]
            named_count = int.from_bytes(blob.take(2), "little")
            if named_count:
                raise ValueError(f"Unsupported named attribute arguments: {name}")
            if blob.pos != len(blob.data):
                raise ValueError(f"Trailing custom attribute bytes: {name}")
            self.attributes[(attr.Parent.table.name, attr.Parent.row_index)].append({
                "type": name, "arguments": args,
            })

    def name(self, table, index):
        if table == "TypeSpec":
            return self.type(Blob(self.md.TypeSpec.rows[index - 1].Signature.value))
        row = getattr(self.md, table).rows[index - 1]
        ns, name = str(row.TypeNamespace), str(row.TypeName)
        return f"{ns}.{name}" if ns else name

    def coded_type(self, value):
        return self.name(["TypeDef", "TypeRef", "TypeSpec"][value & 3], value >> 2)

    def type(self, blob):
        kind = blob.byte()
        if kind in PRIMITIVES:
            return PRIMITIVES[kind]
        if kind in (0x11, 0x12):
            return self.coded_type(blob.compressed())
        if kind == 0x15:
            marker = blob.byte()
            if marker not in (0x11, 0x12):
                raise ValueError("Invalid GENERICINST marker")
            name = re.sub(r"`\d+$", "", self.coded_type(blob.compressed()))
            types = [self.type(blob) for _ in range(blob.compressed())]
            return f"{name}<{', '.join(types)}>"
        if kind == 0x1D:
            return self.type(blob) + "[]"
        if kind in (0x13, 0x1E):
            return ("!" if kind == 0x13 else "!!") + str(blob.compressed())
        if kind == 0x10:
            return self.type(blob) + "&"
        if kind in (0x1F, 0x20):
            modifier = self.coded_type(blob.compressed())
            return self.type(blob) + f" mod{('req' if kind == 0x1F else 'opt')}({modifier})"
        raise ValueError(f"Unsupported signature element 0x{kind:02x}")

    def method_signature(self, data):
        blob = Blob(data)
        flags = blob.byte()
        if flags & 0x10:
            blob.compressed()  # generic parameter count
        count = blob.compressed()
        result = self.type(blob)
        args = [self.type(blob) for _ in range(count)]
        if blob.pos != len(data):
            raise ValueError("Trailing method signature bytes")
        return result, args

    def attribute_value(self, blob, type_name):
        if type_name == "System.String":
            return blob.string()
        if type_name == "System.String[]":
            count = int.from_bytes(blob.take(4), "little", signed=True)
            return None if count == -1 else [blob.string() for _ in range(count)]
        raise ValueError(f"Unsupported attribute value type: {type_name}")

    def attrs(self, table, index):
        return self.attributes[(table, index)]


def first_attribute(attrs, name):
    for attr in attrs:
        if attr["type"] == name:
            return attr
    return None


def short(type_name):
    return re.sub(r"(?:System(?:\.Collections\.Generic|\.Threading(?:\.Tasks)?|\.Net\.Http)?|Spydus\.Domain\.DTO(?:\.Auth)?|Refit)\.", "", type_name)


def recover(assembly, evidence_dir):
    meta = Metadata(assembly)
    sha = hashlib.sha256(assembly.read_bytes()).hexdigest()
    mvid = str(uuid.UUID(bytes_le=meta.md.Module.rows[0].Mvid.value))
    evidence = []
    for filename in ("rea-domain-artifact.json", "rea-domain-members.json"):
        path = evidence_dir / filename
        if not path.exists():
            continue
        obj = json.loads(path.read_text())
        if obj["subject"]["digest"]["sha256"] != sha:
            raise ValueError(f"REA evidence digest does not match assembly: {filename}")
        if filename == "rea-domain-members.json" and obj["normalized_result"]["module"]["mvid"] != mvid:
            raise ValueError("REA evidence MVID does not match assembly")
        evidence.append({"file": filename, "evidence_id": obj["evidence_id"], "operation": obj["operation"]})
    operations = []
    for ti, typedef in enumerate(meta.md.TypeDef, 1):
        if str(typedef.TypeName) not in CLIENTS:
            continue
        type_attrs = meta.attrs("TypeDef", ti)
        for method_idx in typedef.MethodList:
            method = method_idx.row
            attrs = meta.attrs("MethodDef", method_idx.row_index)
            http = [a for a in attrs if re.fullmatch(r"Refit\.(Get|Post|Patch|Delete|Put|Head|Options)Attribute", a["type"])]
            if len(http) != 1:
                raise ValueError(f"Expected one HTTP attribute for {method.Name}")
            route = http[0]["arguments"][0]
            return_type, parameter_types = meta.method_signature(method.Signature.value)
            params = []
            placeholders = set(re.findall(r"\{([^}]+)\}", route))
            query_start = route.find("?")
            for pi in method.ParamList:
                param = pi.row
                if param.Sequence == 0:
                    continue
                name = str(param.Name)
                type_name = parameter_types[param.Sequence - 1]
                pa = meta.attrs("Param", pi.row_index)
                alias = first_attribute(pa, "Refit.AliasAsAttribute")
                wire_name = alias["arguments"][0] if alias else name
                header = first_attribute(pa, "Refit.HeaderAttribute")
                authorize = first_attribute(pa, "Refit.AuthorizeAttribute")
                body = first_attribute(pa, "Refit.BodyAttribute")
                binding = {"kind": "unannotated", "basis": "metadata only"}
                if header:
                    binding = {"kind": "header", "name": header["arguments"][0], "basis": "explicit attribute"}
                elif authorize:
                    binding = {"kind": "authorization", "header": "Authorization", "scheme": authorize["arguments"][0], "basis": "explicit attribute"}
                elif body:
                    binding = {"kind": "body", "basis": "explicit attribute", "serialization": "Refit default; runtime serializer configuration not established here"}
                elif type_name == "System.Threading.CancellationToken":
                    binding = {"kind": "cancellation", "basis": "signature type"}
                elif wire_name in placeholders:
                    position = route.index("{" + wire_name + "}")
                    binding = {"kind": "query-template" if query_start >= 0 and position > query_start else "path-template", "placeholder": wire_name, "basis": "route template + alias/name"}
                elif type_name in PRIMITIVES.values():
                    binding = {"kind": "query-default", "name": wire_name, "basis": "inferred Refit default; not live observed"}
                params.append({"name": name, "type": type_name, "alias": wire_name if alias else None, "binding": binding, "attributes": pa, "metadata_token": f"0x080000{pi.row_index:02x}"})
            headers = []
            for a in type_attrs + attrs:
                if a["type"] == "Refit.HeadersAttribute":
                    headers.extend(a["arguments"][0] or [])
            operations.append({
                "interface": meta.name("TypeDef", ti), "method": str(method.Name),
                "verb": http[0]["type"].split(".")[-1].replace("Attribute", "").upper(),
                "route_template": route, "return_type": return_type,
                "parameters": params, "declared_headers": headers,
                "attributes": attrs, "evidence_status": "static recovered contract",
                "live_observed": False, "metadata_token": f"0x060000{method_idx.row_index:02x}",
            })
    if len(operations) != 23:
        raise ValueError(f"Expected 23 scoped operations, recovered {len(operations)}")
    dto_types = []
    maps = {r.Parent.row_index: r.PropertyList for r in meta.md.PropertyMap}
    for ti, typedef in enumerate(meta.md.TypeDef, 1):
        if not str(typedef.TypeNamespace).startswith("Spydus.Domain.DTO"):
            continue
        properties = []
        for pi in maps.get(ti, []):
            row = pi.row
            type_name, indexers = meta.method_signature(row.Type.value)
            if indexers:
                raise ValueError(f"Unexpected DTO indexer: {row.Name}")
            attrs = meta.attrs("Property", pi.row_index)
            json_name = first_attribute(attrs, "System.Text.Json.Serialization.JsonPropertyNameAttribute")
            ignored = first_attribute(attrs, "System.Text.Json.Serialization.JsonIgnoreAttribute")
            properties.append({
                "name": str(row.Name), "type": type_name,
                "json_name": json_name["arguments"][0] if json_name else None,
                "json_name_basis": "explicit JsonPropertyNameAttribute" if json_name else "unresolved runtime naming policy",
                "json_ignore": bool(ignored), "attributes": attrs,
                "metadata_token": f"0x170000{pi.row_index:02x}",
            })
        if properties:
            dto_types.append({"type": meta.name("TypeDef", ti), "properties": properties,
                              "metadata_token": f"0x020000{ti:02x}"})
    return {
        "format_version": 1, "scope": "Declared native Refit interfaces in Spydus.Domain.dll",
        "source": {"assembly": assembly.name, "sha256": sha,
                   "mvid": mvid,
                   "rea_evidence": evidence},
        "evidence_status": "static recovered contracts; no live request or response claimed",
        "limitations": [
            "This is not a complete inventory of webview, JavaScript, native platform, or dynamically constructed requests.",
            "Base URLs, interceptors, cookies, bearer values, serializer configuration, and actual server responses are not established by interface metadata.",
            "Parameters lacking explicit binding attributes use inferred Refit defaults; verify them against traffic before implementing.",
            "A null json_name means no explicit JsonPropertyNameAttribute; the runtime serializer naming policy still determines the wire name.",
            "Reference-type nullability and property defaults are not inferred; System.Nullable<T> is decoded from signatures.",
            "DTO schemas describe declared properties only, not validation, required fields, computed property behavior, or server guarantees.",
        ],
        "operations": operations, "dto_types": dto_types,
    }


def attach_observations(data, har_path):
    """Attach only safe field names, empty values and public paging defaults.

    The HAR contains session credentials and patron data. Never copy complete
    URLs, headers, bodies, responses or arbitrary scalar values into outputs.
    """
    if not har_path.exists():
        return
    entries = json.loads(har_path.read_text())["log"]["entries"]
    observed_count = 0
    for entry_index, entry in enumerate(entries):
        request = entry["request"]
        url = urlsplit(request["url"])
        candidates = []
        for operation in data["operations"]:
            route_path = operation["route_template"].split("?", 1)[0]
            pattern = re.sub(r"\\\{[^}]+\\\}", r"[^/]+", re.escape(route_path))
            if request["method"] == operation["verb"] and re.fullmatch(pattern, url.path):
                candidates.append(operation)
        if len(candidates) != 1:
            continue
        operation = candidates[0]
        headers = []
        for header in request.get("headers", []):
            name, value = header["name"], header["value"]
            # Ordinary transport headers are not part of the app contract.
            if name.lower() not in {"content-type", "authorization", "spydus-lang"}:
                continue
            safe = None
            if name.lower() == "spydus-lang" and value == "":
                safe = ""
            elif name.lower() == "content-type" and value == "application/json; charset=UTF-8":
                safe = value
            elif name.lower() == "authorization":
                safe = "Bearer <redacted>" if value.startswith("Bearer ") else "<redacted>"
            headers.append({"name": name, "value": safe if safe is not None else "<redacted>"})
        query = []
        for name, value in parse_qsl(url.query, keep_blank_values=True):
            safe = "<redacted>"
            if name == "wifiName" and value == "":
                safe = ""
            elif (name, value) in {("setId", "0"), ("lastIrn", "0"), ("count", "500")}:
                safe = int(value)
            elif (name, value) == ("keywords", "Harry Potter"):
                safe = value  # Explicitly selected non-personal test search.
            query.append({"name": name, "value": safe})
        post = request.get("postData", {})
        body_keys = []
        if post.get("text", "").startswith("{"):
            body = json.loads(post["text"])
            body_keys = sorted(body)
        content = entry["response"].get("content", {})
        text = content.get("text", "")
        if content.get("encoding") == "base64":
            text = base64.b64decode(text).decode("utf-8")
        try:
            response = json.loads(text)
            shape = "object" if isinstance(response, dict) else "array" if isinstance(response, list) else "JSON string" if isinstance(response, str) else type(response).__name__
            keys = sorted(response) if isinstance(response, dict) else []
        except (ValueError, TypeError):
            shape, keys = "non-JSON or unavailable", []
        observation = {
            "source": "original Pixel_10_Pro application HTTP capture",
            "capture_file": har_path.name,
            "capture_entry_index": entry_index,
            "request_path_template": operation["route_template"].split("?", 1)[0],
            "query": query, "headers": headers, "body_field_names": body_keys,
            "response_status": entry["response"]["status"],
            "response_shape": shape, "response_field_names": keys,
        }
        if operation["method"] == "GetSearchBook" and shape == "JSON string":
            handoff = urlsplit(response)
            if handoff.path == "/cgi-bin/spydus.exe/ENQ/OPAC/ALLWRKENQ":
                observation["response_handoff_path"] = handoff.path
                observation["response_handoff_query_names"] = sorted(k for k, _ in parse_qsl(handoff.query))
                observation["response_note"] = "The native endpoint returns a JSON-encoded HTML navigation URL, not result records."
        operation.setdefault("observations", []).append(observation)
        operation["live_observed"] = True
        operation["evidence_status"] = "static recovered contract; live observed original app request"
        observed_count += 1
    capture = {
        "file": har_path.name, "sha256": hashlib.sha256(har_path.read_bytes()).hexdigest(),
        "entry_count": len(entries), "matched_entry_count": observed_count,
        "redaction": "All credentials and patron values excluded. Only field/header names, blank values, whitelisted public paging defaults and the explicit non-personal test search retained.",
    }
    data.setdefault("live_captures", []).append(capture)
    data["distinct_live_observed_operations"] = sum(o["live_observed"] for o in data["operations"])
    data["evidence_status"] = "static recovered contracts with separately labeled live observations"


def render(data):
    observed = sum(o["live_observed"] for o in data["operations"])
    intro = f"Recovered 23 declared operations from the installed app's managed assembly. {observed} operations have separate observations from original app HTTP traffic; the remaining {23 - observed} are static contracts only."
    lines = ["# Native API contracts", "", intro, "",
             f"Assembly SHA-256: `{data['source']['sha256']}`", "",
             f"Module MVID: `{data['source']['mvid']}`", "", "REA evidence:", ""]
    for evidence in data["source"]["rea_evidence"]:
        lines.append(f"- `{evidence['operation']}`: `{evidence['evidence_id']}`")
    if data.get("live_captures"):
        lines.extend(["", "Original-app HTTP capture evidence (raw files stay private):", ""])
        for capture in data["live_captures"]:
            lines.append(f"- `{capture['file']}`: {capture['matched_entry_count']} matched request(s); SHA-256 `{capture['sha256']}`.")
    lines.extend(["", "Reproduce with `.private/tools/venv/bin/python scripts/extract_api_contract.py`. Machine-readable definitions, explicit attributes, parameter types and evidence tokens are in [native-endpoints.json](native-endpoints.json). Parameterized requests are in [spydus.postman_collection.json](spydus.postman_collection.json).", "", "## Operations", "", "| Interface / method | Verb | Exact route template | Evidence |", "| --- | --- | --- | --- |"])
    for op in data["operations"]:
        lines.append(f"| `{op['interface'].split('.')[-1]}.{op['method']}` | {op['verb']} | `{op['route_template']}` | {'static + observed' if op['live_observed'] else 'static'} |")
    lines.extend(["", "The declarations preserve several unusual details: token exchange and refresh use GET; the device DELETE route omits `/patrons`; the branches route repeats `/mobile`; and the self issue interface declares `Authorisation: Bearer` with that exact spelling and no token parameter. Additional HTTP handler behavior or server support for those declarations requires separate evidence."])
    lines.extend(["", "## Request and response bindings", ""])
    for op in data["operations"]:
        lines.extend([f"### {op['method']}", "", f"`{op['verb']} {op['route_template']}`", "", f"Declared return: `{short(op['return_type'])}`.", ""])
        if op.get("observations"):
            observations = op["observations"]
            ob = observations[0]
            lines.append(f"Observed in original app traffic: {len(observations)} request(s), HTTP {ob['response_status']}, {ob['response_shape']} response.")
            details = []
            for q in ob["query"]:
                details.append(f"query `{q['name']}={q['value']}`")
            for h in ob["headers"]:
                details.append(f"header `{h['name']}: {h['value']}`")
            if ob["body_field_names"]:
                details.append("JSON body keys " + ", ".join(f"`{k}`" for k in ob["body_field_names"]))
            if ob["response_field_names"]:
                details.append("JSON response keys " + ", ".join(f"`{k}`" for k in ob["response_field_names"]))
            if ob.get("response_handoff_path"):
                details.append(f"JSON-string handoff to `{ob['response_handoff_path']}`; no result records in the native response")
            lines.extend(["", "; ".join(details) + ".", ""])
        if op["declared_headers"]:
            lines.append("Declared headers: " + ", ".join(f"`{h}`" for h in op["declared_headers"]) + ".")
            lines.append("")
        if op["parameters"]:
            lines.extend(["| Parameter | Type | Binding |", "| --- | --- | --- |"])
            for p in op["parameters"]:
                b = p["binding"]
                detail = b["kind"]
                if b["kind"] == "authorization":
                    detail += f" `Authorization: {b['scheme']} <token>`"
                elif "name" in b:
                    detail += f" `{b['name']}`"
                elif "placeholder" in b:
                    detail += f" `{{{b['placeholder']}}}`"
                if b["kind"] == "query-default":
                    detail += " (inferred; not captured)"
                lines.append(f"| `{p['name']}` | `{short(p['type'])}` | {detail} |")
            lines.append("")
    lines.extend(["## DTO properties", "", "Explicit JSON names are shown below. An unresolved name requires the application's runtime serializer policy. Ignored properties are application helpers, not declared serialized fields. Nullable value types are recorded as `Nullable<T>`; reference-type nullability is not asserted.", ""])
    for dto in data["dto_types"]:
        lines.extend([f"### {dto['type'].split('.')[-1]}", "", "| CLR property | Type | JSON name / status |", "| --- | --- | --- |"])
        for p in dto["properties"]:
            name = "ignored (`JsonIgnore`)" if p["json_ignore"] else f"`{p['json_name']}`" if p["json_name"] else "unresolved naming policy"
            lines.append(f"| `{p['name']}` | `{short(p['type'])}` | {name} |")
        lines.append("")
    lines.extend(["## Limits", ""])
    lines.extend(f"- {limit}" for limit in data["limitations"])
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assembly", type=Path, default=ROOT / ".private/apk/assemblies/Spydus.Domain.dll")
    parser.add_argument("--evidence-dir", type=Path, default=ROOT / ".private/captures")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "docs/api")
    parser.add_argument("--har", type=Path, action="append", help="HAR source; repeat for multiple captures")
    args = parser.parse_args()
    data = recover(args.assembly, args.evidence_dir)
    for har in args.har or [ROOT / ".private/captures/emulator.har", ROOT / ".private/captures/discover.har"]:
        attach_observations(data, har)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "native-endpoints.json").write_text(json.dumps(data, indent=2) + "\n")
    (args.output_dir / "native-endpoints.md").write_text(render(data))
    print(f"Recovered {len(data['operations'])} operations and {len(data['dto_types'])} DTO types")


if __name__ == "__main__":
    main()
