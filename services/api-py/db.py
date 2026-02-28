import time

from sqlalchemy import select, text
from sqlalchemy.ext.asyncio import AsyncSession, async_sessionmaker, create_async_engine

from models import Attestation, Customer, Organization, OrgApiKey, Base

_engine = None
_session_factory: async_sessionmaker[AsyncSession] | None = None


async def init_db(database_url: str):
    global _engine, _session_factory
    # asyncpg requires postgresql+asyncpg:// scheme
    url = database_url.replace("postgresql://", "postgresql+asyncpg://", 1)
    _engine = create_async_engine(url, pool_size=5)
    _session_factory = async_sessionmaker(_engine, expire_on_commit=False)
    async with _engine.begin() as conn:
        await conn.execute(text("CREATE EXTENSION IF NOT EXISTS vector"))
        await conn.run_sync(Base.metadata.create_all)
        # Migrate existing tables — add columns that may not exist yet
        migrations = [
            "ALTER TABLE customers ADD COLUMN IF NOT EXISTS org_id INTEGER",
            "ALTER TABLE customers ADD COLUMN IF NOT EXISTS email VARCHAR",
            "ALTER TABLE customers ADD COLUMN IF NOT EXISTS auth_method VARCHAR",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS org_id INTEGER",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS org_domain VARCHAR",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS content_type VARCHAR DEFAULT 'file'",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS source_url VARCHAR",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS mime_type VARCHAR",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS content_size INTEGER",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS stored BOOLEAN DEFAULT false",
            "ALTER TABLE customers ADD COLUMN IF NOT EXISTS privacy_mode BOOLEAN DEFAULT false",
            "ALTER TABLE attestations ADD COLUMN IF NOT EXISTS private BOOLEAN DEFAULT false",
            "CREATE UNIQUE INDEX IF NOT EXISTS ix_customers_email ON customers(email) WHERE email IS NOT NULL",
        ]
        for sql in migrations:
            await conn.execute(text(sql))


async def close_db():
    global _engine, _session_factory
    if _engine:
        await _engine.dispose()
        _engine = None
        _session_factory = None


def get_session() -> AsyncSession:
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    return _session_factory()


async def insert_attestation(
    *,
    content_hash: str,
    proof_type: str = "trusted_verifier",
    tx_signature: str | None = None,
    pda: str | None = None,
    has_c2pa: bool | None = None,
    trust_list_match: str | None = None,
    validation_state: str | None = None,
    digital_source_type: str | None = None,
    issuer: str | None = None,
    common_name: str | None = None,
    software_agent: str | None = None,
    signing_time: str | None = None,
    cert_fingerprint: str | None = None,
    email_domain: str | None = None,
    wallet_pubkey: str | None = None,
    submitted_by: str | None = None,
    verifier_version: str | None = None,
    trust_bundle_hash: str | None = None,
    tlsh_hash: str | None = None,
    clip_embedding: list[float] | None = None,
    org_id: int | None = None,
    org_domain: str | None = None,
    content_type: str = "file",
    source_url: str | None = None,
    mime_type: str | None = None,
    content_size: int | None = None,
    stored: bool = False,
    private: bool = False,
    created_at: int | None = None,
):
    if _session_factory is None:
        return
    if created_at is None:
        created_at = int(time.time())
    async with get_session() as session:
        stmt = select(Attestation).where(
            Attestation.content_hash == content_hash,
        )
        existing = (await session.execute(stmt)).scalar_one_or_none()
        if existing:
            return
        row = Attestation(
            content_hash=content_hash,
            proof_type=proof_type,
            tx_signature=tx_signature,
            pda=pda,
            has_c2pa=has_c2pa,
            trust_list_match=trust_list_match,
            validation_state=validation_state,
            digital_source_type=digital_source_type,
            issuer=issuer,
            common_name=common_name,
            software_agent=software_agent,
            signing_time=signing_time,
            cert_fingerprint=cert_fingerprint,
            email_domain=email_domain,
            wallet_pubkey=wallet_pubkey,
            submitted_by=submitted_by,
            verifier_version=verifier_version,
            trust_bundle_hash=trust_bundle_hash,
            tlsh_hash=tlsh_hash,
            clip_embedding=clip_embedding,
            org_id=org_id,
            org_domain=org_domain,
            content_type=content_type,
            source_url=source_url,
            mime_type=mime_type,
            content_size=content_size,
            stored=stored,
            private=private,
            created_at=created_at,
        )
        session.add(row)
        await session.commit()


