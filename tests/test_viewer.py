import gzip
from pathlib import Path

from viewer import GzipIndexedViewer, PlainTextViewer


def test_plain_text_slice(tmp_path: Path):
    p = tmp_path / "big.txt"
    p.write_text("\n".join(f"line-{i}" for i in range(1000)), encoding="utf-8")

    viewer = PlainTextViewer(p)
    result = viewer.read_slice(10, 40)

    assert result.start == 10
    assert result.end == 50
    assert len(result.text.encode("utf-8")) == 40


def test_gzip_indexed_slice(tmp_path: Path):
    raw = "\n".join(f"line-{i:05d}" for i in range(5000)).encode("utf-8")
    gz = tmp_path / "big.txt.gz"
    with gzip.open(gz, "wb") as f:
        f.write(raw)

    viewer = GzipIndexedViewer(gz, checkpoint_span=8192)
    result = viewer.read_slice(9000, 300)

    assert result.start == 9000
    assert result.end == 9300
    assert result.text.encode("utf-8") == raw[9000:9300]
    assert viewer.indexed_checkpoints > 2
