"""Content storage abstraction — local filesystem or S3.

Call init_storage(settings) once at startup.
Then use get_storage() to access the singleton.
"""

import asyncio
import json
import logging
import os

log = logging.getLogger(__name__)


class ContentStore:
    """Abstract content store interface."""

    async def save(self, key: str, data: bytes, content_type: str) -> None:
        raise NotImplementedError

    async def get(self, key: str) -> tuple[bytes, str] | None:
        raise NotImplementedError

    async def exists(self, key: str) -> bool:
        raise NotImplementedError

    async def delete_all(self) -> None:
        raise NotImplementedError


class LocalStore(ContentStore):
    """Store content on the local filesystem with two-level hash prefix dirs."""

    def __init__(self, base_dir: str):
        self.base_dir = os.path.abspath(base_dir)
        os.makedirs(self.base_dir, exist_ok=True)

    def _path(self, key: str) -> str:
        return os.path.join(self.base_dir, key[:2], key[2:4], key)

    def _meta_path(self, key: str) -> str:
        return self._path(key) + ".meta"

    async def save(self, key: str, data: bytes, content_type: str) -> None:
        path = self._path(key)
        if os.path.exists(path):
            return  # idempotent — content-addressed
        os.makedirs(os.path.dirname(path), exist_ok=True)
        # Write data
        def _write():
            with open(path, "wb") as f:
                f.write(data)
            with open(self._meta_path(key), "w") as f:
                json.dump({"content_type": content_type}, f)
        await asyncio.to_thread(_write)

    async def get(self, key: str) -> tuple[bytes, str] | None:
        path = self._path(key)
        meta_path = self._meta_path(key)
        if not os.path.exists(path):
            return None
        def _read():
            with open(path, "rb") as f:
                data = f.read()
            ct = "application/octet-stream"
            if os.path.exists(meta_path):
                with open(meta_path) as f:
                    ct = json.load(f).get("content_type", ct)
            return data, ct
        return await asyncio.to_thread(_read)

    async def exists(self, key: str) -> bool:
        return os.path.exists(self._path(key))

    async def delete_all(self) -> None:
        import shutil
        def _clear():
            if os.path.exists(self.base_dir):
                shutil.rmtree(self.base_dir)
                os.makedirs(self.base_dir, exist_ok=True)
        await asyncio.to_thread(_clear)


class S3Store(ContentStore):
    """Store content in an S3 bucket."""

    def __init__(self, bucket: str, prefix: str = "content/"):
        import boto3
        self.bucket = bucket
        self.prefix = prefix
        self._client = boto3.client("s3")

    def _key(self, key: str) -> str:
        return f"{self.prefix}{key}"

    async def save(self, key: str, data: bytes, content_type: str) -> None:
        s3_key = self._key(key)
        # Check if already exists (idempotent)
        if await self.exists(key):
            return
        def _put():
            self._client.put_object(
                Bucket=self.bucket,
                Key=s3_key,
                Body=data,
                ContentType=content_type,
            )
        await asyncio.to_thread(_put)

    async def get(self, key: str) -> tuple[bytes, str] | None:
        s3_key = self._key(key)
        def _get():
            try:
                resp = self._client.get_object(Bucket=self.bucket, Key=s3_key)
                data = resp["Body"].read()
                ct = resp.get("ContentType", "application/octet-stream")
                return data, ct
            except self._client.exceptions.NoSuchKey:
                return None
        return await asyncio.to_thread(_get)

    async def exists(self, key: str) -> bool:
        s3_key = self._key(key)
        def _head():
            try:
                self._client.head_object(Bucket=self.bucket, Key=s3_key)
                return True
            except Exception:
                return False
        return await asyncio.to_thread(_head)

    async def delete_all(self) -> None:
        log.warning("delete_all not supported for S3Store")


# ── Singleton ──────────────────────────────────────────────────────

_storage: ContentStore | None = None


def init_storage(settings) -> ContentStore:
    global _storage
    if settings.storage_backend == "s3":
        _storage = S3Store(bucket=settings.s3_bucket, prefix=settings.s3_prefix)
        log.info("Storage: S3 (bucket=%s, prefix=%s)", settings.s3_bucket, settings.s3_prefix)
    else:
        _storage = LocalStore(base_dir=settings.storage_dir)
        log.info("Storage: local (%s)", settings.storage_dir)
    return _storage


def get_storage() -> ContentStore:
    if _storage is None:
        raise RuntimeError("Storage not initialized — call init_storage() first")
    return _storage
