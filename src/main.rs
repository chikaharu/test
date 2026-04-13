use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

#[derive(Debug, Default)]
struct Cli {
    key_cols: Vec<String>,
    agg_col: String,
    emit_all: bool,
    bulk_sort: bool,
    min_values: Vec<String>,
    max_values: Vec<String>,
    strict: u8,
    file: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct UnitSig {
    pre: String,
    post: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct KeySig {
    key: Vec<String>,
    unit: UnitSig,
}

#[derive(Clone, Debug)]
struct ParsedValue {
    num: f64,
    unit: UnitSig,
}

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            2
        }
    };
    std::process::exit(code);
}

fn run() -> Result<(), String> {
    let cli = parse_args(env::args().skip(1).collect())?;
    if cli.strict > 3 {
        return Err("--strict must be 0..=3".to_string());
    }

    let mut input: Box<dyn Read> = if let Some(path) = &cli.file {
        Box::new(File::open(path).map_err(|e| format!("failed to open {path}: {e}"))?)
    } else {
        Box::new(io::stdin())
    };

    let mut content = String::new();
    input
        .read_to_string(&mut content)
        .map_err(|e| format!("failed to read input: {e}"))?;

    let mut min_limits: HashMap<UnitSig, f64> = HashMap::new();
    for raw in &cli.min_values {
        let pv = parse_value(raw)?;
        min_limits.insert(pv.unit, pv.num);
    }
    let mut max_limits: HashMap<UnitSig, f64> = HashMap::new();
    for raw in &cli.max_values {
        let pv = parse_value(raw)?;
        max_limits.insert(pv.unit, pv.num);
    }

    let mut stdout = io::stdout().lock();
    let mut reader = BufReader::new(content.as_bytes());
    let mut line = String::new();
    let mut block_running: Option<(KeySig, f64)> = None;
    let mut global_totals: BTreeMap<KeySig, f64> = BTreeMap::new();
    let mut key_order: Vec<KeySig> = Vec::new();
    let mut seen_keys: HashSet<KeySig> = HashSet::new();
    let mut strict3_units: HashMap<Vec<String>, HashSet<UnitSig>> = HashMap::new();

    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| format!("read failure: {e}"))?;
        if read == 0 {
            break;
        }
        line = line.trim_end_matches(['\n', '\r']).to_string();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let agg_idx = match parse_col_spec(&cli.agg_col, cols.len(), false) {
            Ok(v) => v,
            Err(e) => {
                handle_error(cli.strict, &e)?;
                continue;
            }
        };

        let key_indices = if cli.key_cols.len() == 1 && cli.key_cols[0] == "$0" {
            (0..cols.len())
                .filter(|i| *i != agg_idx)
                .collect::<Vec<_>>()
        } else {
            let mut idxs = Vec::new();
            for k in &cli.key_cols {
                match parse_col_spec(k, cols.len(), true) {
                    Ok(v) => idxs.push(v),
                    Err(e) => {
                        handle_error(cli.strict, &e)?;
                        idxs.clear();
                        break;
                    }
                }
            }
            if idxs.is_empty() {
                continue;
            }
            idxs
        };

        let pv = match parse_value(cols[agg_idx]) {
            Ok(v) => v,
            Err(e) => {
                handle_error(cli.strict, &e)?;
                continue;
            }
        };
        let key_values = key_indices
            .iter()
            .map(|i| cols[*i].to_string())
            .collect::<Vec<_>>();
        let sig = KeySig {
            key: key_values.clone(),
            unit: pv.unit.clone(),
        };

        if cli.strict == 3 {
            strict3_units
                .entry(key_values)
                .or_default()
                .insert(pv.unit.clone());
        }

        if cli.bulk_sort {
            let new_total = global_totals.get(&sig).copied().unwrap_or(0.0) + pv.num;
            check_limits(new_total, &pv.unit, &min_limits, &max_limits, &sig)?;
            global_totals.insert(sig.clone(), new_total);
            if seen_keys.insert(sig.clone()) {
                key_order.push(sig.clone());
            }
            if cli.emit_all {
                writeln!(stdout, "{}", format_output(&sig, new_total))
                    .map_err(|e| e.to_string())?;
            }
        } else {
            match &mut block_running {
                Some((running_key, total)) if *running_key == sig => {
                    *total += pv.num;
                    check_limits(*total, &pv.unit, &min_limits, &max_limits, &sig)?;
                    if cli.emit_all {
                        writeln!(stdout, "{}", format_output(&sig, *total))
                            .map_err(|e| e.to_string())?;
                    }
                }
                Some((running_key, total)) => {
                    if !cli.emit_all {
                        writeln!(stdout, "{}", format_output(running_key, *total))
                            .map_err(|e| e.to_string())?;
                    }
                    *running_key = sig.clone();
                    *total = pv.num;
                    check_limits(*total, &pv.unit, &min_limits, &max_limits, &sig)?;
                    if cli.emit_all {
                        writeln!(stdout, "{}", format_output(&sig, *total))
                            .map_err(|e| e.to_string())?;
                    }
                }
                None => {
                    block_running = Some((sig.clone(), pv.num));
                    check_limits(pv.num, &pv.unit, &min_limits, &max_limits, &sig)?;
                    if cli.emit_all {
                        writeln!(stdout, "{}", format_output(&sig, pv.num))
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            *global_totals.entry(sig).or_insert(0.0) += pv.num;
        }
    }

    if !cli.bulk_sort {
        if let Some((k, t)) = block_running {
            if !cli.emit_all {
                writeln!(stdout, "{}", format_output(&k, t)).map_err(|e| e.to_string())?;
            }
        }
    } else if !cli.emit_all {
        for k in key_order {
            if let Some(t) = global_totals.get(&k) {
                writeln!(stdout, "{}", format_output(&k, *t)).map_err(|e| e.to_string())?;
            }
        }
    }

    if cli.strict == 3 {
        for (k, units) in strict3_units {
            if units.len() > 1 {
                return Err(format!(
                    "strict level 3 violation: mixed units for key {:?}",
                    k
                ));
            }
        }
    }

    if cli.emit_all {
        let mut stderr = io::stderr().lock();
        for (k, t) in global_totals {
            writeln!(stderr, "{}", format_output(&k, t)).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-k" => {
                i += 1;
                while i < args.len() && !args[i].starts_with('-') {
                    cli.key_cols.push(args[i].clone());
                    i += 1;
                }
                continue;
            }
            "-a" => {
                i += 1;
                cli.agg_col = args.get(i).ok_or("missing value for -a")?.clone();
            }
            "-A" | "--all" => cli.emit_all = true,
            "--bulk-sort" => cli.bulk_sort = true,
            "--min" => {
                i += 1;
                cli.min_values
                    .push(args.get(i).ok_or("missing value for --min")?.clone());
            }
            "--max" => {
                i += 1;
                cli.max_values
                    .push(args.get(i).ok_or("missing value for --max")?.clone());
            }
            "--strict" => {
                i += 1;
                cli.strict = args
                    .get(i)
                    .ok_or("missing value for --strict")?
                    .parse::<u8>()
                    .map_err(|_| "invalid --strict level")?;
            }
            "-h" | "--help" => {
                println!("croll -k COL [COL ...] -a COL [OPTIONS] [FILE]");
                std::process::exit(0);
            }
            x if x.starts_with('-') => return Err(format!("unknown option: {x}")),
            x => cli.file = Some(x.to_string()),
        }
        i += 1;
    }

    if cli.key_cols.is_empty() || cli.agg_col.is_empty() {
        return Err("-k and -a are required".to_string());
    }
    Ok(cli)
}

