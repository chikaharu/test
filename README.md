# text viewer

1MB 以上のテキストファイルと gzip 圧縮テキストを、バイト範囲指定で読むためのシンプルな viewer です。

## 仕様

- **通常テキスト**: `seek + read` で `start` から `length` バイトを読む。
- **gzip テキスト**: 8KiB ごとにインデックス（解凍状態）を作成し、近いチェックポイントから再開して部分読み取りする。
- 部分読み取りなので、指定範囲が行の途中なら先頭/末尾が欠けたテキストになります（想定どおり）。

## 使い方

```bash
python viewer.py ./sample.txt --start 1048576 --length 2048
python viewer.py ./sample.txt.gz --start 1048576 --length 2048
```

出力:

- 実際に読んだ範囲 (`start/end/bytes`)
- 取り出したテキスト

## テスト

```bash
pytest -q
```
