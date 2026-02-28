"""R3L Edge Node CLI — verify files locally, attest on-chain via the R3L API.

Usage:
  r3l-edge register    [--name NAME] [--keypair PATH] [--api URL]
  r3l-edge attest      <file> [--keypair PATH] [--api URL] [--api-key KEY]
  r3l-edge attest-url  <url> [--api URL] [--api-key KEY] [--no-store] [--header K:V]
  r3l-edge attest-text <text> [--title TITLE] [--api URL] [--api-key KEY] [--no-store]
  r3l-edge lookup      <hash> [--api URL]
  r3l-edge query       <hash> [--api URL]

Environment variables (alternative to flags):
  R3L_API_URL    — API base URL (default: http://localhost:3001)
  R3L_API_KEY    — API key from registration
  R3L_KEYPAIR    — path to edge-keypair.json
"""
import argparse
import json
import os
import subprocess
import sys

from .client import R3LEdgeClient


def _env(name: str, default: str = "") -> str:
    return os.environ.get(name, default)


def _services_dir() -> str:
    """Resolve the services/ directory relative to this package.
    Layout: services/edge-nodes/python/src/r3l_edge/cli.py
    """
    pkg_dir = os.path.dirname(os.path.abspath(__file__))
    return os.path.abspath(os.path.join(pkg_dir, "..", "..", "..", ".."))


def _project_root() -> str:
    return os.path.abspath(os.path.join(_services_dir(), ".."))


def _find_verifier() -> str:
    """Locate the verifier binary."""
    pkg_dir = os.path.dirname(os.path.abspath(__file__))
    candidates = [
        # Bundled inside the pip package
        os.path.join(pkg_dir, "bin", "verifier"),
        # Relative to package: services/edge-nodes/python -> services/verifier
        os.path.join(_services_dir(), "verifier", "target", "release", "verifier"),
        # Relative to cwd (running from project root)
        os.path.join(os.getcwd(), "services", "verifier", "target", "release", "verifier"),
    ]
    for c in candidates:
        if os.path.isfile(c) and os.access(c, os.X_OK):
            return c
    return ""


def _find_trust_dir() -> str:
    """Locate the trust directory."""
    candidates = [
        os.path.join(_project_root(), "data", "trust"),
        os.path.join(os.getcwd(), "data", "trust"),
    ]
    for c in candidates:
        if os.path.isdir(c):
            return c
    return ""


def cmd_register(args):
    kp_path = args.keypair or _env("R3L_KEYPAIR", "edge-keypair.json")
    client = R3LEdgeClient(api_url=args.api or _env("R3L_API_URL", "http://localhost:3001"))

    if os.path.exists(kp_path):
        client.load_keypair(kp_path)
        print(f"Using existing keypair: {kp_path}")
    else:
        client.generate_keypair(kp_path)
        print(f"Generated keypair: {kp_path}")

    resp = client.register(name=args.name)

    print(f"\nRegistered successfully!")
    print(f"  Pubkey:  {resp['pubkey']}")
    print(f"  Name:    {resp['name']}")
    print(f"  API Key: {resp['api_key']}")
    print(f"\nSave your API key:")
    print(f"  export R3L_API_KEY={resp['api_key']}")


def cmd_attest(args):
    filepath = args.file
    if not os.path.isfile(filepath):
        print(f"File not found: {filepath}", file=sys.stderr)
        sys.exit(1)

    api_key = args.api_key or _env("R3L_API_KEY")
    if not api_key:
        print("No API key. Set R3L_API_KEY or pass --api-key, or run 'r3l-edge register' first.", file=sys.stderr)
        sys.exit(1)

    kp_path = args.keypair or _env("R3L_KEYPAIR", "edge-keypair.json")
    client = R3LEdgeClient(
        api_url=args.api or _env("R3L_API_URL", "http://localhost:3001"),
        api_key=api_key,
        keypair_path=kp_path if os.path.exists(kp_path) else None,
    )

    verifier = args.verifier or _env("R3L_VERIFIER") or _find_verifier()
    if not verifier:
        print("Cannot find verifier binary. Pass --verifier or set R3L_VERIFIER.", file=sys.stderr)
        sys.exit(1)
    trust_dir = args.trust_dir or _env("R3L_TRUST_DIR") or _find_trust_dir()

    print(f"Verifying: {filepath}")
    try:
        resp = client.verify_and_attest(filepath, verifier_bin=verifier, trust_dir=trust_dir)
    except RuntimeError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if resp.get("existing"):
        print(f"\nAttestation already exists:")
    else:
        print(f"\nAttestation created:")
    print(f"  Content hash: {resp['content_hash']}")
    print(f"  PDA:          {resp['attestation_pda']}")
    if resp.get("signature"):
        print(f"  Tx signature: {resp['signature']}")
    if resp.get("wallet_pubkey"):
        print(f"  Wallet:       {resp['wallet_pubkey']}")


