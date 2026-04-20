// experiment-60: Rustソースコードのビット誤り検出と訂正提案
// Galois LFSR GOLD列 / nibble保存則 / MBS-LBS偶奇 / 循環XCORR / 条件付エントロピー
// 再帰訂正深度: MAX_RECURSION=10

use plotters::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

const MAX_RECURSION: usize = 10;
const LFSR_POLY_A: u32 = 0b0110_0000_0000_0000_0000; // x^15+x^14: tap mask (16-bit reg)
const LFSR_POLY_B: u32 = 0b0101_0010_0000_0000_0000; // x^15+x^13+x^10: tap mask

// ── nibble ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Nibble(u8); // 0..=15

impl Nibble {
    fn mbs(self) -> u8 { (self.0 >> 3) & 1 }
    fn lbs(self) -> u8 { self.0 & 1 }
    fn parity(self) -> u8 { self.0.count_ones() as u8 & 1 }
    fn hamming_to(self, other: Nibble) -> u32 { (self.0 ^ other.0).count_ones() }
}

impl fmt::Debug for Nibble {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04b}({:X})", self.0, self.0)
    }
}

fn bytes_to_nibbles(data: &[u8]) -> Vec<Nibble> {
    data.iter()
        .flat_map(|&b| [Nibble(b >> 4), Nibble(b & 0xF)])
        .collect()
}

fn nibbles_to_bytes(nibs: &[Nibble]) -> Vec<u8> {
    nibs.chunks(2)
        .map(|c| (c[0].0 << 4) | c.get(1).map(|n| n.0).unwrap_or(0))
        .collect()
}

// ── Galois LFSR GOLD列 ───────────────────────────────────────────────────────

struct GoldSeq {
    bits: Vec<u8>,
}

impl GoldSeq {
    fn new(len: usize) -> Self {
        let mut a: u32 = 0x0001; // seed A
        let mut b: u32 = 0x0003; // seed B
        let mut bits = Vec::with_capacity(len);
        for _ in 0..len {
            let bit_a = a & 1;
            let bit_b = b & 1;
            bits.push((bit_a ^ bit_b) as u8);

            // Galois LFSR A (x^15 + x^14 + 1, 15-bit)
            let fb_a = bit_a ^ ((a >> 14) & 1);
            a = (a >> 1) | (fb_a << 14);

            // Galois LFSR B (x^15 + x^13 + x^10 + 1, 15-bit)
            let fb_b = bit_b ^ ((b >> 14) & 1) ^ ((b >> 12) & 1) ^ ((b >> 9) & 1);
            b = (b >> 1) | (fb_b << 14);
        }
        GoldSeq { bits }
    }

    fn bit(&self, i: usize) -> u8 {
        self.bits[i % self.bits.len()]
    }
}

// ── 循環XOR相互相関 ──────────────────────────────────────────────────────────

fn cyclic_xcorr(sig: &[u8], reference: &[u8]) -> Vec<f64> {
    let n = sig.len().min(reference.len());
    (0..n)
        .map(|shift| {
            let sum: i32 = (0..n)
                .map(|i| {
                    let s = sig[i] as i32 * 2 - 1;
                    let r = reference[(i + shift) % n] as i32 * 2 - 1;
                    s * r
                })
                .sum();
            sum as f64 / n as f64
        })
        .collect()
}

// ── 条件付エントロピー (OOV → ∞ = anchor) ───────────────────────────────────

fn conditional_entropy(token: &[u8], freq: &HashMap<Vec<u8>, usize>, total: usize) -> f64 {
    if total == 0 {
        return f64::INFINITY;
    }
    match freq.get(token) {
        None => f64::INFINITY,
        Some(&c) => {
            let p = c as f64 / total as f64;
            if p <= 0.0 { f64::INFINITY } else { -p.log2() }
        }
    }
}

// ── 誤り検出・訂正 (再帰深度 ≤ MAX_RECURSION) ───────────────────────────────

#[derive(Debug)]
struct Detection {
    nibble_pos: usize,
    original: Nibble,
    proposed: Nibble,
    confidence: f64,
    depth: usize,
    kind: DetectionKind,
}

#[derive(Debug)]
enum DetectionKind {
    ParityViolation,
    HammingOutlier,
    OovAnchor,
}