async def get_attestation(content_hash: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Attestation).where(
            Attestation.content_hash == content_hash,
        )
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            return None
        return row.to_dict()


async def list_attestations(include_private: bool = False) -> list[dict]:
    if _session_factory is None:
        return []
    async with get_session() as session:
        stmt = select(Attestation).order_by(Attestation.created_at.desc())
        if not include_private:
            stmt = stmt.where(Attestation.private == False)
        rows = (await session.execute(stmt)).scalars().all()
        return [r.to_dict() for r in rows]


# ── Customer functions ──────────────────────────────────────────────

async def insert_customer(
    *, name: str, api_key: str, wallet_pubkey: str | None = None,
    email: str | None = None, auth_method: str | None = None,
) -> dict:
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        row = Customer(
            name=name,
            email=email,
            auth_method=auth_method,
            wallet_pubkey=wallet_pubkey,
            api_key=api_key,
            created_at=int(time.time()),
        )
        session.add(row)
        await session.commit()
        return row.to_dict()


async def get_customer_by_api_key(api_key: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Customer).where(Customer.api_key == api_key)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            return None
        return row.to_dict()


async def get_customer_by_wallet(wallet_pubkey: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Customer).where(Customer.wallet_pubkey == wallet_pubkey)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            return None
        return row.to_dict()


async def get_customer_by_email(email: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Customer).where(Customer.email == email)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            return None
        return row.to_dict()


async def link_email_to_customer(customer_id: int, email: str):
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        stmt = select(Customer).where(Customer.id == customer_id)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            raise RuntimeError("customer not found")
        row.email = email
        await session.commit()


async def link_wallet_to_customer(customer_id: int, wallet_pubkey: str):
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        stmt = select(Customer).where(Customer.id == customer_id)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            raise RuntimeError("customer not found")
        row.wallet_pubkey = wallet_pubkey
        await session.commit()


async def update_customer_privacy_mode(customer_id: int, privacy_mode: bool) -> dict:
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        stmt = select(Customer).where(Customer.id == customer_id)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if row is None:
            raise RuntimeError("customer not found")
        row.privacy_mode = privacy_mode
        await session.commit()
        await session.refresh(row)
        return row.to_dict()


async def merge_customers(keep_id: int, remove_id: int) -> dict:
    """Merge remove_id into keep_id: copy missing identities, reassign attestations, delete remove."""
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        keep = (await session.execute(select(Customer).where(Customer.id == keep_id))).scalar_one_or_none()
        remove = (await session.execute(select(Customer).where(Customer.id == remove_id))).scalar_one_or_none()
        if not keep or not remove:
            raise RuntimeError("customer not found")

        # Copy missing identities from remove → keep
        # Clear from remove first to avoid unique constraint violations
        if not keep.email and remove.email:
            keep.email = remove.email
            remove.email = None
        if not keep.wallet_pubkey and remove.wallet_pubkey:
            keep.wallet_pubkey = remove.wallet_pubkey
            remove.wallet_pubkey = None

        # Flush the clears so the unique columns are free before delete
        await session.flush()

        # Reassign attestations submitted by the removed account's API key
        await session.execute(
            text("UPDATE attestations SET submitted_by = :new_key WHERE submitted_by = :old_key"),
            {"new_key": keep.api_key, "old_key": remove.api_key},
        )

        # Delete the removed account
        await session.delete(remove)
        await session.commit()
        await session.refresh(keep)
        return keep.to_dict()


# ── Similarity search functions ───────────────────────────────────

