"""Local mitmproxy capture addon; use only for the selected library host."""
import json
import os
from pathlib import Path
from mitmproxy import http

ROOT = Path(os.environ.get("SPYDUS_CAPTURE_DIR", ".private/captures"))
ROOT.mkdir(parents=True, exist_ok=True)
ROOT.chmod(0o700)

def append_private(name, record):
    path = ROOT / name
    fd = os.open(path, os.O_CREAT | os.O_WRONLY | os.O_APPEND, 0o600)
    with os.fdopen(fd, "a") as f:
        f.write(json.dumps(record) + "\n")

def http_connect(flow: http.HTTPFlow):
    append_private("connect-hosts.jsonl", {"host": flow.request.host, "port": flow.request.port})

def response(flow: http.HTTPFlow):
    # Emulator-level proxy tunnels name an IP address; HTTP Host/SNI supplies
    # the actual library origin, exposed as pretty_host by mitmproxy.
    if flow.request.pretty_host != "canning.spydus.com":
        return
    append_private("live-flows.jsonl", {
            "method": flow.request.method, "url": flow.request.pretty_url,
            "request_headers": list(flow.request.headers.items(multi=True)),
            "request_body": flow.request.get_text(strict=False),
            "status": flow.response.status_code,
            "response_headers": list(flow.response.headers.items(multi=True)),
            "response_body": flow.response.get_text(strict=False),
            "timestamp": flow.request.timestamp_start,
    })
