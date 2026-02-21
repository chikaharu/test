# EC Order Webhook

ECサイトの注文Webhookを受け取り、PostgreSQLに保存するサンプルです。

## 仕様

- `POST /webhooks/orders` で注文を受信
- カート内商品は `JSONB` で保存（`[{"SKU":"...","qty":...}]`）
- 以下の場合は `status=hold` で保留
  - 郵便番号と住所内郵便番号が不一致
  - 住所文字列に半角/全角の数字が含まれない

## セットアップ

```bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## DB作成

```bash
psql "$DATABASE_URL" -f schema.sql
```

## 起動

```bash
export DATABASE_URL='postgresql://user:pass@localhost:5432/mydb'
uvicorn app:app --reload --host 0.0.0.0 --port 8000
```

## リクエスト例

```json
{
  "order_id": "ORDER-1001",
  "postal_code": "123-4567",
  "address": "〒123-4567 東京都千代田区丸の内1-1",
  "cart_items": [
    {"sku": "ABC-001", "qty": 2},
    {"sku": "XYZ-002", "qty": 1}
  ]
}
```
