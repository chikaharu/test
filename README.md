# Binary 圧縮の数学的探究 — プロンプト圧縮 v1

## 統一目標
$$\min_\Phi H(X \mid \Phi X)$$
データ $X \in \mathbb{R}^L$ を $K$ ビットに圧縮する最適写像 Φ を求める。

---

## I. 車輪の再発明リスト（既知理論の再発見）

| 再発見した内容 | 正式名 | 気づいた実験 |
|---|---|---|
| 最適 Φ = PCA 固有ベクトル | **Ky Fan 最大原理 (1949)** | exp011-014 |
| p_copy が増すと共分散の有効ランクが下がる | **スペクトル集中** (一般常識) | exp018-019 |
| AR(1) Toeplitz の固有ベクトル ≈ DFT | **Szegő の定理 (1915)** | exp022 |
| 有限 L で PCA > DFT | Szegő 定理の有限 L 補正 (既知) | exp023 |
| ソフトツリーの学習 = sigmoid 閾値最適化 | **ニューラルツリー** (既知手法) | exp003-006 |
| コンテキスト長 k の飽和 ≈ 相関長 | **有効記憶長** (情報理論の常識) | exp016-017 |

**教訓:** 「linear Φ の最適化」に数実験を費やして PCA を再発見。
これ自体は無駄だが、**なぜ PCA が最適か**の直感を実験で得たことは有益。

---

## II. 本当に新しかったもの

### ★1. 玉ねぎ薄皮モデル: 皮の厚み $T_k = \sin(\pi/2^k)$

**着想:** 二分木の深度 k のセル = S^{L-1} 上の弧幅 $2\pi/2^k$ の領域。
O(L) 測地距離で「セル半径」を弦長に変換:

$$T_k = \sin\!\left(\frac{\pi}{2^k}\right) \approx \frac{\pi}{2^k}$$

- k=1: T=1.0 (厚い皮), k=8: T=0.012 (極薄)
- **Gaussian で erf 公式と誤差 < 0.2% で一致** (exp025)

### ★2. BSC 情報損失の正しい公式

1次近似 $\Delta I \approx T_k \cdot p(0) \cdot \ln 2$ は **7〜14倍** 過小評価。

正しい式 (BSC モデル):
$$\boxed{\Delta I_k = H_{\text{bin}}\!\left(\frac{m_k}{2}\right) \approx \frac{m_k}{2}\log_2\frac{2e}{m_k}}$$

ここで $m_k = \text{erf}(T_k/\sqrt{2\lambda_k})$ は境界質量。

### ★3. 収束速度の因子分解

$$r_k = \frac{\Delta I_{k+1}}{\Delta I_k} = \underbrace{\frac{T_{k+1}}{T_k}\sqrt{\frac{\lambda_k}{\lambda_{k+1}}}}_{r_{\text{geom}} \approx 0.636} \times \underbrace{\frac{\log(2/m_{k+1})}{\log(2/m_k)}}_{r_{\text{log}} \approx 1.103} \approx 0.70$$

- r > 0.5 の理由: AR(1) の固有値比が小さい (λ_k/λ_{k+1} ≈ 1.4) + 対数補正 > 1
- **O(2^{-0.51k})**: 各層で半分ではなく **0.70倍** しか減らない

### ★4. Toeplitz ≠ 巡回行列 → sin/cos 固有値が分裂

**exp030 の核心的発見:**

Jacobi 固有値分解で解析的 Toeplitz $\rho^{|i-j|}$ を厳密に対角化:

| rank | 型 | λ | gap(次との差) |
|---|---|---|---|
| 1 | DC | 16.79 | 27% |
| 2 | **sin** k=1 | 12.26 | 33% |
| 3 | **cos** k=1 | 8.24 | 33% |
| 4 | **sin** k=2 | 5.52 | 31% |
| 5 | **cos** k=2 | 3.81 | 28% |

- **巡回行列**: cos(k) と sin(k) が同一固有値 (Szegő の世界)
- **Toeplitz**: λ_sin > λ_cos が常に成立 (gap 16〜33%)
- 原因: 非周期境界 — sin(0)=0 (端が寄与しない) vs cos(0)=1 (端が最大寄与)
- **これが R_k の振動を引き起こす**

