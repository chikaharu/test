import json
import os
from typing import Any

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field, field_validator
import psycopg

from validation import normalize_postal_code, validate_address


class CartItem(BaseModel):
    sku: str = Field(..., min_length=1)
    qty: int = Field(..., gt=0)


class OrderPayload(BaseModel):
    order_id: str = Field(..., min_length=1)
    postal_code: str = Field(..., min_length=1)
    address: str = Field(..., min_length=1)
    cart_items: list[CartItem]

    @field_validator("cart_items")
    @classmethod
    def cart_items_must_not_be_empty(cls, value: list[CartItem]) -> list[CartItem]:
        if not value:
            raise ValueError("cart_items must not be empty")
        return value


def get_db_connection() -> psycopg.Connection:
    dsn = os.getenv("DATABASE_URL")
    if not dsn:
        raise RuntimeError("DATABASE_URL is required")
    return psycopg.connect(dsn)


app = FastAPI(title="EC Order Webhook")


@app.post("/webhooks/orders")
def ingest_order(payload: OrderPayload) -> dict[str, Any]:
    validation = validate_address(payload.postal_code, payload.address)

    insert_sql = """
        INSERT INTO orders (order_id, postal_code, address, cart_items, status, hold_reason)
        VALUES (%s, %s, %s, %s::jsonb, %s, %s)
        RETURNING id
    """

    cart_json = json.dumps([{"SKU": item.sku, "qty": item.qty} for item in payload.cart_items], ensure_ascii=False)

    try:
        with get_db_connection() as conn:
            with conn.cursor() as cur:
                cur.execute(
                    insert_sql,
                    (
                        payload.order_id,
                        normalize_postal_code(payload.postal_code),
                        payload.address,
                        cart_json,
                        validation.status,
                        validation.reason,
                    ),
                )
                inserted_id = cur.fetchone()
            conn.commit()
    except RuntimeError as exc:
        raise HTTPException(status_code=500, detail=str(exc)) from exc

    return {
        "id": inserted_id[0] if inserted_id else None,
        "order_id": payload.order_id,
        "status": validation.status,
        "hold_reason": validation.reason,
    }
