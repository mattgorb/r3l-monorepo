import asyncio
import hashlib
import secrets

import httpx
from fastapi import APIRouter, Depends, File, Form, Header, HTTPException, UploadFile
from pydantic import BaseModel
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError
import base58

from auth import require_api_key
from config import Settings
from similarity import compute_tlsh, compute_clip_embedding
from routes.verify import run_verifier, validate_upload
from routes.attest import _submit_attestation, MAX_FILE_SIZE
from storage import get_storage
import db

router = APIRouter()


# ── Register ──────────────────────────────────────────────────────


class RegisterRequest(BaseModel):
    name: str | None = None
    email: str | None = None
    wallet_pubkey: str | None = None
    wallet_message: str | None = None
    wallet_signature: str | None = None
    org_domain: str | None = None


@router.post("/register")
async def register(req: RegisterRequest):
    """Single-step developer registration. Provide any combo of email, wallet, org_domain."""
    has_email = bool(req.email)
    has_wallet = bool(req.wallet_pubkey)
    has_org = bool(req.org_domain)

    if not has_email and not has_wallet and not has_org:
        raise HTTPException(400, "provide at least one identity: email, wallet_pubkey, or org_domain")

    # ── Wallet verification (inline Ed25519) ──
    if has_wallet:
        if not req.wallet_message or not req.wallet_signature:
            raise HTTPException(400, "wallet_message and wallet_signature required with wallet_pubkey")
        if not req.wallet_message.startswith("R3L-register:"):
            raise HTTPException(400, "wallet_message must start with 'R3L-register:'")
        try:
            pk_bytes = base58.b58decode(req.wallet_pubkey)
            sig_bytes = base58.b58decode(req.wallet_signature)
        except Exception:
            raise HTTPException(400, "invalid base58 encoding")
        if len(pk_bytes) != 32:
            raise HTTPException(400, "invalid wallet pubkey length")
        try:
            verify_key = VerifyKey(pk_bytes)
            verify_key.verify(req.wallet_message.encode(), sig_bytes)
        except BadSignatureError:
            raise HTTPException(400, "invalid wallet signature")

    # ── Email normalization ──
    email = req.email.strip().lower() if req.email else None

    # ── Idempotency: check existing accounts ──
    if has_wallet:
        existing = await db.get_customer_by_wallet(req.wallet_pubkey)
        if existing:
            result = {"api_key": existing["api_key"], "name": existing["name"], "existing": True}
            if existing.get("email"):
                result["email"] = existing["email"]
            if existing.get("wallet_pubkey"):
                result["wallet_pubkey"] = existing["wallet_pubkey"]
            return result

    if email:
        existing = await db.get_customer_by_email(email)
        if existing:
            result = {"api_key": existing["api_key"], "name": existing["name"], "existing": True}
            if existing.get("email"):
                result["email"] = existing["email"]
            if existing.get("wallet_pubkey"):
                result["wallet_pubkey"] = existing["wallet_pubkey"]
            return result

    # ── Org lookup ──
    org_id = None
    org_domain = None
    if has_org:
        org = await db.get_organization_by_domain(req.org_domain)
        if not org:
            raise HTTPException(404, f"organization '{req.org_domain}' not found")
        if not org["verified"]:
            raise HTTPException(400, f"organization '{req.org_domain}' is not verified")
        org_id = org["id"]
        org_domain = org["domain"]

    # ── Create account ──
    api_key = "r3l_" + secrets.token_hex(24)
    name = req.name or email or (req.wallet_pubkey[:8] if req.wallet_pubkey else "developer")

    customer = await db.insert_customer(
        name=name,
        api_key=api_key,
        email=email,
        wallet_pubkey=req.wallet_pubkey if has_wallet else None,
        auth_method="wallet" if has_wallet else ("email" if email else None),
    )

    # Link org if provided
    if org_id:
        from sqlalchemy import select, text as sql_text
        async with db.get_session() as session:
            from models import Customer
            stmt = select(Customer).where(Customer.id == customer["id"])
            row = (await session.execute(stmt)).scalar_one_or_none()
            if row:
                row.org_id = org_id
                await session.commit()

    result = {
        "api_key": customer["api_key"],
        "name": customer["name"],
        "existing": False,
    }
    if email:
        result["email"] = email
    if has_wallet:
        result["wallet_pubkey"] = req.wallet_pubkey
    if org_domain:
        result["org_domain"] = org_domain
    return result