fn detect_errors(
    nibs: &[Nibble],
    gold: &GoldSeq,
    freq: &HashMap<Vec<u8>, usize>,
    corpus_total: usize,
    depth: usize,
) -> Vec<Detection> {
    if depth >= MAX_RECURSION {
        return vec![];
    }

    let mut dets: Vec<Detection> = Vec::new();

    for (i, &nib) in nibs.iter().enumerate() {
        let g = gold.bit(i * 4); // nibble位置ごとにGOLD参照ビット
        let expected_parity = g ^ ((i as u8) & 1);

        if nib.parity() != expected_parity {
            let (proposed, conf) = repair_nibble(nib, expected_parity, depth + 1);
            dets.push(Detection {
                nibble_pos: i,
                original: nib,
                proposed,
                confidence: conf,
                depth,
                kind: DetectionKind::ParityViolation,
            });
        }

        // nibbleをbyte境界で対にしてOOVチェック
        if i % 2 == 0 {
            let hi = nib;
            let lo = nibs.get(i + 1).copied().unwrap_or(Nibble(0));
            let token = vec![(hi.0 << 4) | lo.0];
            let h = conditional_entropy(&token, freq, corpus_total);
            if h.is_infinite() && depth < MAX_RECURSION {
                // OOVアンカー: Hamming最近傍に押す
                if let Some((best, dist)) = nearest_known(hi, lo, freq) {
                    if dist <= 2 {
                        dets.push(Detection {
                            nibble_pos: i,
                            original: hi,
                            proposed: best,
                            confidence: 1.0 / (dist as f64 + 1.0),
                            depth,
                            kind: DetectionKind::OovAnchor,
                        });
                    }
                }
            }
        }
    }

    dets
}

fn repair_nibble(nib: Nibble, target_parity: u8, depth: usize) -> (Nibble, f64) {
    if depth >= MAX_RECURSION {
        return (nib, 0.0);
    }
    // LBS優先でフリップ: 意味的変化を最小化
    for bit_pos in [0u8, 3, 1, 2] {
        let candidate = Nibble(nib.0 ^ (1 << bit_pos));
        if candidate.parity() == target_parity {
            let conf = match bit_pos {
                0 | 3 => 0.75 / (depth as f64 + 1.0),
                _ => 0.50 / (depth as f64 + 1.0),
            };
            return (candidate, conf);
        }
    }
    (nib, 0.0)
}

fn nearest_known(
    hi: Nibble,
    lo: Nibble,
    freq: &HashMap<Vec<u8>, usize>,
) -> Option<(Nibble, u32)> {
    let orig_byte = (hi.0 << 4) | lo.0;
    let mut best_nibble = hi;
    let mut best_dist = u32::MAX;
    for b in 0u8..=255 {
        let key = vec![b];
        if freq.contains_key(&key) {
            let dist = (orig_byte ^ b).count_ones();
            if dist < best_dist {
                best_dist = dist;
                best_nibble = Nibble(b >> 4);
            }
        }
    }
    if best_dist < u32::MAX {
        Some((best_nibble, best_dist))
    } else {
        None
    }
}

// ── コーパス頻度表構築 ───────────────────────────────────────────────────────

fn build_freq_table(corpus_dir: &str) -> (HashMap<Vec<u8>, usize>, usize) {
    let mut freq: HashMap<Vec<u8>, usize> = HashMap::new();
    let mut total = 0usize;
    if let Ok(entries) = fs::read_dir(corpus_dir) {
        for entry in entries.flatten() {
            if let Ok(data) = fs::read(entry.path()) {
                for b in &data {
                    *freq.entry(vec![*b]).or_insert(0) += 1;
                    total += 1;
                }
            }
        }
    }
    (freq, total)
}

// ── Plotters 可視化 ──────────────────────────────────────────────────────────

fn plot_results(
    out_path: &str,
    nibs: &[Nibble],
    detections: &[Detection],
    xcorr: &[f64],
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(out_path, (1200, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    let (top, bottom) = root.split_vertically(300);

    // 上段: nibble parity stream + エラー位置
    {
        let mut chart = ChartBuilder::on(&top)
            .x_label_area_size(0)
            .y_label_area_size(0)
            .build_cartesian_2d(0f64..nibs.len() as f64, -0.5f64..1.5f64)?;
        chart.configure_mesh().disable_mesh().draw()?;

        let parity_data: Vec<(f64, f64)> = nibs
            .iter()
            .enumerate()
            .map(|(i, n)| (i as f64, n.parity() as f64))
            .collect();
        chart.draw_series(LineSeries::new(parity_data, &BLUE.mix(0.5)))?;

        // エラー位置を赤丸
        let err_points: Vec<(f64, f64)> = detections
            .iter()
            .map(|d| (d.nibble_pos as f64, d.original.parity() as f64))
            .collect();
        chart.draw_series(
            err_points
                .iter()
                .map(|&(x, y)| Circle::new((x, y), 4, RED.filled())),
        )?;
    }

    // 下段: 循環XCORR
    {
        let n = xcorr.len();
        let ymin = xcorr.iter().cloned().fold(f64::INFINITY, f64::min);
        let ymax = xcorr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let margin = (ymax - ymin).max(0.1) * 0.1;

        let mut chart = ChartBuilder::on(&bottom)
            .x_label_area_size(0)
            .y_label_area_size(0)
            .build_cartesian_2d(0f64..n as f64, (ymin - margin)..(ymax + margin))?;
        chart.configure_mesh().disable_mesh().draw()?;

        let xcorr_data: Vec<(f64, f64)> = xcorr
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v))
            .collect();
        chart.draw_series(LineSeries::new(xcorr_data, &GREEN))?;
    }

    root.present()?;
    Ok(())
}

