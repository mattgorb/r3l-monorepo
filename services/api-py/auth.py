from fastapi import Header, HTTPException

import db


async def require_api_key(x_api_key: str = Header(...)) -> dict:
    """Check customers table first, then org_api_keys. Returns enriched dict."""
    customer = await db.get_customer_by_api_key(x_api_key)
    if customer:
        customer["type"] = "individual"
        return customer

    # Check org API keys
    org_key = await db.get_org_api_key(x_api_key)
    if org_key:
        org = await db.get_organization_by_id(org_key["org_id"])
        return {
            "type": "org",
            "org_id": org_key["org_id"],
            "org_domain": org["domain"] if org else None,
            "org_verified": org["verified"] if org else False,
            "role": org_key["role"],
            "email": org_key["email"],
            "api_key": org_key["api_key"],
        }

    raise HTTPException(401, "invalid API key")


async def require_org_admin(x_api_key: str = Header(...)) -> dict:
    """Require an org API key with admin role."""
    org_key = await db.get_org_api_key(x_api_key)
    if not org_key or org_key["role"] != "admin":
        raise HTTPException(403, "requires org admin API key")
    org = await db.get_organization_by_id(org_key["org_id"])
    if not org:
        raise HTTPException(404, "organization not found")
    return {"org": org, "key": org_key}
