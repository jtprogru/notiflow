#!/usr/bin/env python3
"""Minimal Telegram Bot API mock for bats tests.

Reads a comma-separated list of responses (status:fixture) and serves them in
order for each incoming POST request. Logs every request to a file with a
delimited format that bats helpers can grep/awk.
"""
import argparse
import json
import os
import sys
import socketserver
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

FIXTURES = {
    "success": (200, {"ok": True, "result": {"message_id": 42}}),
    "success_99": (200, {"ok": True, "result": {"message_id": 99}}),
    "rate_limit": (
        429,
        {
            "ok": False,
            "error_code": 429,
            "description": "Too Many Requests: retry after 1",
            "parameters": {"retry_after": 1},
        },
    ),
    # Pathological retry_after value used to exercise the NF_MAX_RETRY_AFTER cap.
    "rate_limit_huge": (
        429,
        {
            "ok": False,
            "error_code": 429,
            "description": "Too Many Requests: retry after 9999",
            "parameters": {"retry_after": 9999},
        },
    ),
    # Non-integer retry_after used to verify safe fallback to 1s.
    "rate_limit_garbage": (
        429,
        {
            "ok": False,
            "error_code": 429,
            "description": "Too Many Requests: retry after soon",
            "parameters": {"retry_after": "soon"},
        },
    ),
    "server_error": (500, {"ok": False, "error_code": 500, "description": "internal"}),
    "bad_request": (
        400,
        {"ok": False, "error_code": 400, "description": "Bad Request: chat not found"},
    ),
    "unauthorized": (401, {"ok": False, "error_code": 401, "description": "Unauthorized"}),
    "forbidden": (403, {"ok": False, "error_code": 403, "description": "Forbidden"}),
    "not_found": (404, {"ok": False, "error_code": 404, "description": "Not Found"}),
    # Sleeps --hang-seconds before responding 200. Used to test curl --max-time.
    "hang": (200, {"ok": True, "result": {"message_id": 42}}),
}


class FastBindServer(ThreadingHTTPServer):
    """ThreadingHTTPServer without the reverse-DNS lookup in its bind path.

    http.server.HTTPServer.server_bind() calls socket.getfqdn() on the bound address. On a
    CI runner whose resolver cannot answer a reverse lookup for 127.0.0.1, that call blocks
    until the DNS timeout — the server binds, but the port file is written seconds later,
    and a caller waiting on it concludes the server never started. Nothing here uses
    server_name, so the lookup is pure cost.
    """

    def server_bind(self):
        socketserver.TCPServer.server_bind(self)
        host, port = self.server_address[:2]
        self.server_name = host
        self.server_port = port


def make_handler(responses, log_path, counter, hang_seconds):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, fmt, *args):  # silence stderr noise
            pass

        def address_string(self):
            # Default implementation reverse-resolves the client address; same DNS stall as
            # server_bind, on every request instead of once.
            return self.client_address[0]

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            body = self.rfile.read(length).decode("utf-8") if length > 0 else ""

            with open(log_path, "a", encoding="utf-8") as fh:
                fh.write("---REQUEST---\n")
                fh.write(f"PATH {self.path}\n")
                fh.write(f"METHOD {self.command}\n")
                fh.write(f"CT {self.headers.get('Content-Type', '')}\n")
                fh.write("---BODY---\n")
                fh.write(body)
                fh.write("\n")

            idx = min(counter["n"], len(responses) - 1)
            counter["n"] += 1
            status, fixture, name = responses[idx]

            if name == "hang":
                # Block long enough that curl --max-time fires first. Wrapped in
                # try/except because the client likely closes the socket and the
                # write below would otherwise raise BrokenPipeError into stderr.
                try:
                    time.sleep(hang_seconds)
                except Exception:
                    return

            payload = json.dumps(fixture).encode("utf-8")
            try:
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)
            except (BrokenPipeError, ConnectionResetError):
                # Client already gave up (e.g. timed out). Nothing to do.
                pass

    return Handler


def parse_responses(spec):
    items = []
    for token in spec.split(","):
        token = token.strip()
        if not token:
            continue
        if ":" not in token:
            raise ValueError(f"bad token: {token!r}")
        status_str, name = token.split(":", 1)
        fixture = FIXTURES.get(name)
        if fixture is None:
            raise ValueError(f"unknown fixture: {name!r}")
        items.append((int(status_str), fixture[1], name))
    if not items:
        raise ValueError("no responses provided")
    return items


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--responses", required=True)
    ap.add_argument("--log", required=True)
    ap.add_argument("--port-file", required=True)
    ap.add_argument("--pid-file", required=True)
    ap.add_argument("--hang-seconds", type=float, default=30.0,
                    help="How long the 'hang' fixture blocks before responding.")
    args = ap.parse_args()

    responses = parse_responses(args.responses)
    counter = {"n": 0}
    handler = make_handler(responses, args.log, counter, args.hang_seconds)

    server = FastBindServer(("127.0.0.1", 0), handler)
    server.daemon_threads = True
    port = server.server_address[1]

    with open(args.port_file, "w", encoding="utf-8") as fh:
        fh.write(str(port))
    with open(args.pid_file, "w", encoding="utf-8") as fh:
        fh.write(str(os.getpid()))

    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()

    try:
        thread.join()
    except KeyboardInterrupt:
        server.shutdown()


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"mock_server error: {exc}", file=sys.stderr)
        sys.exit(1)
