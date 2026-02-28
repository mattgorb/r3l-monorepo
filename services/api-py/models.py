from sqlalchemy import BigInteger, Boolean, Column, Integer, String, UniqueConstraint
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column
from pgvector.sqlalchemy import Vector


class Base(DeclarativeBase):
    pass


class Attestation(Base):
    __tablename__ = "attestations"
    __table_args__ = (UniqueConstraint("content_hash"),)

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    content_hash: Mapped[str] = mapped_column(String, nullable=False, unique=True)
    proof_type: Mapped[str] = mapped_column(String, nullable=False, default="trusted_verifier")
    tx_signature: Mapped[str | None] = mapped_column(String)
    pda: Mapped[str | None] = mapped_column(String)
    has_c2pa: Mapped[bool | None] = mapped_column(Boolean)
    trust_list_match: Mapped[str | None] = mapped_column(String)
    validation_state: Mapped[str | None] = mapped_column(String)
    digital_source_type: Mapped[str | None] = mapped_column(String)
    issuer: Mapped[str | None] = mapped_column(String)
    common_name: Mapped[str | None] = mapped_column(String)
    software_agent: Mapped[str | None] = mapped_column(String)
    signing_time: Mapped[str | None] = mapped_column(String)
    cert_fingerprint: Mapped[str | None] = mapped_column(String)
    email_domain: Mapped[str | None] = mapped_column(String)
    wallet_pubkey: Mapped[str | None] = mapped_column(String)
    submitted_by: Mapped[str | None] = mapped_column(String)
    org_id: Mapped[int | None] = mapped_column(Integer)
    org_domain: Mapped[str | None] = mapped_column(String)
    verifier_version: Mapped[str | None] = mapped_column(String)
    trust_bundle_hash: Mapped[str | None] = mapped_column(String)
    tlsh_hash: Mapped[str | None] = mapped_column(String)
    clip_embedding = Column(Vector(512), nullable=True)
    content_type: Mapped[str] = mapped_column(String, nullable=False, default="file")
    source_url: Mapped[str | None] = mapped_column(String)
    mime_type: Mapped[str | None] = mapped_column(String)
    content_size: Mapped[int | None] = mapped_column(Integer)
    stored: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    private: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    created_at: Mapped[int] = mapped_column(BigInteger, nullable=False)

    def to_dict(self) -> dict:
        d = {c.name: getattr(self, c.name) for c in self.__table__.columns}
        # Convert numpy array to list for JSON serialization
        if d.get("clip_embedding") is not None:
            try:
                d["clip_embedding"] = list(d["clip_embedding"])
            except TypeError:
                pass
        return d


class Customer(Base):
    __tablename__ = "customers"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    name: Mapped[str] = mapped_column(String, nullable=False)
    email: Mapped[str | None] = mapped_column(String, unique=True)
    auth_method: Mapped[str | None] = mapped_column(String)  # "email" | "wallet"
    wallet_pubkey: Mapped[str | None] = mapped_column(String, unique=True)
    api_key: Mapped[str] = mapped_column(String, unique=True, nullable=False)
    org_id: Mapped[int | None] = mapped_column(Integer)
    privacy_mode: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
    created_at: Mapped[int] = mapped_column(BigInteger, nullable=False)

    def to_dict(self) -> dict:
        return {c.name: getattr(self, c.name) for c in self.__table__.columns}


class Organization(Base):
    __tablename__ = "organizations"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    domain: Mapped[str] = mapped_column(String, unique=True, nullable=False)
    name: Mapped[str | None] = mapped_column(String)
    verified: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    verification_method: Mapped[str | None] = mapped_column(String)  # "email" | "dns"
    dns_token: Mapped[str | None] = mapped_column(String)
    admin_email: Mapped[str | None] = mapped_column(String)
    did_web: Mapped[str | None] = mapped_column(String)
    created_at: Mapped[int] = mapped_column(BigInteger, nullable=False)

    def to_dict(self) -> dict:
        return {c.name: getattr(self, c.name) for c in self.__table__.columns}


class OrgApiKey(Base):
    __tablename__ = "org_api_keys"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    org_id: Mapped[int] = mapped_column(Integer, nullable=False, index=True)
    email: Mapped[str | None] = mapped_column(String)
    role: Mapped[str] = mapped_column(String, nullable=False, default="attester")
    api_key: Mapped[str] = mapped_column(String, unique=True, nullable=False)
    created_at: Mapped[int] = mapped_column(BigInteger, nullable=False)
    revoked: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)

    def to_dict(self) -> dict:
        return {c.name: getattr(self, c.name) for c in self.__table__.columns}
