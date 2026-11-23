#!/usr/bin/env python3
"""Minimal mock server for integration testing.

Implements a small subset of SEP-6 and SEP-10 endpoints using the Python
standard library (no external deps).

Supported endpoints:
- GET  /auth?account=<G...>   SEP-10 step 1 — returns a signed challenge envelope
- POST /auth                  SEP-10 step 2 — accepts the signed envelope, returns a JWT
- POST /deposit
- POST /withdraw
- POST /transaction

Also supports:
- OPTIONS preflight for CORS
- GET /health

SEP-10 two-step flow
---------------------
Step 1: GET /auth?account=G...
    Response: {"transaction": "<mock-XDR-challenge>", "network_passphrase": "..."}

Step 2: POST /auth  body: {"transaction": "<signed-XDR>"}
    Response: {"token": "<mock-JWT>"}

The mock is intentionally lenient about input format (JSON body + query
params) to accommodate slightly different client implementations.
"""

from __future__ import annotations

import base64
import json
import secrets
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import parse_qs, urlparse


class _State:
    def __init__(self):
        self._lock = threading.Lock()
        self._next_tx = 1
        # tx_id -> dict (transaction_id, kind?, status, amounts, message)
        self._tx = {}

    def new_tx_id(self) -> str:
        with self._lock:
            tx_id = f"txn-{self._next_tx:06d}"
            self._next_tx += 1
            return tx_id

    def put_tx(self, tx_id: str, record: dict) -> None:
        with self._lock:
            self._tx[tx_id] = record

    def get_tx(self, tx_id: str) -> dict | None:
        with self._lock:
            return self._tx.get(tx_id)


STATE = _State()


def _json_body(handler: BaseHTTPRequestHandler) -> dict:
    length = int(handler.headers.get("Content-Length") or 0)
    if length <= 0:
        return {}
    raw = handler.rfile.read(length)
    if not raw:
        return {}
    try:
        return json.loads(raw.decode("utf-8"))
    except Exception:
        return {}


def _send_json(handler: BaseHTTPRequestHandler, status_code: int, payload: dict) -> None:
    body = json.dumps(payload).encode("utf-8")
    handler.send_response(status_code)
    handler.send_header("Access-Control-Allow-Origin", "*")
    handler.send_header("Access-Control-Allow-Headers", "Content-Type")
    handler.send_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
    handler.send_header("Content-Type", "application/json")
    handler.send_header("Content-Length", str(len(body)))
    handler.end_headers()
    handler.wfile.write(body)


def _parse_query(handler: BaseHTTPRequestHandler) -> dict:
    parsed = urlparse(handler.path)
    q = parse_qs(parsed.query)
    # flatten single values
    out = {}
    for k, v in q.items():
        out[k] = v[0] if isinstance(v, list) and v else ""
    return out


