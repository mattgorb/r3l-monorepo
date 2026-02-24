#!/usr/bin/env python3
"""Sign a wallet attestation message for manual entry in the R3L UI.

Usage:
  python scripts/sign-message.py <content_hash>

Generates a fresh keypair, signs "R3L: attest <content_hash>",
and prints the pubkey + signature in base58 for pasting into the UI.
"""
import sys
import base58
from nacl.signing import SigningKey

if len(sys.argv) != 2:
    print("Usage: python scripts/sign-message.py <content_hash>")
    sys.exit(1)

content_hash = sys.argv[1]
message = f"R3L: attest {content_hash}"

key = SigningKey.generate()
sig = key.sign(message.encode()).signature

print(f"Message: {message}")
print(f"Pubkey:    {base58.b58encode(key.verify_key.encode()).decode()}")
print(f"Signature: {base58.b58encode(sig).decode()}")
