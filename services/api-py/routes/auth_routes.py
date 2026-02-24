import asyncio
import random
import secrets
import smtplib
import string
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from email.mime.text import MIMEText

from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError
import base58

import db
from auth import require_api_key
from config import Settings

router = APIRouter()

EXPIRY = timedelta(minutes=30)
MAX_ATTEMPTS = 5
CHALLENGE_EXPIRY = timedelta(minutes=5)


# ── Helpers ──────────────────────────────────────────────────────────

def _generate_api_key() -> str:
    return "r3l_" + secrets.token_hex(24)


def _generate_code() -> str:
    return "".join(random.choices(string.digits, k=6))


def _send_email(settings: Settings, msg: MIMEText):
    with smtplib.SMTP_SSL(settings.smtp_host, 465) as server:
        server.login(settings.smtp_user, settings.smtp_pass)
        server.send_message(msg)


# ── In-memory stores ────────────────────────────────────────────────

@dataclass
class EmailCode:
    email: str
    code: str
    attempts: int = 0
    created_at: datetime = field(default_factory=datetime.now)


@dataclass
class WalletChallenge:
    nonce: str
    created_at: datetime = field(default_factory=datetime.now)


_email_codes: dict[str, EmailCode] = {}  # keyed by lowercase email
_wallet_challenges: dict[str, WalletChallenge] = {}  # keyed by nonce


def _clean_expired_emails():
    now = datetime.now()
    expired = [k for k, v in _email_codes.items() if now - v.created_at > EXPIRY]
    for k in expired:
        del _email_codes[k]


def _clean_expired_challenges():
    now = datetime.now()
    expired = [k for k, v in _wallet_challenges.items() if now - v.created_at > CHALLENGE_EXPIRY]
    for k in expired:
        del _wallet_challenges[k]


# ── Request models ──────────────────────────────────────────────────

class EmailStartRequest(BaseModel):
    email: str


class EmailVerifyRequest(BaseModel):
    email: str
    code: str


class WalletVerifyRequest(BaseModel):
    pubkey: str
    message: str
    signature: str
    name: str = ""


# ── POST /api/auth/email/start ──────────────────────────────────────

@router.post("/email/start")
async def email_start(req: EmailStartRequest):
    email = req.email.lower().strip()
    if "@" not in email:
        raise HTTPException(400, "invalid email")

    domain = email.split("@", 1)[1]
    if "." not in domain or len(domain) < 3:
        raise HTTPException(400, "invalid email domain")

    _clean_expired_emails()
    code = _generate_code()
    _email_codes[email] = EmailCode(email=email, code=code)

    settings = Settings()
    resp = {"status": "pending", "email": email}

    if settings.smtp_host:
        html_body = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,sans-serif;background:#0a0a0f;color:#e5e5e5;margin:0;padding:40px;">