// ── メイン ───────────────────────────────────────────────────────────────────

fn main() {
    let corpus_dir = "../experiment-36/github_corpus";
    let hard_dir = "../experiment-38/rust_ui_dataset";
    let out_dir = "output";
    fs::create_dir_all(out_dir).ok();

    // コーパス頻度表
    let (freq, corpus_total) = build_freq_table(corpus_dir);
    println!("[corpus] {} バイトトークン, {} ユニーク", corpus_total, freq.len());

    // 難関ファイルを処理
    let hard_files: Vec<_> = fs::read_dir(hard_dir)
        .map(|rd| rd.flatten().collect())
        .unwrap_or_default();

    let mut total_detections = 0usize;
    let mut total_anchors = 0usize;

    for entry in &hard_files {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let nibs = bytes_to_nibbles(&data);
        let gold = GoldSeq::new(nibs.len() * 4 + 1);

        // 誤り検出 (再帰深度0スタート)
        let detections = detect_errors(&nibs, &gold, &freq, corpus_total, 0);

        // 訂正適用
        let mut corrected = nibs.clone();
        for d in &detections {
            if d.confidence > 0.1 {
                corrected[d.nibble_pos] = d.proposed;
            }
        }
        let corrected_bytes = nibbles_to_bytes(&corrected);
        let corrected_src = String::from_utf8_lossy(&corrected_bytes);

        // 循環XCORR: 入力パリティビットvsGOLD
        let sig: Vec<u8> = nibs.iter().map(|n| n.parity()).collect();
        let gold_ref: Vec<u8> = (0..sig.len()).map(|i| gold.bit(i)).collect();
        let xcorr = cyclic_xcorr(&sig, &gold_ref);

        // 最大相関位相 = 最有力エラー集中位置
        let (peak_shift, peak_val) = xcorr
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, &v)| (i, v))
            .unwrap_or((0, 0.0));

        // 可視化
        let out_png = format!("{out_dir}/{name}.png");
        if let Err(e) = plot_results(&out_png, &nibs, &detections, &xcorr) {
            eprintln!("[plot error] {e}");
        }

        let anchors = detections
            .iter()
            .filter(|d| matches!(d.kind, DetectionKind::OovAnchor))
            .count();
        total_anchors += anchors;
        total_detections += detections.len();

        // H_cond統計
        let h_vals: Vec<f64> = (0..data.len())
            .map(|i| conditional_entropy(&[data[i]], &freq, corpus_total))
            .collect();
        let h_finite: Vec<f64> = h_vals.iter().copied().filter(|v| v.is_finite()).collect();
        let h_oov = h_vals.iter().filter(|v| v.is_infinite()).count();
        let h_mean = if h_finite.is_empty() {
            0.0
        } else {
            h_finite.iter().sum::<f64>() / h_finite.len() as f64
        };

        println!(
            "\n=== {} ===",
            name
        );
        println!("  入力バイト数    : {}", data.len());
        println!("  nibble数        : {}", nibs.len());
        println!("  誤り検出数      : {}", detections.len());
        println!("  OOVアンカー数   : {anchors}");
        println!("  H_cond平均(有限): {h_mean:.4} bit");
        println!("  H_cond=∞ (OOV)  : {h_oov} / {}", data.len());
        println!("  XCORR ピーク位相: {peak_shift} (値={peak_val:.4})");
        println!("  出力PNG         : {out_png}");

        for d in detections.iter().take(5) {
            println!(
                "  [depth={depth}] nibble@{pos} {orig:?} → {prop:?} conf={conf:.3} ({kind:?})",
                depth = d.depth,
                pos = d.nibble_pos,
                orig = d.original,
                prop = d.proposed,
                conf = d.confidence,
                kind = d.kind,
            );
        }
        if detections.len() > 5 {
            println!("  ... (他{}件)", detections.len() - 5);
        }

        // 訂正後コードの先頭40文字を表示
        let preview: String = corrected_src.chars().take(40).collect();
        println!("  訂正後プレビュー: {preview:?}");
    }

    println!("\n=== 全体サマリー ===");
    println!("処理ファイル数      : {}", hard_files.len());
    println!("総誤り検出数        : {total_detections}");
    println!("総OOVアンカー数     : {total_anchors}");
    println!(
        "nibble保存則        : MBS/LBSフリップ優先, MAX_RECURSION={}",
        MAX_RECURSION
    );
    println!("可視化              : {out_dir}/");
}
