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

import db
from auth import require_api_key, require_org_admin
from config import Settings
from did import get_all_dids_for_org

router = APIRouter()

EXPIRY = timedelta(minutes=30)


# ── In-memory email code store ──────────────────────────────────────

MAX_ATTEMPTS = 5


@dataclass
class EmailCode:
    domain: str
    email: str
    code: str
    attempts: int = 0
    created_at: datetime = field(default_factory=datetime.now)


_email_codes: dict[str, EmailCode] = {}  # keyed by email


def _clean_expired():
    now = datetime.now()
    expired = [k for k, v in _email_codes.items() if now - v.created_at > EXPIRY]
    for k in expired:
        del _email_codes[k]


def _generate_api_key() -> str:
    return "r3l_" + secrets.token_hex(24)


def _generate_code() -> str:
    return "".join(random.choices(string.digits, k=6))


# ── Request models ──────────────────────────────────────────────────

class RegisterRequest(BaseModel):
    domain: str
    method: str  # "dns" | "email"
    admin_email: str | None = None
    name: str | None = None


class VerifyDnsRequest(BaseModel):
    domain: str


class VerifyEmailRequest(BaseModel):
    email: str
    code: str


class CreateKeyRequest(BaseModel):
    email: str | None = None
    role: str = "attester"


# ── Blocked domains ──────────────────────────────────────────────────

BLOCKED_DOMAINS = {
    # Freemail
    "gmail.com", "googlemail.com", "yahoo.com", "yahoo.co.uk",
    "hotmail.com", "outlook.com", "live.com", "msn.com",
    "aol.com", "icloud.com", "me.com", "mac.com",
    "protonmail.com", "proton.me", "mail.com", "zoho.com",
    "yandex.com", "yandex.ru", "gmx.com", "gmx.net",
    "tutanota.com", "tuta.com", "fastmail.com",
    # Temp/disposable
    "mailinator.com", "guerrillamail.com", "tempmail.com",
    "throwaway.email", "sharklasers.com", "guerrillamailblock.com",
    "grr.la", "dispostable.com", "yopmail.com", "10minutemail.com",
    # Major platforms (not org domains)
    "facebook.com", "twitter.com", "instagram.com", "tiktok.com",
    "amazon.com", "apple.com", "microsoft.com", "google.com",
}


# ── POST /api/org/register ──────────────────────────────────────────

@router.post("/register")
async def register(req: RegisterRequest):
    domain = req.domain.lower().strip()
    if not domain or "." not in domain:
        raise HTTPException(400, "invalid domain")
    if domain in BLOCKED_DOMAINS:
        raise HTTPException(400, f"{domain} is a public email provider and cannot be registered as an organization")

    existing = await db.get_organization_by_domain(domain)
    if existing and existing["verified"]:
        raise HTTPException(409, "domain already registered and verified")

    if req.method == "dns":
        dns_token = "r3l-verify=" + secrets.token_hex(16)
        if existing:
            # Update existing unverified org
            org = existing
        else:
            org = await db.insert_organization(
                domain=domain,
                name=req.name,
                verification_method="dns",
                dns_token=dns_token,
                admin_email=req.admin_email,
            )
        # For existing orgs, update dns_token in-place
        if existing:
            async with db.get_session() as session:
                from sqlalchemy import select
                from models import Organization
                stmt = select(Organization).where(Organization.domain == domain)
                row = (await session.execute(stmt)).scalar_one_or_none()
                if row:
                    row.dns_token = dns_token
                    row.verification_method = "dns"
                    if req.admin_email:
                        row.admin_email = req.admin_email
                    if req.name:
                        row.name = req.name
                    await session.commit()

        return {
            "status": "pending",
            "method": "dns",
            "domain": domain,
            "instruction": f"Add a TXT record to {domain} with value: {dns_token}",
            "txt_value": dns_token,
        }

    elif req.method == "email":
        if not req.admin_email or "@" not in req.admin_email:
            raise HTTPException(400, "admin_email required for email verification")

        email_domain = req.admin_email.split("@", 1)[1].lower()
        if email_domain != domain:
            raise HTTPException(400, f"email must be @{domain}")

        if not existing:
            await db.insert_organization(
                domain=domain,
                name=req.name,
                verification_method="email",
                admin_email=req.admin_email,
            )

        code = _generate_code()
        _clean_expired()
        _email_codes[req.admin_email.lower()] = EmailCode(
            domain=domain,
            email=req.admin_email.lower(),
            code=code,
        )

        settings = Settings()
        resp = {
            "status": "pending",
            "method": "email",
            "domain": domain,
            "email": req.admin_email,
        }

        if settings.smtp_host:
            html_body = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,sans-serif;background:#0a0a0f;color:#e5e5e5;margin:0;padding:40px;">
