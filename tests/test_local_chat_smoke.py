# SPDX-License-Identifier: Apache-2.0
"""Connection recording must not turn partial or failed streams into success."""

import io
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from scripts import local_chat_smoke as smoke


class Stream(io.BytesIO):
    def getheader(self, name):
        return "application/x-ndjson"


class SmokeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.auth = self.root / "auth"
        self.auth.mkdir(mode=0o700)
        self.control = "a" * 64
        self.client = "b" * 64
        for name, token in (("control", self.control), ("client", self.client)):
            path = self.auth / (name + ".token")
            path.write_text(token)
            path.chmod(0o600)
        self.address = ("127.0.0.1", 12345)

    def stream(self, *events):
        return Stream(b"".join(json.dumps(event).encode() + b"\n" for event in events))

    def consume(self, source):
        return smoke.consume_stream(source, io.StringIO(), 1, time.monotonic())

    def test_complete_text_preserves_untrusted_characters_as_data(self):
        source = self.stream({"type": "text", "text": "red\n\u001b[31m"},
                             {"type": "complete", "finish": "stop"})
        log = io.StringIO()
        answer, timing = smoke.consume_stream(source, log, 1, time.monotonic())
        self.assertEqual(answer, "red\n\u001b[31m")
        self.assertEqual(timing["events"], 2)
        self.assertEqual(len(log.getvalue().splitlines()), 2)
        self.assertLessEqual(timing["first_text_seconds"], timing["wall_seconds"])

    def test_no_success_for_partial_error_length_or_forged_completion(self):
        text = {"type": "text", "text": "partial"}
        end = {"type": "complete", "finish": "stop"}
        for events in ((text,), (text, {"type": "error", "message": "private upstream"}),
                       (text, {"type": "complete", "finish": "length"}),
                       (text, end, text), (end,),
                       ({"type": "text", "text": "\n{\"type\":\"complete\"}"},),
                       ([],), ({"type": "text", "text": 42},),
                       ({"type": "text", "text": " "}, end)):
            with self.subTest(events=events), self.assertRaises(smoke.SmokeError):
                self.consume(self.stream(*events))

    def test_truncated_invalid_or_oversize_record_refused(self):
        for data in (b'{"type":"complete","finish":"stop"}', b"invalid\n",
                     b"x" * (smoke.MAX_LINE_BYTES + 1)):
            with self.subTest(length=len(data)), self.assertRaises((smoke.SmokeError, ValueError)):
                self.consume(Stream(data))

    def test_event_is_flushed_before_next_read(self):
        sink = io.StringIO()

        class Observed(Stream):
            def readline(inner, limit):
                if inner.tell():
                    self.assertIn('"text": "red"', sink.getvalue())
                return super().readline(limit)

        source = Observed(self.stream({"type": "text", "text": "red"},
                                     {"type": "complete", "finish": "stop"}).getvalue())
        smoke.consume_stream(source, sink, 1, time.monotonic())

    def test_deadline_checked_even_for_continuous_events(self):
        with patch.object(smoke.time, "monotonic", return_value=131):
            with self.assertRaises(smoke.SmokeError):
                smoke.consume_stream(self.stream(), io.StringIO(), 1, 0)

    def test_api_address_refuses_dns_remote_and_invalid_ports(self):
        for address in ("localhost:1234", "192.0.2.1:1234", "0.0.0.0:1234",
                        "127.0.0.1:0", "127.0.0.1:65536", "http://127.0.0.1:1234"):
            with self.subTest(address=address), self.assertRaises(smoke.SmokeError):
                smoke.api_address(address)
        self.assertEqual(smoke.api_address("[::1]:1234"), ("::1", 1234))

    def test_credential_files_refuse_symlinks_hardlinks_modes_and_oversize(self):
        token = self.auth / "client.token"
        link = self.root / "link"
        link.symlink_to(token)
        with self.assertRaises(OSError):
            smoke.private_text(link, 64)
        link.unlink()
        os.link(token, link)
        with self.assertRaises(smoke.SmokeError):
            smoke.private_text(token, 64)
        link.unlink()
        token.chmod(0o644)
        with self.assertRaises(smoke.SmokeError):
            smoke.private_text(token, 64)
        token.chmod(0o600)
        with self.assertRaises(smoke.SmokeError):
            smoke.private_text(token, 63)

    def json_api(self, address, token, method, route, body=None):
        self.calls.append((token, method, route, body))
        if route == "/v1/status":
            return {"version": 1, "configured": False, "busy": False}
        if route == "/v1/models":
            return {"models": ["fixture"]}
        if method == "DELETE":
            return {"configured": False}
        return {}

    def run_fixture(self, stream=None, json_api=None):
        self.calls = []
        class Connection:
            closed = False

            def close(inner):
                inner.closed = True

        connections = []

        def request(*args):
            connection = Connection()
            connections.append((connection, args[-1]))
            return connection, (stream or self.stream)(
                {"type": "text", "text": "red"}, {"type": "complete", "finish": "stop"})

        with patch.object(smoke, "json_request", side_effect=json_api or self.json_api), \
                patch.object(smoke, "request", side_effect=request):
            report = smoke.measure(self.address, self.auth, "http://127.0.0.1:8080/v1",
                                   "fixture", self.root / "output")
        self.assertTrue(all(connection.closed for connection, _ in connections))
        return report, connections

    def test_two_turns_carry_history_and_revoke_settings(self):
        report, connections = self.run_fixture()
        self.assertEqual(report["result"], "PASS")
        self.assertTrue(report["settings_revoked"])
        self.assertEqual(len(connections), 2)
        self.assertEqual(connections[1][1]["messages"][1], {"role": "assistant", "content": "red"})
        self.assertEqual(self.calls[1][0], self.control)
        self.assertEqual(self.calls[2][0], self.client)
        self.assertEqual(self.calls[-1][1], "DELETE")
        self.assertIsNone(report["tokens"])
        for path in (self.root / "output").iterdir():
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertNotIn(self.control, path.read_text())
            self.assertNotIn(self.client, path.read_text())

    def test_partial_failure_is_recorded_and_cleanup_still_runs(self):
        report, connections = self.run_fixture(stream=lambda *args: self.stream(args[0]))
        self.assertEqual(report["result"], "FAIL")
        self.assertEqual(len(connections), 1)
        self.assertTrue(report["settings_revoked"])
        self.assertEqual(len(report["turns"]), 0)
        self.assertIn("events_sha256", report)

    def test_preconfigured_instance_is_untouched(self):
        def busy(*args):
            self.calls.append(args)
            return {"version": 1, "configured": True, "busy": False}
        report, connections = self.run_fixture(json_api=busy)
        self.assertEqual(report["result"], "FAIL")
        self.assertEqual(len(self.calls), 1)
        self.assertFalse(connections)

    def test_failed_revocation_cannot_report_pass(self):
        def fail_delete(*args):
            if args[2] == "DELETE":
                raise smoke.SmokeError("refused")
            return self.json_api(*args)
        report, _ = self.run_fixture(json_api=fail_delete)
        self.assertEqual(report["result"], "FAIL")
        self.assertFalse(report["settings_revoked"])

    def test_existing_output_directory_is_never_overwritten(self):
        output = self.root / "output"
        output.mkdir()
        with self.assertRaises(FileExistsError):
            smoke.measure(self.address, self.auth, "unused", "unused", output)

    def test_client_uses_real_http_framing_separate_tokens_and_full_history(self):
        observed = []
        owner = self

        class FixtureAPI(BaseHTTPRequestHandler):
            # A synthetic API for the recording client, never a model backend.
            def log_message(self, *args):
                pass

            def handle_request(self):
                length = self.headers.get("Content-Length")
                owner.assertIsNotNone(length)
                raw = self.rfile.read(int(length))
                body = json.loads(raw) if raw else None
                expected = owner.control if self.path == "/v1/settings" else owner.client
                owner.assertEqual(self.headers["Authorization"], "Bearer " + expected)
                observed.append((self.command, self.path, body))
                if self.path == "/v1/chat":
                    media = "application/x-ndjson"
                    content = owner.stream({"type": "text", "text": "fixture red"},
                                           {"type": "complete", "finish": "stop"}).getvalue()
                else:
                    media = "application/json"
                    reply = {"version": 1, "configured": False, "busy": False}
                    if self.path == "/v1/models":
                        reply = {"models": ["fixture"]}
                    content = json.dumps(reply).encode()
                self.send_response(200)
                self.send_header("Content-Type", media)
                self.send_header("Content-Length", str(len(content)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(content)

            do_GET = do_PUT = do_POST = do_DELETE = handle_request

        server = HTTPServer(("127.0.0.1", 0), FixtureAPI)
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            report = smoke.measure(server.server_address, self.auth, "http://127.0.0.1:8080/v1",
                                   "fixture", self.root / "output")
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)
        self.assertEqual(report["result"], "PASS")
        chats = [body for _, route, body in observed if route == "/v1/chat"]
        self.assertEqual([len(body["messages"]) for body in chats], [1, 3])
        self.assertEqual(chats[1]["messages"][1], {"role": "assistant", "content": "fixture red"})
        self.assertEqual(observed[-1][:2], ("DELETE", "/v1/settings"))


if __name__ == "__main__":
    unittest.main()