async def get_all_with_tlsh() -> list[dict]:
    """Return all attestations that have a TLSH hash."""
    if _session_factory is None:
        return []
    async with get_session() as session:
        stmt = select(Attestation).where(Attestation.tlsh_hash.isnot(None))
        rows = (await session.execute(stmt)).scalars().all()
        return [r.to_dict() for r in rows]


async def search_similar_clip(
    embedding: list[float], limit: int = 20,
) -> list[dict]:
    """Find attestations with similar CLIP embeddings using pgvector cosine distance."""
    if _session_factory is None:
        return []
    async with get_session() as session:
        # cosine distance: <=> returns distance (0=identical, 2=opposite)
        # similarity = 1 - distance
        stmt = (
            select(
                Attestation,
                (1 - Attestation.clip_embedding.cosine_distance(embedding)).label("clip_similarity"),
            )
            .where(Attestation.clip_embedding.isnot(None))
            .order_by(Attestation.clip_embedding.cosine_distance(embedding))
            .limit(limit)
        )
        rows = (await session.execute(stmt)).all()
        results = []
        for row, similarity in rows:
            d = row.to_dict()
            d["clip_similarity"] = round(float(similarity), 4)
            results.append(d)
        return results


# ── Organization functions ─────────────────────────────────────────

async def insert_organization(
    *, domain: str, name: str | None = None,
    verification_method: str | None = None,
    dns_token: str | None = None,
    admin_email: str | None = None,
) -> dict:
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        row = Organization(
            domain=domain,
            name=name,
            verified=False,
            verification_method=verification_method,
            dns_token=dns_token,
            admin_email=admin_email,
            did_web=f"did:web:{domain}",
            created_at=int(time.time()),
        )
        session.add(row)
        await session.commit()
        await session.refresh(row)
        return row.to_dict()


async def get_organization_by_domain(domain: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Organization).where(Organization.domain == domain)
        row = (await session.execute(stmt)).scalar_one_or_none()
        return row.to_dict() if row else None


async def get_organization_by_id(org_id: int) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Organization).where(Organization.id == org_id)
        row = (await session.execute(stmt)).scalar_one_or_none()
        return row.to_dict() if row else None


async def verify_organization(domain: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(Organization).where(Organization.domain == domain)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if not row:
            return None
        row.verified = True
        await session.commit()
        await session.refresh(row)
        return row.to_dict()


# ── Org API Key functions ──────────────────────────────────────────

async def insert_org_api_key(
    *, org_id: int, api_key: str, email: str | None = None, role: str = "attester",
) -> dict:
    if _session_factory is None:
        raise RuntimeError("DB not initialized")
    async with get_session() as session:
        row = OrgApiKey(
            org_id=org_id,
            api_key=api_key,
            email=email,
            role=role,
            revoked=False,
            created_at=int(time.time()),
        )
        session.add(row)
        await session.commit()
        await session.refresh(row)
        return row.to_dict()


async def get_org_api_key(api_key: str) -> dict | None:
    if _session_factory is None:
        return None
    async with get_session() as session:
        stmt = select(OrgApiKey).where(
            OrgApiKey.api_key == api_key,
            OrgApiKey.revoked == False,
        )
        row = (await session.execute(stmt)).scalar_one_or_none()
        return row.to_dict() if row else None


async def list_org_api_keys(org_id: int) -> list[dict]:
    if _session_factory is None:
        return []
    async with get_session() as session:
        stmt = select(OrgApiKey).where(OrgApiKey.org_id == org_id).order_by(OrgApiKey.created_at.desc())
        rows = (await session.execute(stmt)).scalars().all()
        return [r.to_dict() for r in rows]


async def revoke_org_api_key(key_id: int, org_id: int) -> bool:
    if _session_factory is None:
        return False
    async with get_session() as session:
        stmt = select(OrgApiKey).where(OrgApiKey.id == key_id, OrgApiKey.org_id == org_id)
        row = (await session.execute(stmt)).scalar_one_or_none()
        if not row:
            return False
        row.revoked = True
        await session.commit()
        return True