<div style="max-width:480px;margin:0 auto;background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:40px;text-align:center;">
<h1 style="color:#facc15;margin:0 0 8px;font-size:24px;">R3L Provenance</h1>
<p style="color:#9ca3af;margin:0 0 24px;">Verify your organization domain.</p>
<p style="color:#e5e5e5;margin:0 0 8px;">Your verification code:</p>
<p style="font-size:36px;font-weight:700;color:#facc15;letter-spacing:8px;margin:0 0 24px;">{code}</p>
<p style="color:#6b7280;font-size:12px;margin:0;">This code expires in 30 minutes.</p>
</div>
</body></html>"""
            from_addr = settings.smtp_from or settings.smtp_user
            msg = MIMEText(html_body, "html")
            msg["Subject"] = "R3L — Organization verification code"
            msg["From"] = from_addr
            msg["To"] = req.admin_email

            try:
                await asyncio.to_thread(_send_email, settings, msg)
            except Exception as e:
                raise HTTPException(500, f"failed to send email: {e}")
        else:
            # Dev mode — return code directly
            resp["dev_code"] = code

        return resp

    else:
        raise HTTPException(400, "method must be 'dns' or 'email'")


def _send_email(settings: Settings, msg: MIMEText):
    with smtplib.SMTP_SSL(settings.smtp_host, 465) as server:
        server.login(settings.smtp_user, settings.smtp_pass)
        server.send_message(msg)


# ── POST /api/org/verify/dns ────────────────────────────────────────

@router.post("/verify/dns")
async def verify_dns(req: VerifyDnsRequest):
    domain = req.domain.lower().strip()
    org = await db.get_organization_by_domain(domain)
    if not org:
        raise HTTPException(404, "organization not found — register first")
    if org["verified"]:
        raise HTTPException(409, "already verified")
    if not org["dns_token"]:
        raise HTTPException(400, "no DNS verification pending for this domain")

    # DNS TXT lookup
    try:
        import dns.resolver
        answers = await asyncio.to_thread(
            lambda: dns.resolver.resolve(domain, "TXT")
        )
        txt_records = []
        for rdata in answers:
            for txt_string in rdata.strings:
                txt_records.append(txt_string.decode())
    except Exception:
        txt_records = []

    if org["dns_token"] not in txt_records:
        raise HTTPException(
            400,
            f"TXT record not found. Expected: {org['dns_token']}. Found: {txt_records}",
        )

    verified_org = await db.verify_organization(domain)
    api_key = _generate_api_key()
    await db.insert_org_api_key(
        org_id=verified_org["id"],
        api_key=api_key,
        email=org.get("admin_email"),
        role="admin",
    )

    return {
        "status": "verified",
        "domain": domain,
        "api_key": api_key,
        "role": "admin",
    }


# ── POST /api/org/verify/email ──────────────────────────────────────

@router.post("/verify/email")
async def verify_email(req: VerifyEmailRequest):
    email = req.email.lower().strip()
    entry = _email_codes.get(email)

    if not entry:
        raise HTTPException(404, "no verification pending for this email")

    if datetime.now() - entry.created_at > EXPIRY:
        del _email_codes[email]
        raise HTTPException(410, "verification code expired — request a new code")

    if entry.attempts >= MAX_ATTEMPTS:
        del _email_codes[email]
        raise HTTPException(429, "too many attempts — request a new code")

    if entry.code != req.code:
        entry.attempts += 1
        remaining = MAX_ATTEMPTS - entry.attempts
        if remaining <= 0:
            del _email_codes[email]
            raise HTTPException(429, "too many attempts — request a new code")
        raise HTTPException(
            400,
            f"invalid code — {remaining} attempt{'s' if remaining != 1 else ''} remaining",
        )

    domain = entry.domain
    del _email_codes[email]

    verified_org = await db.verify_organization(domain)
    if not verified_org:
        raise HTTPException(500, "failed to verify organization")

    api_key = _generate_api_key()
    await db.insert_org_api_key(
        org_id=verified_org["id"],
        api_key=api_key,
        email=email,
        role="admin",
    )

    return {
        "status": "verified",
        "domain": domain,
        "api_key": api_key,
        "role": "admin",
    }


# ── POST /api/org/resend ────────────────────────────────────────────

class ResendRequest(BaseModel):
    domain: str
    admin_email: str


@router.post("/resend")
async def resend_code(req: ResendRequest):
    """Resend a verification code. Generates a fresh code and resets attempts."""
    domain = req.domain.lower().strip()
    email = req.admin_email.lower().strip()

    email_domain = email.split("@", 1)[1].lower() if "@" in email else ""
    if email_domain != domain:
        raise HTTPException(400, f"email must be @{domain}")

    org = await db.get_organization_by_domain(domain)
    if not org:
        raise HTTPException(404, "organization not found — register first")
    if org["verified"]:
        raise HTTPException(409, "already verified")

    code = _generate_code()
    _clean_expired()
    _email_codes[email] = EmailCode(domain=domain, email=email, code=code)

    settings = Settings()
    resp = {"status": "pending", "method": "email", "domain": domain, "email": req.admin_email}

    if settings.smtp_host:
        html_body = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,sans-serif;background:#0a0a0f;color:#e5e5e5;margin:0;padding:40px;">
<div style="max-width:480px;margin:0 auto;background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:40px;text-align:center;">
<h1 style="color:#facc15;margin:0 0 8px;font-size:24px;">R3L Provenance</h1>
<p style="color:#9ca3af;margin:0 0 24px;">Verify your organization domain.</p>
<p style="color:#e5e5e5;margin:0 0 8px;">Your new verification code:</p>
<p style="font-size:36px;font-weight:700;color:#facc15;letter-spacing:8px;margin:0 0 24px;">{code}</p>
<p style="color:#6b7280;font-size:12px;margin:0;">This code expires in 30 minutes.</p>
</div>
</body></html>"""
        from_addr = settings.smtp_from or settings.smtp_user
        msg = MIMEText(html_body, "html")
        msg["Subject"] = "R3L — New verification code"
        msg["From"] = from_addr
        msg["To"] = req.admin_email

        try:
            await asyncio.to_thread(_send_email, settings, msg)
        except Exception as e:
            raise HTTPException(500, f"failed to send email: {e}")
    else:
        resp["dev_code"] = code

    return resp