# ── Verify identity ───────────────────────────────────────────────


class VerifyIdentityRequest(BaseModel):
    email: str | None = None
    code: str | None = None
    wallet_pubkey: str | None = None
    wallet_message: str | None = None
    wallet_signature: str | None = None
    org_domain: str | None = None


async def _process_verify_identity(req: VerifyIdentityRequest, caller: dict) -> dict:
    """Shared identity verification logic — processes whichever fields are present."""
    has_email = bool(req.email)
    has_code = bool(req.code)
    has_wallet = bool(req.wallet_pubkey)
    has_org = bool(req.org_domain)

    result: dict = {}

    # ── Email: send code or confirm ──
    if has_email:
        email = req.email.strip().lower()
        if "@" not in email:
            raise HTTPException(400, "invalid email")
        domain = email.split("@", 1)[1]
        if "." not in domain or len(domain) < 3:
            raise HTTPException(400, "invalid email domain")

        if has_code:
            from routes.auth_routes import _email_codes, EXPIRY, MAX_ATTEMPTS
            from datetime import datetime

            entry = _email_codes.get(email)
            if not entry:
                raise HTTPException(404, "no verification pending for this email")
            if datetime.now() - entry.created_at > EXPIRY:
                del _email_codes[email]
                raise HTTPException(410, "code expired")
            if entry.attempts >= MAX_ATTEMPTS:
                del _email_codes[email]
                raise HTTPException(429, "too many attempts")
            if entry.code != req.code:
                entry.attempts += 1
                remaining = MAX_ATTEMPTS - entry.attempts
                if remaining <= 0:
                    del _email_codes[email]
                    raise HTTPException(429, "too many attempts")
                raise HTTPException(400, f"invalid code \u2014 {remaining} attempt{'s' if remaining != 1 else ''} remaining")

            del _email_codes[email]

            existing = await db.get_customer_by_email(email)
            if existing:
                if existing["id"] != caller["id"]:
                    await db.merge_customers(keep_id=caller["id"], remove_id=existing["id"])
                    result["email"] = email
                    result["email_merged"] = True
                else:
                    result["email"] = email
            else:
                await db.link_email_to_customer(caller["id"], email)
                result["email"] = email
            result["email_status"] = "verified"
        else:
            from routes.auth_routes import _email_codes, _clean_expired_emails, _generate_code, _send_email, EmailCode

            _clean_expired_emails()
            code = _generate_code()
            _email_codes[email] = EmailCode(email=email, code=code)

            settings = Settings()
            result["email"] = email
            result["email_status"] = "pending"

            if settings.smtp_host:
                from email.mime.text import MIMEText
                html_body = f"""<!DOCTYPE html>
<html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,sans-serif;background:#0a0a0f;color:#e5e5e5;margin:0;padding:40px;">
<div style="max-width:480px;margin:0 auto;background:#1a1a2e;border:1px solid #2d2d44;border-radius:12px;padding:40px;text-align:center;">
<h1 style="color:#facc15;margin:0 0 8px;font-size:24px;">R3L Provenance</h1>
<p style="color:#9ca3af;margin:0 0 24px;">Verify your email address.</p>
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
                result["dev_code"] = code

    # ── Wallet: inline Ed25519 verification ──
    if has_wallet:
        if not req.wallet_message or not req.wallet_signature:
            raise HTTPException(400, "wallet_message and wallet_signature required with wallet_pubkey")
        if not req.wallet_message.startswith("R3L-verify:"):
            raise HTTPException(400, "wallet_message must start with 'R3L-verify:'")

        try:
            pk_bytes = base58.b58decode(req.wallet_pubkey)
            sig_bytes = base58.b58decode(req.wallet_signature)
        except Exception:
            raise HTTPException(400, "invalid base58 encoding")
        if len(pk_bytes) != 32:
            raise HTTPException(400, "invalid wallet pubkey length")

        try:
            verify_key = VerifyKey(pk_bytes)
            verify_key.verify(req.wallet_message.encode(), sig_bytes)
        except BadSignatureError:
            raise HTTPException(400, "invalid wallet signature")

        existing = await db.get_customer_by_wallet(req.wallet_pubkey)
        if existing:
            if existing["id"] != caller["id"]:
                await db.merge_customers(keep_id=caller["id"], remove_id=existing["id"])
                result["wallet_merged"] = True
        else:
            await db.link_wallet_to_customer(caller["id"], req.wallet_pubkey)

        result["wallet_pubkey"] = req.wallet_pubkey
        result["wallet_status"] = "verified"

    # ── Org: verify via email domain match, then link ──
    if has_org:
        # Check caller has a verified email matching this org domain
        caller_email = caller.get("email") or (req.email if result.get("email_status") == "verified" else None)
        if not caller_email:
            raise HTTPException(400, "org verification requires a verified email on your account")
        email_domain = caller_email.split("@", 1)[1] if "@" in caller_email else ""
        if email_domain != req.org_domain:
            raise HTTPException(400, f"your email domain '{email_domain}' does not match org '{req.org_domain}'")

        org = await db.get_organization_by_domain(req.org_domain)
        if not org:
            org = await db.insert_organization(domain=req.org_domain, name=req.org_domain)

        # Mark verified (email domain match is proof of org membership)
        if not org.get("verified"):
            from sqlalchemy import select as sa_select
            async with db.get_session() as session:
                from models import Organization
                row = (await session.execute(sa_select(Organization).where(Organization.id == org["id"]))).scalar_one()
                row.verified = True
                row.verification_method = "email"
                row.admin_email = caller_email
                await session.commit()
                org = row.to_dict()

        from sqlalchemy import select
        async with db.get_session() as session:
            from models import Customer
            stmt = select(Customer).where(Customer.id == caller["id"])
            row = (await session.execute(stmt)).scalar_one_or_none()
            if not row:
                raise HTTPException(404, "account not found")
            row.org_id = org["id"]
            await session.commit()

        result["org_domain"] = org["domain"]
        result["org_status"] = "verified"

    return result


@router.post("/verify-identity")
async def verify_identity(req: VerifyIdentityRequest, caller: dict = Depends(require_api_key)):
    """Verify and link identities to your account. Any combination in one call.

    - email (no code): sends a 6-digit verification code
    - email + code: confirms the code and links the email
    - wallet_pubkey + wallet_message + wallet_signature: verifies Ed25519 sig inline
    - org_domain: links your account to a verified organization
    """
    types = sum([bool(req.email), bool(req.wallet_pubkey), bool(req.org_domain)])
    if types == 0:
        raise HTTPException(400, "provide at least one: email, wallet_pubkey, or org_domain")
    return await _process_verify_identity(req, caller)


# ── Content verification + attestation ────────────────────────────


async def _attest_file(file_bytes: bytes, filename: str, content_type: str | None,
                       settings: Settings, caller: dict | None,
                       should_store: bool, is_private: bool,
                       wallet_pubkey: str | None, wallet_message: str | None,
                       wallet_signature: str | None) -> dict:
    validate_upload(file_bytes, content_type)
    file_tlsh = compute_tlsh(file_bytes)
    file_clip = compute_clip_embedding(file_bytes, content_type)
    verify_output = await run_verifier(file_bytes, filename, settings)
    content_hash_hex = verify_output.get("content_hash")
    if not content_hash_hex:
        raise HTTPException(500, "no content hash from verifier")
    if should_store:
        storage = get_storage()
        await storage.save(content_hash_hex, file_bytes, content_type or "application/octet-stream")
    result = await _submit_attestation(
        settings=settings, content_hash_hex=content_hash_hex,
        verify_output=verify_output, tlsh_hash=file_tlsh,
        clip_embedding=file_clip, content_type="file",
        mime_type=content_type, content_size=len(file_bytes),
        stored=should_store, wallet_pubkey=wallet_pubkey,
        wallet_message=wallet_message, wallet_signature=wallet_signature,
        privacy_mode=caller.get("privacy_mode", False) if caller else False,
        private_mode=is_private,
    )
    result["type"] = "file"
    return result


async def _attest_url(url: str, settings: Settings, caller: dict | None,
                      should_store: bool, is_private: bool) -> dict:
    fetch_headers = {"User-Agent": "R3L-Attester/1.0"}
    try:
        async with httpx.AsyncClient(follow_redirects=True, timeout=30) as client:
            resp = await client.get(url, headers=fetch_headers)
            resp.raise_for_status()
    except httpx.HTTPStatusError as e:
        raise HTTPException(502, f"URL returned {e.response.status_code}")
    except Exception as e:
        raise HTTPException(502, f"failed to fetch URL: {e}")

    page_bytes = resp.content
    if len(page_bytes) > MAX_FILE_SIZE:
        raise HTTPException(413, "page too large")

    ct = resp.headers.get("content-type", "text/html").split(";")[0].strip()
    content_hash_hex = hashlib.sha256(page_bytes).hexdigest()
    file_tlsh = compute_tlsh(page_bytes)
    file_clip = compute_clip_embedding(page_bytes, ct)

    if should_store:
        storage = get_storage()
        await storage.save(content_hash_hex, page_bytes, ct)

    verify_output = {"content_hash": content_hash_hex, "has_c2pa": False}
    result = await _submit_attestation(
        settings=settings, content_hash_hex=content_hash_hex,
        verify_output=verify_output, tlsh_hash=file_tlsh,
        clip_embedding=file_clip, content_type="url",
        source_url=url, mime_type=ct, content_size=len(page_bytes),
        stored=should_store,
        privacy_mode=caller.get("privacy_mode", False) if caller else False,
        private_mode=is_private,
    )
    result["type"] = "url"
    return result


async def _attest_text(text: str, settings: Settings, caller: dict | None,
                       should_store: bool, is_private: bool) -> dict:
    text_bytes = text.encode("utf-8")
    if len(text_bytes) > MAX_FILE_SIZE:
        raise HTTPException(413, "text too large")

    content_hash_hex = hashlib.sha256(text_bytes).hexdigest()
    file_tlsh = compute_tlsh(text_bytes)
    file_clip = compute_clip_embedding(text_bytes, "text/plain")

    if should_store:
        storage = get_storage()
        await storage.save(content_hash_hex, text_bytes, "text/plain")

    verify_output = {"content_hash": content_hash_hex, "has_c2pa": False}
    result = await _submit_attestation(
        settings=settings, content_hash_hex=content_hash_hex,
        verify_output=verify_output, tlsh_hash=file_tlsh,
        clip_embedding=file_clip, content_type="text",
        mime_type="text/plain", content_size=len(text_bytes),
        stored=should_store,
        privacy_mode=caller.get("privacy_mode", False) if caller else False,
        private_mode=is_private,
    )
    result["type"] = "text"
    return result


@router.post("/attest-content")
async def attest_content(
    file: UploadFile | None = File(None),
    url: str | None = Form(None),
    text: str | None = Form(None),
    title: str | None = Form(None),
    store_content: str = Form("true"),
    private_mode: str = Form("false"),
    wallet_pubkey: str | None = Form(None),
    wallet_message: str | None = Form(None),
    wallet_signature: str | None = Form(None),
    x_api_key: str | None = Header(None),
):
    """Verify and attest one content item. Provide exactly one of: file, url, or text.

    Files are verified for C2PA metadata. All content is hashed, stored, and attested on Solana.
    """
    has_file = file is not None and bool(file.filename)
    has_url = bool(url)
    has_text = bool(text)
    types = sum([has_file, has_url, has_text])

    if types == 0:
        raise HTTPException(400, "provide one of: file, url, or text")
    if types > 1:
        raise HTTPException(400, "only one content type per request — use /attest-content/batch for multiple")

    settings = Settings()
    caller = await db.get_customer_by_api_key(x_api_key) if x_api_key else None
    should_store = store_content.lower() not in ("false", "0", "no")
    is_private = private_mode.lower() not in ("false", "0", "no")

    if has_file:
        file_bytes = await file.read()
        ct = file.content_type
        if not ct or ct == "application/octet-stream":
            import mimetypes
            ct = mimetypes.guess_type(file.filename or "")[0] or "application/octet-stream"
        return await _attest_file(file_bytes, file.filename or "upload", ct,
                                  settings, caller, should_store, is_private,
                                  wallet_pubkey, wallet_message, wallet_signature)
    elif has_url:
        return await _attest_url(url, settings, caller, should_store, is_private)
    else:
        return await _attest_text(text, settings, caller, should_store, is_private)


@router.post("/attest-content/batch")
async def attest_content_batch(
    file: list[UploadFile] = File(default=[]),
    url: str | None = Form(None),
    text: str | None = Form(None),
    title: str | None = Form(None),
    store_content: str = Form("true"),
    private_mode: str = Form("false"),
    wallet_pubkey: str | None = Form(None),
    wallet_message: str | None = Form(None),
    wallet_signature: str | None = Form(None),
    x_api_key: str | None = Header(None),
):
    """Verify and attest multiple content items in one call. Supports multiple files, plus optional url and text."""
    real_files = [f for f in file if f.filename]
    has_url = bool(url)
    has_text = bool(text)

    if not real_files and not has_url and not has_text:
        raise HTTPException(400, "provide at least one of: files, url, or text")

    settings = Settings()
    caller = await db.get_customer_by_api_key(x_api_key) if x_api_key else None
    should_store = store_content.lower() not in ("false", "0", "no")
    is_private = private_mode.lower() not in ("false", "0", "no")

    results = []
    for f in real_files:
        try:
            file_bytes = await f.read()
            ct = f.content_type
            # curl sends application/octet-stream for unknown types — infer from extension
            if not ct or ct == "application/octet-stream":
                import mimetypes
                ct = mimetypes.guess_type(f.filename or "")[0] or "application/octet-stream"
            results.append(await _attest_file(file_bytes, f.filename or "upload", ct,
                                              settings, caller, should_store, is_private,
                                              wallet_pubkey, wallet_message, wallet_signature))
        except HTTPException as e:
            results.append({"type": "file", "filename": f.filename, "error": e.detail, "status": e.status_code})
        except Exception as e:
            results.append({"type": "file", "filename": f.filename, "error": str(e), "status": 500})
    if has_url:
        try:
            results.append(await _attest_url(url, settings, caller, should_store, is_private))
        except HTTPException as e:
            results.append({"type": "url", "url": url, "error": e.detail, "status": e.status_code})
        except Exception as e:
            results.append({"type": "url", "url": url, "error": str(e), "status": 500})
    if has_text:
        try:
            results.append(await _attest_text(text, settings, caller, should_store, is_private))
        except HTTPException as e:
            results.append({"type": "text", "error": e.detail, "status": e.status_code})
        except Exception as e:
            results.append({"type": "text", "error": str(e), "status": 500})

    return {"results": results}


# ── Account ───────────────────────────────────────────────────────


@router.get("/me")
async def get_me(caller: dict = Depends(require_api_key)):
    """Get account info. Alias for /api/auth/me."""
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
            "privacy_mode": caller.get("privacy_mode", False),
        }
