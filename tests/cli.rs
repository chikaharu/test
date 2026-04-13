use std::path::PathBuf;
use std::process::Command;

fn bin() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_croll") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current exe");
    exe.parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("croll"))
        .expect("bin path")
}

#[test]
fn default_mode_emits_block_totals() {
    let output = Command::new(bin())
        .args(["-k", "0", "1", "-a", "2", "tests/data/block.tsv"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "A\tx\t3kg\nB\ty\t3kg\nA\tx\t4kg\n"
    );
}

#[test]
fn all_mode_emits_rows_and_summary_on_stderr() {
    let output = Command::new(bin())
        .args(["-k", "0", "-a", "1", "-A", "tests/data/units.tsv"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "item1\t1L\nitem1\t1000mL\nitem1\t2L\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("item1\t3L"));
    assert!(stderr.contains("item1\t1000mL"));
}

#[test]
fn bulk_sort_aggregates_across_stream() {
    let output = Command::new(bin())
        .args([
            "-k",
            "0",
            "1",
            "-a",
            "2",
            "--bulk-sort",
            "tests/data/block.tsv",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "A\tx\t7kg\nB\ty\t3kg\n"
    );
}

#[test]
fn strict3_rejects_mixed_units_for_same_key() {
    let output = Command::new(bin())
        .args([
            "-k",
            "0",
            "-a",
            "1",
            "--strict",
            "3",
            "tests/data/units.tsv",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("mixed units"));
}

#[test]
fn min_limit_is_enforced() {
    let output = Command::new(bin())
        .args([
            "-k",
            "0",
            "1",
            "-a",
            "2",
            "--min",
            "4kg",
            "tests/data/block.tsv",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("below min"));
}