# ── POST /api/org/keys ─────────────────────────────────────────────

@router.post("/keys")
async def create_key(req: CreateKeyRequest, admin: dict = Depends(require_org_admin)):
    org = admin["org"]
    api_key = _generate_api_key()
    key = await db.insert_org_api_key(
        org_id=org["id"],
        api_key=api_key,
        email=req.email,
        role=req.role,
    )
    return {
        "id": key["id"],
        "api_key": api_key,
        "email": req.email,
        "role": req.role,
    }


# ── GET /api/org/keys ──────────────────────────────────────────────

@router.get("/keys")
async def list_keys(admin: dict = Depends(require_org_admin)):
    org = admin["org"]
    keys = await db.list_org_api_keys(org["id"])
    # Mask API keys — show first 8 chars only
    for k in keys:
        k["api_key"] = k["api_key"][:8] + "..."
    return {"keys": keys}


# ── DELETE /api/org/keys/{key_id} ──────────────────────────────────

@router.delete("/keys/{key_id}")
async def revoke_key(key_id: int, admin: dict = Depends(require_org_admin)):
    org = admin["org"]
    # Prevent revoking the key currently in use
    if admin["key"]["id"] == key_id:
        raise HTTPException(400, "cannot revoke your own active key")
    ok = await db.revoke_org_api_key(key_id, org["id"])
    if not ok:
        raise HTTPException(404, "key not found or already revoked")
    return {"status": "revoked"}


# ── GET /api/org/info ──────────────────────────────────────────────

@router.get("/info")
async def org_info(caller: dict = Depends(require_api_key)):
    if caller.get("type") != "org":
        raise HTTPException(403, "not an org API key")

    org = await db.get_organization_by_id(caller["org_id"])
    if not org:
        raise HTTPException(404, "organization not found")

    dids = get_all_dids_for_org(org)
    return {
        "org": org,
        "dids": dids,
    }