### ★5. R_k の sin/cos 振動は本物

Binary AR(1) の境界質量補正比:

$$R_k = \frac{m_k^{\text{binary}}}{m_k^{\text{Gauss}}} = \begin{cases}
< 1 & \text{cos型 (DC, cos1, cos2, ...)} \\
> 1 & \text{sin型 (sin1, sin2, sin3, ...)}
\end{cases}$$

| rank | 型 | R_k |
|---|---|---|
| 1 | DC | 0.82 |
| 2 | sin1 | 1.05 |
| 3 | cos1 | 0.88 |
| 4 | sin2 | 1.26 |
| 5 | cos2 | 0.83 |
| 6 | sin3 | 1.73 |

**物理的理由:**
- sin 基底: u[0]=0 → X₀=±1 が寄与しない → 離散性が生き残る → R>1
- cos 基底: u[0]=max → X₀=±1 が最大寄与 → 離散ジャンプが平滑化 → R<1

**3つの構造の共鳴:**
1. Szegő 近似 (L→∞) → R_k = 1 (全て同一)
2. Toeplitz 有限境界 → sin/cos 分裂
3. Binary 離散性 → 分裂に方向依存性を付加

---

## III. 探究の三幕構造

### 第1幕 (exp001-015): 「Φ を学ぼう」

ソフトツリーで二分木分割を学習 → 驚き度で腐敗検出 → テキスト圧縮テスト → 線形 Φ の学習で PCA を再発見。

**残った問い:** PCA は最適だが、**なぜ sin(π/2^k) が厚みとして自然なのか？**

### 第2幕 (exp016-025): 「幾何を見よう」

コンテキスト圧縮で有効記憶長 → 共分散のランク構造 → AR(1) で DFT 対応を確認 → 複素球面学習 (PCA > DFT を実証) → 玉ねぎ薄皮の可視化 → 厚み T_k の定量化 (Gaussian 一致、Binary 乖離)

**残った問い:** Binary での乖離 m_binary ≠ m_theory の**正体は何か？**

### 第3幕 (exp026-030): 「なぜ振動する？」

BSC モデルで ΔI 収束速度 r=0.70 を解析 → 特性関数で m_k^binary を導出 → DFT 基底では振動せず → 「縮退アーティファクト」仮説 → **Jacobi で否定: Toeplitz の sin/cos は分裂している！** → 振動は本物、3つの構造の共鳴

---

## IV. 実験インフラ

- **言語:** Rust (plotters SVG 出力)
- **ディレクトリ:** `experiments/expXXX/` に `src/main.rs`, `readme`, `result`, `Cargo.toml`
- **SVG:** 絶対パス `/home/runner/workspace/experiments/expXXX/result.svg` 必須
- **plotters features:** `["svg_backend","full_palette","line_series","point_series"]`
- **乱数:** 自前 LCG (`Rng(u64)` 構造体)
- **線形代数:** 自前実装 (べき乗法 + deflation or Jacobi)
- **フォント:** `experiments/exp021/fonts/NotoSans-Regular.ttf`

---

## V. 定理の状態

### 確定したもの

| 命題 | 根拠 |
|---|---|
| 最適基底 = C の固有ベクトル | Ky Fan (1949) |
| AR(1) Toeplitz → DFT (L→∞) | Szegő (1915) |
| 皮の厚み T_k = sin(π/2^k) | O(L) 測地距離 + exp025 検証 |
| Gaussian: m_k = erf(T_k/√(2λ)) | 定義 + exp025 で誤差 < 0.2% |
| ΔI_k = H_bin(m_k/2) | BSC モデル + exp029 |
| 収束速度 r ≈ 0.70 | r_geom × r_log の因子分解 |
| Toeplitz: λ_sin > λ_cos | Jacobi 厳密対角化 (exp030) |
| R_k 振動は sin/cos 交互 | exp030 CF + Jacobi |

### 三補題の体系 (proof v6, §11)

