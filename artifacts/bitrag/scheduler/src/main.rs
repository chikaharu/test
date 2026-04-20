// グリッドエンジン風ジョブスケジューラ
// 使用法: qsub "shell command" [並列数]
use std::env;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct JobResult {
    id: usize,
    cmd: String,
    stdout: String,
    stderr: String,
    exit_code: i32,
    elapsed_ms: u128,
}

fn run_job(id: usize, cmd: &str) -> JobResult {
    let t0 = Instant::now();
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(o) => JobResult {
            id,
            cmd: cmd.to_string(),
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            exit_code: o.status.code().unwrap_or(-1),
            elapsed_ms: t0.elapsed().as_millis(),
        },
        Err(e) => JobResult {
            id,
            cmd: cmd.to_string(),
            stdout: String::new(),
            stderr: format!("launch error: {e}"),
            exit_code: -1,
            elapsed_ms: t0.elapsed().as_millis(),
        },
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("使用法: qsub \"cmd1\" [\"cmd2\" ...] [-j N]");
        std::process::exit(1);
    }

    // -j N で並列数
    let mut parallelism = 4usize;
    let mut cmds: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-j" {
            i += 1;
            if i < args.len() {
                parallelism = args[i].parse().unwrap_or(4);
            }
        } else {
            cmds.push(args[i].clone());
        }
        i += 1;
    }

    if cmds.is_empty() {
        eprintln!("ジョブなし");
        std::process::exit(1);
    }

    let parallelism = parallelism.min(cmds.len());
    let results: Arc<Mutex<Vec<JobResult>>> = Arc::new(Mutex::new(Vec::new()));
    let job_queue: Arc<Mutex<Vec<(usize, String)>>> = Arc::new(Mutex::new(
        cmds.into_iter().enumerate().collect(),
    ));

    eprintln!("[scheduler] {} ジョブ / 並列数 {}", {
        let q = job_queue.lock().unwrap();
        q.len()
    }, parallelism);

    let mut handles = Vec::new();
    for _ in 0..parallelism {
        let queue = Arc::clone(&job_queue);
        let res = Arc::clone(&results);
        let h = thread::spawn(move || loop {
            let job = {
                let mut q = queue.lock().unwrap();
                q.pop()
            };
            match job {
                None => break,
                Some((id, cmd)) => {
                    eprintln!("[scheduler] job-{id} 開始: {cmd}");
                    let r = run_job(id, &cmd);
                    eprintln!("[scheduler] job-{id} 完了 exit={} {}ms", r.exit_code, r.elapsed_ms);
                    res.lock().unwrap().push(r);
                }
            }
            thread::sleep(Duration::from_millis(10));
        });
        handles.push(h);
    }

    for h in handles { h.join().unwrap(); }

    let mut all = results.lock().unwrap();
    all.sort_by_key(|r| r.id);

    println!("=== SCHEDULER RESULTS ===");
    for r in all.iter() {
        println!("--- job-{} exit={} {}ms ---", r.id, r.exit_code, r.elapsed_ms);
        println!("CMD: {}", r.cmd);
        if !r.stdout.is_empty() { print!("STDOUT:\n{}", r.stdout); }
        if !r.stderr.is_empty() { print!("STDERR:\n{}", r.stderr); }
    }
    println!("=== END ===");
}