class MockAnchorHandler(BaseHTTPRequestHandler):
    server_version = "MockAnchor/1.0"

    def log_message(self, fmt, *args):
        # Keep tests quiet
        return

    def _handle_options(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        self.end_headers()

    def do_OPTIONS(self):
        self._handle_options()

    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path == "/health":
            _send_json(self, 200, {"status": "ok"})
            return
        if parsed.path == "/auth":
            self._get_auth_challenge()
            return
        _send_json(self, 404, {"error": "not_found", "path": parsed.path})

    def do_POST(self):
        parsed = urlparse(self.path)
        path = parsed.path
        query = _parse_query(self)
        body = _json_body(self)

        # Convenience: allow setting tx status through either body or query.
        force_status = (
            body.get("status")
            or body.get("transaction_status")
            or query.get("status")
            or query.get("transaction_status")
        )

        if path == "/deposit":
            self._post_deposit(body, query, force_status)
            return
        if path == "/withdraw":
            self._post_withdraw(body, query, force_status)
            return
        if path == "/transaction":
            self._post_transaction(body, query, force_status)
            return
        if path == "/auth":
            self._post_auth(body, query)
            return

        _send_json(self, 404, {"error": "not_found", "path": path})

    def _post_deposit(self, body: dict, query: dict, force_status: str | None) -> None:
        tx_id = body.get("transaction_id") or query.get("transaction_id") or STATE.new_tx_id()
        how = body.get("how") or query.get("how") or "Send to mock deposit address"

        status = force_status or body.get("status") or "pending_external"

        record = {
            "transaction_id": tx_id,
            "kind": "deposit",
            "status": status,
            "amount_in": body.get("amount") or body.get("amount_in"),
            "amount_out": body.get("amount_out"),
            "amount_fee": body.get("fee") or body.get("amount_fee"),
            "message": body.get("message") or None,
        }
        STATE.put_tx(tx_id, record)

        resp = {
            "transaction_id": tx_id,
            "how": how,
            "extra_info": body.get("extra_info") or query.get("extra_info"),
            "min_amount": body.get("min_amount") or query.get("min_amount"),
            "max_amount": body.get("max_amount") or query.get("max_amount"),
            "fee_fixed": body.get("fee_fixed") or query.get("fee_fixed"),
            "fee_percent": body.get("fee_percent") or query.get("fee_percent"),
            "status": status,
        }

        # Strip null-like values to keep responses closer to real anchors
        resp = {k: v for k, v in resp.items() if v is not None}
        _send_json(self, 200, resp)

    def _post_withdraw(self, body: dict, query: dict, force_status: str | None) -> None:
        tx_id = body.get("transaction_id") or query.get("transaction_id") or STATE.new_tx_id()
        account_id = body.get("account_id") or query.get("account_id") or "GABC1234567890MOCKMOCKMOCKMOCKMOCKMOCKMOCKMOCKMOCK"[:56]

        status = force_status or body.get("status") or "pending_user"

        record = {
            "transaction_id": tx_id,
            "kind": "withdraw",
            "status": status,
            "amount_in": body.get("amount") or body.get("amount_in"),
            "amount_out": body.get("amount_out"),
            "amount_fee": body.get("fee") or body.get("amount_fee"),
            "message": body.get("message") or None,
        }
        STATE.put_tx(tx_id, record)

        resp = {
            "transaction_id": tx_id,
            "account_id": account_id,
            "dest_account_id": body.get("dest_account_id") or query.get("dest_account_id"),
            "memo": body.get("memo") or query.get("memo"),
            "memo_type": body.get("memo_type") or query.get("memo_type"),
            "min_amount": body.get("min_amount") or query.get("min_amount"),
            "max_amount": body.get("max_amount") or query.get("max_amount"),
            "fee_fixed": body.get("fee_fixed") or query.get("fee_fixed"),
            "fee_percent": body.get("fee_percent") or query.get("fee_percent"),
            "status": status,
        }
        resp = {k: v for k, v in resp.items() if v is not None}
        _send_json(self, 200, resp)

    def _post_transaction(self, body: dict, query: dict, force_status: str | None) -> None:
        tx_id = (
            body.get("transaction_id")
            or body.get("id")
            or query.get("transaction_id")
            or query.get("id")
        )
        if not tx_id:
            _send_json(self, 400, {"error": "missing_transaction_id"})
            return

        record = STATE.get_tx(tx_id)
        if record is None:
            # Mimic a typical anchor behavior: not found
            _send_json(self, 404, {"error": "not_found", "transaction_id": tx_id})
            return

        if force_status is not None:
            record = dict(record)
            record["status"] = force_status
            STATE.put_tx(tx_id, record)

        resp = {
            "transaction_id": tx_id,
            "kind": record.get("kind"),
            "status": record.get("status", "pending"),
            "amount_in": record.get("amount_in"),
            "amount_out": record.get("amount_out"),
            "amount_fee": record.get("amount_fee"),
            "message": record.get("message"),
        }
        resp = {k: v for k, v in resp.items() if v is not None}
        _send_json(self, 200, resp)

    def _get_auth_challenge(self) -> None:
        """SEP-10 step 1: GET /auth?account=G...

        Returns a mock challenge transaction envelope that the client must
        sign with the account's private key before posting back to POST /auth.

        Real anchors return a base64-encoded XDR Stellar transaction.  This
        mock returns a human-readable placeholder so tests can inspect it
        without needing the Stellar SDK.
        """
        query = _parse_query(self)
        account = query.get("account") or query.get("client_domain") or ""

        if not account:
            _send_json(self, 400, {
                "error": "missing_account",
                "message": "account query parameter is required",
            })
            return

        # Produce a deterministic-but-unique nonce per request so the
        # challenge transaction is never reused (mirrors real anchor behaviour).
        nonce = secrets.token_hex(16)
        issued_at = int(time.time())
        # A real anchor would build and sign a Stellar transaction XDR here.
        # We encode a JSON descriptor so test assertions can decode and inspect it.
        challenge_payload = {
            "account": account,
            "nonce": nonce,
            "issued_at": issued_at,
            "home_domain": "mock.example.com",
        }
        mock_xdr = base64.b64encode(
            json.dumps(challenge_payload).encode("utf-8")
        ).decode("ascii")

        _send_json(self, 200, {
            "transaction": mock_xdr,
            "network_passphrase": "Test SDF Network ; September 2015",
        })

    def _post_auth(self, body: dict, query: dict) -> None:
        """SEP-10 step 2: POST /auth

        Accepts the signed challenge transaction and returns a JWT token.

        The client must supply the signed XDR in the ``transaction`` field of
        the JSON body (the field name specified by SEP-10).  For maximum
        compatibility with clients that use different field names, the mock
        also checks several common aliases.
        """
        transaction = (
            body.get("transaction")
            or body.get("envelope")
            or query.get("transaction")
            or query.get("envelope")
        )

        if not transaction:
            _send_json(self, 400, {
                "error": "missing_transaction",
                "message": (
                    "A signed challenge transaction is required. "
                    "Call GET /auth?account=<G...> first to obtain the challenge."
                ),
            })
            return

        # Decode the mock challenge to extract the account so the JWT subject
        # matches what the client expects.  Real anchors verify the signature;
        # we skip that here since this is a test mock.
        account = "unknown"
        try:
            decoded = json.loads(base64.b64decode(transaction).decode("utf-8"))
            account = decoded.get("account", "unknown")
        except Exception:
            # Client may pass a real or hand-crafted XDR — that's fine for a mock.
            pass

        # Build a plausible but clearly mock JWT (header.payload.signature).
        issued_at = int(time.time())
        expires_at = issued_at + 86400  # 24 hours
        header = base64.b64encode(b'{"alg":"none","typ":"JWT"}').decode("ascii").rstrip("=")
        payload_data = {
            "sub": account,
            "iss": "mock.example.com",
            "iat": issued_at,
            "exp": expires_at,
        }
        payload_b64 = base64.b64encode(
            json.dumps(payload_data).encode("utf-8")
        ).decode("ascii").rstrip("=")
        mock_jwt = f"{header}.{payload_b64}.mock-signature"

        _send_json(self, 200, {"token": mock_jwt})


def main() -> None:
    host = "0.0.0.0"
    port = 8080
    print(f"Mock anchor server running on http://localhost:{port}")
    HTTPServer((host, port), MockAnchorHandler).serve_forever()


if __name__ == "__main__":
    main()