| 補題 | 主張 | 状態 |
|---|---|---|
| **L.A** | $m_k = 2p_k^G(0) \cdot T_k \cdot R_k + O(T_k^3)$ | ✅ 証明完了 |
| **L.B** | $R_k^{(\cos)} < 1$, $R_k^{(\sin)} > 1$ (CF+転送行列で計算可能) | 🔶 符号の証明済み、閉形式 $g_1,g_2$ 未導出 |
| **L.C** | $T_k = \sin(\pi/2^k)$ は等角二分木の幾何的必然 + 最小性 | ✅ 証明完了 |
| **定理 11.5** | L.A+L.B+L.C → $\Delta I_k$ の完全評価式 | 🔶 L.B の閉形式に依存 |
| **OP-3** | 非 Gaussian 上界 → **L.B の $R_k$ 上界に帰着** | ✅ 帰着完了 |

### 未解決 (Open Problems)

| # | 問い | ヒント |
|---|---|---|
| OP-2' | R_k の閉じた式 $g_1, g_2$ (L.B 完全証明) | CF 転送行列の固有値解析 |
| OP-5 | U(K) 非可換性が最適解に与える影響 | BCH 公式 |
| OP-6 | Toeplitz の sin/cos gap の L 依存性 | 有限 L で O(1) → L→∞ で O(1/L) に縮退？ |

### 撤回した仮説

| 仮説 | 否定した実験 | 真相 |
|---|---|---|
| 「ΔI ≈ T_k·p(0)·ln2 で十分」 | exp026 | 7-14× ズレ。H_bin(m/2) が正しい |
| 「PCA の縮退が R_k 振動の原因」 | exp030 | 固有値 gap 16-33%、縮退ではない |
| 「DFT 基底での R_k が正しい」 | exp030 | DFT ≠ Toeplitz 固有ベクトル |

---

## VI. 数学的文脈の整理

### Lie 群の接続

| 群 | 役割 |
|---|---|
| O(L) | 分割超平面の回転群。測地距離 → T_k |
| Stiefel(K,L) = O(L)/O(L-K) | 圧縮行列 Φ の空間 |
| U(1) ⊂ O(2) | S¹ = 複素数極座標。exp024 の z_k |
| U(K) | K 次元複素圧縮。非可換 → DFT ≠ 最適 (OP-5) |

### 情報理論の接続

| 量 | 意味 |
|---|---|
| H(X\|ΦX) | 圧縮後の残余エントロピー (最小化したい) |
| m_k = P(\|s_k\| < T_k) | 境界帯の質量 (ハード→ソフト の曖昧さ) |
| ΔI_k = H_bin(m_k/2) | 深度 k での1ビット当たりの情報損失 |
| r_k ≈ 0.70 | 層ごとの情報損失の減衰率 |

### 玉ねぎの絵

```text
S^{L-1}  (データの球面)
  │
  ├─ k=1: T₁=1.0   ← 厚い皮 (ΔI=0.46 bit)
  ├─ k=2: T₂=0.71  ← まだ厚い
  ├─ k=3: T₃=0.38
  ├─ k=4: T₄=0.20  ← ここから薄い
  │  ...
  └─ k→∞: T→0     ← 測度ゼロ (ハード分割に収束)

各層は O(L) 測地距離 sin(π/2^k) の厚みを持つ。
Binary では sin 基底方向で厚く、cos 基底方向で薄い。
```

---

> 注: 元原稿（v2〜v13、exp026〜exp045、BLT/大定理の詳細）はこのリポジトリ外の研究ノートに保持。
> 本 README は「プロンプト圧縮 v1」の骨子をまとめたサマリ版。

---

## Perl 検証スクリプト

`verify_model.pl` を追加。bmp / avi / wav / txt(UTF-8) / センサーバイナリをビット列として読み込み、
各データセットについて `p_copy`, `rho`, `H(X)` と、深度 `k` ごとの
`T_k`, `lambda_k`, `m_gauss`, `ΔI_k`, `r_k` を表形式で出力します。

### 使い方

```bash
perl verify_model.pl \
  --bmp sample.bmp \
  --avi sample.avi \
  --wav sample.wav \
  --txt sample.txt \
  --sensor-bin sensor1024.bin \
  --kmax 8 --window-bits 4096
```

デモ実行（疑似データ生成付き）:

```bash
perl verify_model.pl --demo
```