<div style="max-width:480px;margin:0 auto;background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:40px;text-align:center;">
<h1 style="color:#facc15;margin:0 0 8px;font-size:24px;">R3L Provenance</h1>
<p style="color:#9ca3af;margin:0 0 24px;">Verify your email to create your account.</p>
<p style="color:#e5e5e5;margin:0 0 8px;">Your verification code:</p>
<p style="font-size:36px;font-weight:700;color:#facc15;letter-spacing:8px;margin:0 0 24px;">{code}</p>
<p style="color:#6b7280;font-size:12px;margin:0;">This code expires in 30 minutes.</p>
</div>
</body></html>"""
        from_addr = settings.smtp_from or settings.smtp_user
        msg = MIMEText(html_body, "html")
        msg["Subject"] = "R3L \u2014 Your verification code"
        msg["From"] = from_addr
        msg["To"] = email

        try:
            await asyncio.to_thread(_send_email, settings, msg)
        except Exception as e:
            raise HTTPException(500, f"failed to send email: {e}")
    else:
        resp["dev_code"] = code

    return resp


# ── POST /api/auth/email/verify ─────────────────────────────────────

@router.post("/email/verify")
async def email_verify(req: EmailVerifyRequest):
    email = req.email.lower().strip()
    entry = _email_codes.get(email)

    if not entry:
        raise HTTPException(404, "no verification pending for this email")

    if datetime.now() - entry.created_at > EXPIRY:
        del _email_codes[email]
        raise HTTPException(410, "code expired \u2014 request a new code")

    if entry.attempts >= MAX_ATTEMPTS:
        del _email_codes[email]
        raise HTTPException(429, "too many attempts \u2014 request a new code")

    if entry.code != req.code:
        entry.attempts += 1
        remaining = MAX_ATTEMPTS - entry.attempts
        if remaining <= 0:
            del _email_codes[email]
            raise HTTPException(429, "too many attempts \u2014 request a new code")
        raise HTTPException(
            400,
            f"invalid code \u2014 {remaining} attempt{'s' if remaining != 1 else ''} remaining",
        )

    del _email_codes[email]

    # Check if email already has an account
    existing = await db.get_customer_by_email(email)
    if existing:
        raise HTTPException(
            409,
            "an account already exists for this email \u2014 use your existing API key to log in",
        )

    # Create new customer
    api_key = _generate_api_key()
    name = email.split("@")[0]
    customer = await db.insert_customer(
        name=name, api_key=api_key, email=email, auth_method="email",
    )
    return {
        "api_key": customer["api_key"],
        "email": email,
        "name": customer["name"],
        "existing": False,
    }


# ── GET /api/auth/wallet/challenge ──────────────────────────────────

@router.get("/wallet/challenge")
async def wallet_challenge():
    _clean_expired_challenges()
    nonce = secrets.token_hex(16)
    _wallet_challenges[nonce] = WalletChallenge(nonce=nonce)
    return {
        "challenge": f"R3L-auth:{nonce}",
        "expires_in": 300,
    }


# ── POST /api/auth/wallet/verify ────────────────────────────────────

@router.post("/wallet/verify")
async def wallet_verify(req: WalletVerifyRequest):
    # Validate message format
    if not req.message.startswith("R3L-auth:"):
        raise HTTPException(400, "invalid challenge format")
    nonce = req.message.split(":", 1)[1]

    # Check nonce
    challenge = _wallet_challenges.get(nonce)
    if not challenge:
        raise HTTPException(400, "invalid or expired challenge")
    if datetime.now() - challenge.created_at > CHALLENGE_EXPIRY:
        del _wallet_challenges[nonce]
        raise HTTPException(400, "challenge expired")
    del _wallet_challenges[nonce]  # single use

    # Verify Ed25519 signature
    try:
        pubkey_bytes = base58.b58decode(req.pubkey)
        sig_bytes = base58.b58decode(req.signature)
    except Exception:
        raise HTTPException(400, "invalid base58 encoding")

    if len(pubkey_bytes) != 32:
        raise HTTPException(400, "invalid pubkey length")

    try:
        verify_key = VerifyKey(pubkey_bytes)
        verify_key.verify(req.message.encode(), sig_bytes)
    except BadSignatureError:
        raise HTTPException(400, "invalid signature")

    # Check if wallet already has an account
    existing = await db.get_customer_by_wallet(req.pubkey)
    if existing:
        raise HTTPException(
            409,
            "an account already exists for this wallet \u2014 use your existing API key to log in",
        )

    # Create new customer
    api_key = _generate_api_key()
    name = req.name or req.pubkey[:8]
    customer = await db.insert_customer(
        name=name, api_key=api_key, wallet_pubkey=req.pubkey, auth_method="wallet",
    )
    return {
        "api_key": customer["api_key"],
        "pubkey": req.pubkey,
        "name": customer["name"],
        "existing": False,
    }


# ── GET /api/auth/me ────────────────────────────────────────────────

@router.get("/me")
async def get_me(caller: dict = Depends(require_api_key)):
    """Unified identity endpoint for any auth type."""
    if caller.get("type") == "org":
        org = await db.get_organization_by_id(caller["org_id"])
        from did import get_all_dids_for_org
        dids = get_all_dids_for_org(org) if org else {}
        return {
            "type": "org",
            "org": org,
            "dids": dids,
            "role": caller.get("role"),
            "email": caller.get("email"),
        }
    else:
        return {
            "type": "individual",
            "name": caller.get("name"),
            "email": caller.get("email"),
            "wallet_pubkey": caller.get("wallet_pubkey"),
            "auth_method": caller.get("auth_method"),
        }


# ── POST /api/auth/link/email/start ──────────────────────────────

@router.post("/link/email/start")
async def link_email_start(req: EmailStartRequest, caller: dict = Depends(require_api_key)):
    if caller.get("email"):
        raise HTTPException(400, "email already linked to this account")

    email = req.email.lower().strip()
    if "@" not in email:
        raise HTTPException(400, "invalid email")

    domain = email.split("@", 1)[1]
    if "." not in domain or len(domain) < 3:
        raise HTTPException(400, "invalid email domain")

    existing = await db.get_customer_by_email(email)
    if existing:
        raise HTTPException(409, "this email is already linked to another account")

    _clean_expired_emails()
    code = _generate_code()
    _email_codes[email] = EmailCode(email=email, code=code)

    settings = Settings()
    resp = {"status": "pending", "email": email}

    if settings.smtp_host:
        html_body = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,sans-serif;background:#0a0a0f;color:#e5e5e5;margin:0;padding:40px;">
<div style="max-width:480px;margin:0 auto;background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:40px;text-align:center;">
<h1 style="color:#facc15;margin:0 0 8px;font-size:24px;">R3L Provenance</h1>
<p style="color:#9ca3af;margin:0 0 24px;">Verify your email to link it to your account.</p>
<p style="color:#e5e5e5;margin:0 0 8px;">Your verification code:</p>
<p style="font-size:36px;font-weight:700;color:#facc15;letter-spacing:8px;margin:0 0 24px;">{code}</p>
<p style="color:#6b7280;font-size:12px;margin:0;">This code expires in 30 minutes.</p>
</div>
</body></html>"""
        from_addr = settings.smtp_from or settings.smtp_user
        msg = MIMEText(html_body, "html")
        msg["Subject"] = "R3L \u2014 Link your email"
        msg["From"] = from_addr
        msg["To"] = email

        try:
            await asyncio.to_thread(_send_email, settings, msg)
        except Exception as e:
            raise HTTPException(500, f"failed to send email: {e}")
    else:
        resp["dev_code"] = code

    return resp


