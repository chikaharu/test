# CLAUDE.md

This file documents the codebase structure, conventions, and workflows for AI assistants working in this repository.

## Repository Purpose

This is a **mathematical research repository** exploring binary compression theory, specifically minimizing conditional entropy H(X|ΦX) through geometric and information-theoretic methods. Experiments are numbered sequentially and tracked under `artifacts/bitrag/experiment-NN/`.

The research is primarily documented in Japanese. Code comments, variable names, and commit messages may also appear in Japanese.

## Repository Layout

```
/
├── README.md                        # Full research summary (Japanese, ~240 lines)
├── verify_model.pl                  # Perl model verifier for binary compression
├── CLAUDE.md                        # This file
└── artifacts/
    └── bitrag/
        ├── experiment-36/
        │   └── github_corpus/       # 35 Rust sample files used as test input
        ├── experiment-38/
        │   └── rust_ui_dataset/     # 5 hard Rust test cases
        ├── experiment-59/
        │   └── gold/                # Post-fix Rust reference files
        ├── experiment-60/           # Rust project: Galois LFSR / GOLD-code bit error detection
        │   ├── Cargo.toml
        │   ├── Cargo.lock
        │   ├── src/main.rs
        │   └── output/              # Generated PNG visualizations (do not edit)
        └── scheduler/               # Rust project: parallel job scheduler (qsub-style)
            ├── Cargo.toml
            ├── Cargo.lock
            └── src/main.rs
```

## Languages & Tooling

| Language | Usage |
|----------|-------|
| Rust (edition 2021) | Experimental algorithms and the scheduler utility |
| Perl 5 | Model verification (`verify_model.pl`) |
| Markdown | Research documentation |

**Rust dependencies:**
- `plotters = "0.3"` — PNG/SVG chart generation (experiment-60 only)

**No CI/CD** is configured. There are no GitHub Actions, Makefile, or test harnesses.

## Building & Running

### Rust projects

Each Rust project lives in its own directory with its own `Cargo.toml`.

```bash
# experiment-60 (bit error detection)
cd artifacts/bitrag/experiment-60
cargo build --release
cargo run --release          # writes PNGs to output/

# scheduler utility
cd artifacts/bitrag/scheduler
cargo build --release
cargo run --release -- "echo hello" -j 8
```

### Perl verifier

```bash
# Run on real files
perl verify_model.pl --input file.bmp

# Generate synthetic demo data and run
perl verify_model.pl --demo
```

## Key Source Files

### `artifacts/bitrag/experiment-60/src/main.rs`

Implements GOLD-sequence-based bit error detection and correction on binary data read from the `github_corpus` and `rust_ui_dataset` sample files.

Key structures and functions:
- `GoldSeq` — Galois LFSR producing a GOLD reference sequence (taps: x^15+x^14 and x^15+x^13+x^10)
- `Nibble` — 4-bit wrapper with parity helpers
- `DetectionKind` — enum: `ParityViolation`, `HammingOutlier`, `OovAnchor`
- `Detection` — error record (position, kind, confidence)
- `correct_errors()` — recursive correction (MAX_RECURSION = 10)
- `xcorr()` — cyclic cross-correlation of signal against GOLD reference
- Output: five PNG charts (1200×600) under `output/`

### `artifacts/bitrag/scheduler/src/main.rs`

Minimal parallel job runner invoked as:
```
scheduler "shell_command" [-j <parallelism>]
```
Default parallelism is 4 threads. Each job captures stdout/stderr and reports elapsed time in milliseconds.

### `verify_model.pl`

Validates the "onion-skin" binary compression model (see README.md §Theorem 11.5).

For each input file it computes, per depth k:
- Skin thickness T_k = sin(π / 2^k)
- Eigenvalue λ_k
- Boundary mass m_gauss (via `erf`)
- Information loss ΔI_k (bits)
- Convergence rate r_k ≈ 0.70

Supported input formats: BMP, AVI, WAV, UTF-8 text, raw sensor binary.

## Code Conventions

### Rust
- Struct-based design; no large trait hierarchies
- `Result<T, Box<dyn Error>>` for error propagation — keep it simple
- Bit-level manipulation is intentional and performance-sensitive; do not add unnecessary abstractions
- Generated output files (PNGs) go in `output/` and are not tracked in git

### Perl
- `use strict; use warnings; use utf8;` at the top of every script
- Clamp values to avoid numerical instability (epsilon = 1e-12 is common)
- `binmode(STDOUT, ':utf8')` for Unicode-safe output

### Commit messages
- Recent commits mix English and Japanese; either is acceptable
- Format: `<scope>: <description>` (e.g., `experiment-60: 説明`)

### Adding a new experiment
1. Create `artifacts/bitrag/experiment-NN/` with its own `Cargo.toml` (or script)
2. Add a section to `README.md` following the existing act/experiment numbering
3. If producing output files, put them under `experiment-NN/output/` and add that path to `.gitignore` if they are large

## Mathematical Context (for AI Assistants)

The core research question is: given a binary sequence X, find a linear map Φ that minimises H(X|ΦX).

Key concepts referenced throughout the code and docs:
- **Onion-skin model**: nested spheres with thickness T_k = sin(π/2^k)
- **BSC channel**: Binary Symmetric Channel information loss formula
- **Toeplitz vs circulant**: eigenvalues split along sin/cos axes (Theorem in README §5)
- **Szegő's theorem**: asymptotic eigenvalue distribution used to justify the model
- **r_k ≈ 0.70**: empirical convergence rate between depth levels

When reading code, assume that "GOLD sequence" refers to the GOLD spreading code from CDMA / GPS, not a coincidence of the word "gold".

## Branch & PR Conventions

- Feature branches follow the pattern `claude/<short-description>-<id>` for AI-generated branches
- No forced pushes to `main`
- No CI is required to pass before merging (none exists)

## What NOT to Do

- Do not delete or modify files under `artifacts/bitrag/experiment-*/output/` — these are generated artifacts kept for reference
- Do not add a `[dev-dependencies]` test harness without discussing it first; the experiments are exploratory, not production code
- Do not rewrite Japanese documentation into English without explicit instruction
- Do not run `cargo test` expecting passing tests — none are written
