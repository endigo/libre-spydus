#!/usr/bin/env python3
"""Generate parameterized Postman requests from recovered native contracts.

No requests are sent. No credentials or patron values are read from captures.
Run: python3 scripts/generate_postman.py
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
PARAMETERS = {
    "refreshToken": "refresh_token", "clientId": "client_id",
    "authorizationCode": "authorization_code", "grantType": "grant_type",
    "searchMode": "search_mode", "searchData": "search_data",
    "languageIdentifier": "language_identifier", "groupCode": "group_code",
    "bearerToken": "bearer_token", "borrowerConsortiumRecordIdentifier": "brwd_irn",
    "borrowerLocalAccountIdentifier": "patron_irn", "patronIrn": "patron_irn",
    "patronId": "patron_id", "id": "notification_id", "status": "notification_status",
    "deviceToken": "device_token", "itemId": "item_id",
    "alertAcknowledged": "alert_acknowledged", "wifiName": "wifi_name",
    "isoCode": "language_iso_code", "setId": "set_id", "lastIrn": "last_irn",
    "count": "count",
}
BODY_PARAMETERS = {
    "DeviceToken": "device_token", "Platform": "device_platform",
    "NewDeviceToken": "new_device_token", "OldDeviceToken": "old_device_token",
    "BorrowerSessionId": "session_id", "Tags": "device_tags_json",
}


def variable(name):
    return "{{" + name + "}}"


def generate(contract):
    # Empty defaults prevent collection exports from carrying user credentials.
    # Numeric paging values are observed original-app defaults, kept as numbers.
    defaults = {
        "library_base_url": "https://canning.spydus.com",
        "library_list_base_url": "https://librarylist.spydus.com/api/1.0",
        "identity_base_url": "https://idp.spydus.com",
        "client_id": "spydusmobile", "redirect_uri": "spyduslibrary://auth",
        "response_type": "code", "scope": "openid profile mobile", "prompt": "",
        "library_id": "", "set_id": 0, "last_irn": 0, "count": 500,
        "device_tags_json": "[]",
    }
    descriptions = {
        "scope": "Static base scope. The app expands this with libid_<library GUID> and lang_<languageIdentifier>; build the actual selected-library scope before authorizing. No suffix is guessed for an empty language.",
        "prompt": "The app conditionally uses login when forcing interactive authorization. Disabled by default in the helper request.",
        "library_id": "Selected library GUID for authorization scope expansion. Empty; supply your own.",
        "device_tags_json": "JSON array of strings. Empty-array example, not an observed registration request.",
        "alert_acknowledged": "System.Boolean query value. Empty because its default was not observed; supply true or false explicitly.",
        "set_id": "System.Int64; observed initial branch paging value 0.",
        "last_irn": "System.Int64; observed initial branch paging value 0.",
        "count": "System.Int32; observed initial branch paging value 500.",
    }
    for name in set(PARAMETERS.values()) | set(BODY_PARAMETERS.values()):
        defaults.setdefault(name, "")
    groups = {}
    dto_by_type = {dto["type"]: dto for dto in contract["dto_types"]}
    for operation in contract["operations"]:
        if operation["route_template"].startswith("/openid/"):
            base = "identity_base_url"
        elif operation["interface"].endswith("ILibrariesClient"):
            base = "library_list_base_url"
        else:
            base = "library_base_url"
        route = operation["route_template"]
        headers = []
        for header in operation["declared_headers"]:
            key, value = header.split(":", 1)
            headers.append({"key": key, "value": value.strip(), "type": "text"})
        query_defaults = []
        body = None
        param_notes = []
        for parameter in operation["parameters"]:
            binding = parameter["binding"]
            kind = binding["kind"]
            param_notes.append(f"{parameter['name']}: {parameter['type']}; {kind}; {binding['basis']}.")
            if kind in {"path-template", "query-template", "query-default", "header", "authorization"}:
                name = PARAMETERS[parameter["name"]]
                value = variable(name)
                if kind in {"path-template", "query-template"}:
                    route = route.replace("{" + binding["placeholder"] + "}", value)
                elif kind == "query-default":
                    query_defaults.append(binding["name"] + "=" + value)
                elif kind == "header":
                    headers.append({"key": binding["name"], "value": value, "type": "text"})
                else:
                    headers.append({"key": "Authorization", "value": binding["scheme"] + " " + value, "type": "text"})
            elif kind == "body":
                lines = []
                for prop in dto_by_type[parameter["type"]]["properties"]:
                    if prop["json_ignore"]:
                        continue
                    if not prop["json_name"]:
                        raise ValueError(f"Unresolved wire name for body property {prop['name']}")
                    name = BODY_PARAMETERS[prop["name"]]
                    value = variable(name)
                    # JSON array placeholders remain unquoted; strings are quoted.
                    # Do not invent boolean/numeric body defaults for unknown types.
                    if prop["type"] == "System.String":
                        value = json.dumps(value)
                    elif prop["type"] != "System.Collections.Generic.List<System.String>":
                        raise ValueError(f"Unsupported body scalar type: {prop['type']}")
                    lines.append("  " + json.dumps(prop["json_name"]) + ": " + value)
                body = {"mode": "raw", "raw": "{\n" + ",\n".join(lines) + "\n}",
                        "options": {"raw": {"language": "json"}}}
                if not any(h["key"].lower() == "content-type" for h in headers):
                    headers.append({"key": "Content-Type", "value": "application/json", "type": "text", "description": "JSON body example; serializer/interceptor-specific headers were not observed for this operation."})
        if query_defaults:
            route += ("&" if "?" in route else "?") + "&".join(query_defaults)
        # Every original template placeholder must now be parameterized.
        if re.search(r"(?<!\{)\{[^{}]+\}(?!\})", route):
            raise ValueError(f"Unresolved route placeholder: {route}")
        description = operation["evidence_status"] + ".\n\n" + "\n".join(param_notes)
        if operation.get("observations"):
            observed = operation["observations"][0]
            description += f"\n\nOriginal app capture: HTTP {observed['response_status']}; {observed['response_shape']} response. Credentials and patron values are excluded."
            if observed.get("response_note"):
                description += " " + observed["response_note"]
        if operation["interface"].endswith("ISelfIssueClient"):
            description += "\n\nThe declared Authorisation: Bearer header is preserved exactly. No extra Authorization header is invented; runtime HTTP handler behavior remains unverified."
        request = {"method": operation["verb"], "header": headers,
                   "url": variable(base) + route, "description": description,
                   "auth": {"type": "noauth"}}
        if body is not None:
            request["body"] = body
        group = operation["interface"].split(".")[-1]
        groups.setdefault(group, []).append({"name": operation["method"], "request": request, "response": []})
    authorize_query = [
        {"key": "client_id", "value": variable("client_id")},
        {"key": "response_type", "value": variable("response_type")},
        {"key": "redirect_uri", "value": variable("redirect_uri")},
        {"key": "scope", "value": variable("scope")},
        {"key": "prompt", "value": variable("prompt"), "disabled": True},
    ]
    raw = variable("identity_base_url") + "/openid/authorize?" + "&".join(q["key"] + "=" + q["value"] for q in authorize_query if not q.get("disabled"))
    helper = {"name": "Authorization helper (static)", "item": [{
        "name": "Authorize in browser", "request": {
            "method": "GET", "auth": {"type": "noauth"}, "header": [],
            "url": {"raw": raw, "host": [variable("identity_base_url")], "path": ["openid", "authorize"], "query": authorize_query},
            "description": "Static helper, not captured live. Evidence: app.config; Spydus.Helpers.AuthConstants; OAuthUriBuilderExtensions; app-string-evidence.json method token 0x06000adf. Route /openid/authorize, response_type=code, client_id=spydusmobile, redirect_uri=spyduslibrary://auth. Base scope is openid profile mobile; the app appends libid_<library GUID> and lang_<languageIdentifier>. Fill scope from the selected library before authorizing; do not guess empty-language suffix behavior. The app conditionally sets prompt=login for forced interaction. Browser authorization redirects to the app callback. No PKCE behavior is asserted by this helper.",
        }, "response": [],
    }]}
    return {
        "info": {"name": "Spydus Library — recovered API contracts",
                 "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json",
                 "description": "23 recovered native Refit operations plus one static browser authorization helper. Live observations are labeled per request; unobserved operations remain static contracts. All patron IDs, tokens and session variables are empty. No global authentication, scripts, automatic token extraction or mutation tests are installed. Declared route/header quirks are preserved. Import this collection and fill the variables needed by your selected request."},
        "variable": [{"key": key, "value": value, "description": descriptions.get(key, "Supply your own value; no captured private value is included.") if value == "" or key in descriptions else "Public static application setting or observed safe paging default."} for key, value in sorted(defaults.items())],
        "item": [helper] + [{"name": key, "item": items} for key, items in groups.items()],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, default=ROOT / "docs/api/native-endpoints.json")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/api/spydus.postman_collection.json")
    args = parser.parse_args()
    contract = json.loads(args.contract.read_text())
    collection = generate(contract)
    args.output.write_text(json.dumps(collection, indent=2) + "\n")
    print(f"Generated {sum(len(group['item']) for group in collection['item'])} parameterized requests; sent none")


if __name__ == "__main__":
    main()