# ── POST /api/auth/link/email/verify ─────────────────────────────

@router.post("/link/email/verify")
async def link_email_verify(req: EmailVerifyRequest, caller: dict = Depends(require_api_key)):
    if caller.get("email"):
        raise HTTPException(400, "email already linked to this account")

    email = req.email.lower().strip()
    entry = _email_codes.get(email)

    if not entry:
        raise HTTPException(404, "no verification pending for this email")

    if datetime.now() - entry.created_at > EXPIRY:
        del _email_codes[email]
        raise HTTPException(410, "code expired \u2014 request a new code")

    if entry.attempts >= MAX_ATTEMPTS:
        del _email_codes[email]
        raise HTTPException(429, "too many attempts \u2014 request a new code")

    if entry.code != req.code:
        entry.attempts += 1
        remaining = MAX_ATTEMPTS - entry.attempts
        if remaining <= 0:
            del _email_codes[email]
            raise HTTPException(429, "too many attempts \u2014 request a new code")
        raise HTTPException(
            400,
            f"invalid code \u2014 {remaining} attempt{'s' if remaining != 1 else ''} remaining",
        )

    del _email_codes[email]

    # Double-check not taken (race condition guard)
    existing = await db.get_customer_by_email(email)
    if existing:
        raise HTTPException(409, "this email is already linked to another account")

    await db.link_email_to_customer(caller["id"], email)
    return {"status": "linked", "email": email}


# ── POST /api/auth/link/wallet ────────────────────────────────────

@router.post("/link/wallet")
async def link_wallet(req: WalletVerifyRequest, caller: dict = Depends(require_api_key)):
    if caller.get("wallet_pubkey"):
        raise HTTPException(400, "wallet already linked to this account")

    # Validate message format (reuse same challenge)
    if not req.message.startswith("R3L-auth:"):
        raise HTTPException(400, "invalid challenge format")
    nonce = req.message.split(":", 1)[1]

    # Check nonce
    challenge = _wallet_challenges.get(nonce)
    if not challenge:
        raise HTTPException(400, "invalid or expired challenge")
    if datetime.now() - challenge.created_at > CHALLENGE_EXPIRY:
        del _wallet_challenges[nonce]
        raise HTTPException(400, "challenge expired")
    del _wallet_challenges[nonce]

    # Verify Ed25519 signature
    try:
        pubkey_bytes = base58.b58decode(req.pubkey)
        sig_bytes = base58.b58decode(req.signature)
    except Exception:
        raise HTTPException(400, "invalid base58 encoding")

    if len(pubkey_bytes) != 32:
        raise HTTPException(400, "invalid pubkey length")

    try:
        verify_key = VerifyKey(pubkey_bytes)
        verify_key.verify(req.message.encode(), sig_bytes)
    except BadSignatureError:
        raise HTTPException(400, "invalid signature")

    # Check wallet not taken
    existing = await db.get_customer_by_wallet(req.pubkey)
    if existing:
        raise HTTPException(409, "this wallet is already linked to another account")

    await db.link_wallet_to_customer(caller["id"], req.pubkey)
    return {"status": "linked", "wallet_pubkey": req.pubkey}