fn parse_col_spec(spec: &str, total_cols: usize, allow_all: bool) -> Result<usize, String> {
    if spec == "$0" {
        if allow_all {
            return Err("$0 is special and only valid as sole -k value".to_string());
        }
        return Err("$0 is invalid for -a".to_string());
    }
    if let Some(rest) = spec.strip_prefix('$') {
        let idx = rest
            .parse::<usize>()
            .map_err(|_| format!("invalid awk-style column: {spec}"))?;
        if idx == 0 || idx > total_cols {
            return Err(format!("column index out of range: {spec}"));
        }
        Ok(idx - 1)
    } else {
        let idx = spec
            .parse::<usize>()
            .map_err(|_| format!("invalid zero-based column: {spec}"))?;
        if idx >= total_cols {
            return Err(format!("column index out of range: {spec}"));
        }
        Ok(idx)
    }
}

fn parse_value(raw: &str) -> Result<ParsedValue, String> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let chars: Vec<char> = compact.chars().collect();
    let mut start = None;
    let mut end = None;
    for i in 0..chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() || c == '+' || c == '-' || c == '.' {
            if looks_like_number_start(&chars, i) {
                start = Some(i);
                break;
            }
        }
    }
    let s = start.ok_or_else(|| format!("invalid numeric value: {raw}"))?;
    for i in (s + 1)..=chars.len() {
        let candidate: String = chars[s..i].iter().collect();
        if valid_number(&candidate) {
            end = Some(i);
        } else if end.is_some() {
            break;
        }
    }
    let e = end.ok_or_else(|| format!("invalid numeric value: {raw}"))?;
    let pre: String = chars[..s].iter().collect();
    let num_s: String = chars[s..e].iter().collect();
    let post: String = chars[e..].iter().collect();
    let num = num_s
        .parse::<f64>()
        .map_err(|_| format!("invalid numeric value: {raw}"))?;
    Ok(ParsedValue {
        num,
        unit: UnitSig { pre, post },
    })
}