def cmd_attest_url(args):
    api_key = args.api_key or _env("R3L_API_KEY")
    if not api_key:
        print("No API key. Set R3L_API_KEY or pass --api-key, or run 'r3l-edge register' first.", file=sys.stderr)
        sys.exit(1)

    client = R3LEdgeClient(
        api_url=args.api or _env("R3L_API_URL", "http://localhost:3001"),
        api_key=api_key,
    )

    # Parse --header flags into dict
    headers = None
    if args.header:
        headers = {}
        for h in args.header:
            idx = h.find(":")
            if idx <= 0:
                print(f"Invalid header (expected 'Key: Value'): {h}", file=sys.stderr)
                sys.exit(1)
            headers[h[:idx].strip()] = h[idx + 1:].strip()

    print(f"Attesting URL: {args.url}")
    try:
        resp = client.attest_url(args.url, store_content=not args.no_store, headers=headers)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if resp.get("existing"):
        print(f"\nAttestation already exists:")
    else:
        print(f"\nAttestation created:")
    print(f"  Content hash: {resp['content_hash']}")
    print(f"  PDA:          {resp['attestation_pda']}")
    if resp.get("signature"):
        print(f"  Tx signature: {resp['signature']}")


def cmd_attest_text(args):
    api_key = args.api_key or _env("R3L_API_KEY")
    if not api_key:
        print("No API key. Set R3L_API_KEY or pass --api-key, or run 'r3l-edge register' first.", file=sys.stderr)
        sys.exit(1)

    client = R3LEdgeClient(
        api_url=args.api or _env("R3L_API_URL", "http://localhost:3001"),
        api_key=api_key,
    )

    # Read from stdin if text is "-"
    text = args.text
    if text == "-":
        text = sys.stdin.read()

    print(f"Attesting text ({len(text)} chars)...")
    try:
        resp = client.attest_text(text, title=args.title, store_content=not args.no_store)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if resp.get("existing"):
        print(f"\nAttestation already exists:")
    else:
        print(f"\nAttestation created:")
    print(f"  Content hash: {resp['content_hash']}")
    print(f"  PDA:          {resp['attestation_pda']}")
    if resp.get("signature"):
        print(f"  Tx signature: {resp['signature']}")


def cmd_lookup(args):
    client = R3LEdgeClient(api_url=args.api or _env("R3L_API_URL", "http://localhost:3001"))
    resp = client.lookup(args.hash)
    print(json.dumps(resp, indent=2))


def cmd_query(args):
    client = R3LEdgeClient(api_url=args.api or _env("R3L_API_URL", "http://localhost:3001"))
    resp = client.query(args.hash)
    print(json.dumps(resp, indent=2))


def main():
    parser = argparse.ArgumentParser(
        prog="r3l-edge",
        description="R3L Edge Node CLI — verify files locally, attest on-chain",
    )
    sub = parser.add_subparsers(dest="command")

    # register
    reg = sub.add_parser("register", help="Register edge node and get API key")
    reg.add_argument("--name", help="Node name (default: edge-<pubkey prefix>)")
    reg.add_argument("--keypair", help="Path to keypair JSON (default: edge-keypair.json)")
    reg.add_argument("--api", help="API URL (default: $R3L_API_URL or http://localhost:3001)")

    # attest
    att = sub.add_parser("attest", help="Verify a file locally and submit attestation")
    att.add_argument("file", help="Path to media file")
    att.add_argument("--keypair", help="Path to keypair JSON for wallet signing")
    att.add_argument("--api", help="API URL")
    att.add_argument("--api-key", help="API key (default: $R3L_API_KEY)")
    att.add_argument("--verifier", help="Path to verifier binary (default: auto-detect or $R3L_VERIFIER)")
    att.add_argument("--trust-dir", help="Path to trust directory (default: auto-detect or $R3L_TRUST_DIR)")

    # attest-url
    au = sub.add_parser("attest-url", help="Attest a URL (API fetches and hashes the content)")
    au.add_argument("url", help="URL to attest")
    au.add_argument("--api", help="API URL")
    au.add_argument("--api-key", help="API key (default: $R3L_API_KEY)")
    au.add_argument("--no-store", action="store_true", help="Don't store content on the server")
    au.add_argument("--header", "-H", action="append", metavar="KEY:VALUE",
                    help="Header forwarded when fetching the URL (repeatable, e.g. -H 'Authorization: Bearer tok')")

    # attest-text
    at = sub.add_parser("attest-text", help="Attest text content (use '-' to read from stdin)")
    at.add_argument("text", help="Text to attest (use '-' to read from stdin)")
    at.add_argument("--title", help="Optional title for the text")
    at.add_argument("--api", help="API URL")
    at.add_argument("--api-key", help="API key (default: $R3L_API_KEY)")
    at.add_argument("--no-store", action="store_true", help="Don't store content on the server")

    # lookup
    lk = sub.add_parser("lookup", help="Look up attestation by content hash (raw)")
    lk.add_argument("hash", help="Content hash (hex)")
    lk.add_argument("--api", help="API URL")

    # query
    qr = sub.add_parser("query", help="Query structured trust verdict for a content hash")
    qr.add_argument("hash", help="Content hash (hex)")
    qr.add_argument("--api", help="API URL")

    args = parser.parse_args()
    if args.command is None:
        parser.print_help()
        sys.exit(0)

    cmds = {
        "register": cmd_register,
        "attest": cmd_attest,
        "attest-url": cmd_attest_url,
        "attest-text": cmd_attest_text,
        "lookup": cmd_lookup,
        "query": cmd_query,
    }
    cmds[args.command](args)
