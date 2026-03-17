from __future__ import annotations

import argparse
import bisect
import os
import zlib
from dataclasses import dataclass
from pathlib import Path


@dataclass
class SliceResult:
    start: int
    end: int
    text: str


class PlainTextViewer:
    def __init__(self, path: str | os.PathLike[str]) -> None:
        self.path = Path(path)

    def read_slice(self, start: int, length: int, encoding: str = "utf-8") -> SliceResult:
        if start < 0 or length < 0:
            raise ValueError("start and length must be non-negative")

        with self.path.open("rb") as f:
            f.seek(start)
            data = f.read(length)

        return SliceResult(start=start, end=start + len(data), text=data.decode(encoding, errors="replace"))


@dataclass
class GzipCheckpoint:
    uncompressed_offset: int
    compressed_offset: int
    state: zlib.Decompress


class GzipIndexedViewer:
    """gzip を 8KiB ごとにインデックス化して部分読み取りする viewer."""

    def __init__(self, path: str | os.PathLike[str], checkpoint_span: int = 8192) -> None:
        self.path = Path(path)
        self.checkpoint_span = checkpoint_span
        self._checkpoints: list[GzipCheckpoint] = []
        self._build_index()

    def _build_index(self) -> None:
        decomp = zlib.decompressobj(wbits=31)
        self._checkpoints = [GzipCheckpoint(0, 0, decomp.copy())]

        compressed_offset = 0
        uncompressed_offset = 0
        next_checkpoint = self.checkpoint_span
        pending = b""

        with self.path.open("rb") as f:
            eof = False
            while not eof:
                if not pending:
                    chunk = f.read(64 * 1024)
                    if not chunk:
                        eof = True
                        break
                    pending = chunk

                need = max(1, next_checkpoint - uncompressed_offset)
                out = decomp.decompress(pending, need)

                consumed = len(pending) - len(decomp.unconsumed_tail)
                compressed_offset += consumed
                pending = decomp.unconsumed_tail
                uncompressed_offset += len(out)

                if uncompressed_offset >= next_checkpoint:
                    self._checkpoints.append(
                        GzipCheckpoint(
                            uncompressed_offset=uncompressed_offset,
                            compressed_offset=compressed_offset,
                            state=decomp.copy(),
                        )
                    )
                    next_checkpoint += self.checkpoint_span

                if decomp.eof:
                    eof = True

        if self._checkpoints[-1].uncompressed_offset != uncompressed_offset:
            self._checkpoints.append(
                GzipCheckpoint(uncompressed_offset, compressed_offset, decomp.copy())
            )

    @property
    def indexed_checkpoints(self) -> int:
        return len(self._checkpoints)

    def read_slice(self, start: int, length: int, encoding: str = "utf-8") -> SliceResult:
        if start < 0 or length < 0:
            raise ValueError("start and length must be non-negative")

        offsets = [cp.uncompressed_offset for cp in self._checkpoints]
        cp = self._checkpoints[max(0, bisect.bisect_right(offsets, start) - 1)]

        decomp = cp.state.copy()
        remaining_skip = start - cp.uncompressed_offset
        target = length
        collected = bytearray()

        with self.path.open("rb") as f:
            f.seek(cp.compressed_offset)
            pending = b""

            while target > 0:
                if not pending:
                    chunk = f.read(64 * 1024)
                    if not chunk:
                        break
                    pending = chunk

                need = remaining_skip + target
                out = decomp.decompress(pending, need)
                pending = decomp.unconsumed_tail

                if remaining_skip:
                    skip_now = min(remaining_skip, len(out))
                    out = out[skip_now:]
                    remaining_skip -= skip_now

                if out:
                    take = min(target, len(out))
                    collected.extend(out[:take])
                    target -= take

                if decomp.eof and not pending:
                    break

        return SliceResult(start=start, end=start + len(collected), text=collected.decode(encoding, errors="replace"))


def is_gzip(path: Path) -> bool:
    with path.open("rb") as f:
        return f.read(2) == b"\x1f\x8b"


def create_viewer(path: str | os.PathLike[str]):
    p = Path(path)
    return GzipIndexedViewer(p) if is_gzip(p) else PlainTextViewer(p)


def main() -> int:
    parser = argparse.ArgumentParser(description="Byte-range text viewer (plain/gzip)")
    parser.add_argument("file")
    parser.add_argument("--start", type=int, default=0)
    parser.add_argument("--length", type=int, default=1024)
    parser.add_argument("--encoding", default="utf-8")
    args = parser.parse_args()

    viewer = create_viewer(args.file)
    result = viewer.read_slice(args.start, args.length, encoding=args.encoding)

    print(f"start={result.start} end={result.end} bytes={result.end - result.start}")
    print(result.text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