fn looks_like_number_start(chars: &[char], i: usize) -> bool {
    let c = chars[i];
    if c.is_ascii_digit() {
        return true;
    }
    if (c == '+' || c == '-') && i + 1 < chars.len() {
        return chars[i + 1].is_ascii_digit() || chars[i + 1] == '.';
    }
    c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()
}

fn valid_number(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    let mut i = 0;
    if matches!(bytes[0] as char, '+' | '-') {
        i += 1;
    }
    let rest = &s[i..];
    if rest.is_empty() {
        return false;
    }
    if rest.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    if let Some((l, r)) = rest.split_once('.') {
        let left_ok = l.chars().all(|c| c.is_ascii_digit());
        let right_ok = r.chars().all(|c| c.is_ascii_digit());
        return left_ok && right_ok && (!l.is_empty() || !r.is_empty());
    }
    false
}

fn handle_error(strict: u8, msg: &str) -> Result<(), String> {
    match strict {
        0 => Ok(()),
        1 => {
            eprintln!("warning: {msg}");
            Ok(())
        }
        _ => Err(msg.to_string()),
    }
}

fn format_output(sig: &KeySig, total: f64) -> String {
    let mut out = sig.key.join("\t");
    if !out.is_empty() {
        out.push('\t');
    }
    out.push_str(&format!(
        "{}{}{}",
        sig.unit.pre,
        trim_float(total),
        sig.unit.post
    ));
    out
}

fn trim_float(v: f64) -> String {
    let s = format!("{v:.10}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn check_limits(
    value: f64,
    unit: &UnitSig,
    min_limits: &HashMap<UnitSig, f64>,
    max_limits: &HashMap<UnitSig, f64>,
    sig: &KeySig,
) -> Result<(), String> {
    if let Some(min_v) = min_limits.get(unit) {
        if value < *min_v {
            return Err(format!("cumulative value below min for key {:?}", sig.key));
        }
    }
    if let Some(max_v) = max_limits.get(unit) {
        if value > *max_v {
            return Err(format!("cumulative value above max for key {:?}", sig.key));
        }
    }
    Ok(())
}
