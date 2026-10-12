"""Produce shareable, deidentified HAR fixtures from local app captures."""
import argparse
import base64
import copy
import json
import re
from pathlib import Path
from urllib.parse import quote, urlsplit

parser = argparse.ArgumentParser()
parser.add_argument("inputs", type=Path, nargs="+")
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
entries = []
for path in args.inputs:
    entries.extend(json.loads(path.read_text())["log"]["entries"])

def content_text(content):
    value = content.get("text", "")
    if content.get("encoding") == "base64":
        value = base64.b64decode(value).decode("utf-8", errors="replace")
    return value

# Extract known private values from the validated session contract before
# sanitizing any URLs or strings. Header credentials are removed outright.
replacements = {}
for e in entries:
    if "validatesessionid" not in e["request"]["url"]:
        continue
    for body in (e["request"].get("postData", {}).get("text", "{}"),
                 content_text(e["response"].get("content", {}))):
        try:
            obj = json.loads(body)
        except ValueError:
            continue
        for key in ("sessionId", "brwSessionId", "name", "barcode", "brwIrn", "brwdIrn"):
            value = obj.get(key)
            if isinstance(value, str) and value:
                replacements[value] = "{{" + key + "}}"

def replace_known(text):
    for value, marker in sorted(replacements.items(), key=lambda x: -len(x[0])):
        text = text.replace(value, marker)
    return text

def sanitize_json(value, private_strings=False):
    if isinstance(value, dict):
        result = {}
        for key, item in value.items():
            if isinstance(item, str) and re.search(r"session|token|password|barcode|brwirn|brwdirn", key, re.I):
                result[key] = "{{" + key + "}}"
            else:
                result[key] = sanitize_json(item, private_strings)
        return result
    if isinstance(value, list):
        return [sanitize_json(x, private_strings) for x in value]
    if isinstance(value, str):
        return "<redacted-string>" if private_strings else replace_known(value)
    if isinstance(value, (int, float)) and not isinstance(value, bool) and (private_strings or str(value) in replacements):
        return 0
    return value

sanitized = []
for original in entries:
    if urlsplit(original["request"]["url"]).hostname != "canning.spydus.com":
        continue
    entry = copy.deepcopy(original)
    request, response = entry["request"], entry["response"]
    private_strings = "/notifications" in request["url"]
    request["url"] = replace_known(request["url"])
    # HAR URL fields remain valid URIs, even when they contain template markers.
    for marker in replacements.values():
        request["url"] = request["url"].replace(marker, quote(marker, safe=""))
    for message in (request, response):
        message["cookies"] = []
        message["headers"] = [h for h in message.get("headers", [])
                              if h["name"].lower() not in ("authorization", "authorisation", "cookie", "set-cookie")]
        for header in message["headers"]:
            header["value"] = replace_known(header["value"])
    if "postData" in request:
        post = request["postData"]
        try:
            obj = json.loads(post.get("text", ""))
            post["text"] = json.dumps(sanitize_json(obj))
        except ValueError:
            post["text"] = "<redacted-non-json-request-body>"
        post.pop("params", None)
    content = response.get("content", {})
    raw = content_text(content)
    try:
        body = sanitize_json(json.loads(raw), private_strings)
        content["text"] = json.dumps(body, ensure_ascii=False)
    except ValueError:
        content["text"] = "<non-json-response-body-omitted>"
    content.pop("encoding", None)
    # Lengths and timings are original evidence, not lengths of redacted data.
    entry["_redaction"] = {"credentials": "removed", "known_account_values": "replaced",
                            "notification_text": "redacted", "original_lengths_retained": True}
    sanitized.append(entry)
output = {"log": {"version": "1.2", "creator": {"name": "Spydus capture sanitizer", "version": "1"},
                  "entries": sanitized}}
serialized = json.dumps(output, ensure_ascii=False, indent=2)
for secret in replacements:
    if secret in serialized:
        raise ValueError("Known private value remains in sanitized output")
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(serialized + "\n")
print(f"Sanitized {len(sanitized)} HTTP records")
