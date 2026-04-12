//! Threading demos: minimal spawn/join, then three independent parallel tasks with timing.

use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn log(msg: impl AsRef<str>) {
    let label = thread::current()
        .name()
        .unwrap_or("main")
        .to_string();
    println!("[{label:>20}] {}", msg.as_ref());
}

/// **Step 0 — absolute minimum:** split an array, two threads each sum a half, main joins and adds.
/// Shows: `spawn`, `move`, `JoinHandle`, and that work is duplicated only by partition (not by magic).
pub fn simple_parallel_sum_two_halves(values: &[i64]) -> i64 {
    log(format!(
        "splitting {} elements across 2 worker threads",
        values.len()
    ));
    let mid = values.len() / 2;
    let left = values[..mid].to_vec();
    let right = values[mid..].to_vec();

    let left_handle = thread::Builder::new()
        .name("worker-sum-left".into())
        .spawn(move || {
            let partial: i64 = left.iter().sum();
            log(format!(
                "finished left chunk: {} elements → partial sum = {partial}",
                left.len()
            ));
            partial
        })
        .expect("spawn left");

    let right_handle = thread::Builder::new()
        .name("worker-sum-right".into())
        .spawn(move || {
            let partial: i64 = right.iter().sum();
            log(format!(
                "finished right chunk: {} elements → partial sum = {partial}",
                right.len()
            ));
            partial
        })
        .expect("spawn right");

    let total = left_handle.join().expect("left panicked")
        + right_handle.join().expect("right panicked");
    log(format!("joined both halves → total sum = {total}"));
    total
}

/// Three **different** parallel problems on the same data (each does a full scan).
/// Returns `(sum, count_even, max)`.
pub fn three_parallel_tasks_on_slice(data: Arc<Vec<i64>>) -> (i64, usize, i64) {
    log("spawning 3 workers: sum | count evens | max (each scans full vector)");

    let d1 = Arc::clone(&data);
    let h_sum = thread::Builder::new()
        .name("task-sum".into())
        .spawn(move || {
            let t0 = Instant::now();
            let s: i64 = d1.iter().sum();
            log(format!(
                "sum done in {:?} → {s}",
                t0.elapsed()
            ));
            s
        })
        .expect("spawn sum");

    let d2 = Arc::clone(&data);
    let h_evens = thread::Builder::new()
        .name("task-count-evens".into())
        .spawn(move || {
            let t0 = Instant::now();
            let c = d2.iter().filter(|&&x| x % 2 == 0).count();
            log(format!(
                "even-count done in {:?} → {c} evens",
                t0.elapsed()
            ));
            c
        })
        .expect("spawn evens");

    let d3 = Arc::clone(&data);
    let h_max = thread::Builder::new()
        .name("task-max".into())
        .spawn(move || {
            let t0 = Instant::now();
            let m = *d3.iter().max().unwrap_or(&i64::MIN);
            log(format!(
                "max done in {:?} → {m}",
                t0.elapsed()
            ));
            m
        })
        .expect("spawn max");

    (
        h_sum.join().expect("sum panicked"),
        h_evens.join().expect("evens panicked"),
        h_max.join().expect("max panicked"),
    )
}

/// Run the three tasks **one after another** on the main thread (same total work, no overlap).
fn three_sequential_passes(data: &[i64]) -> (i64, usize, i64) {
    log("sequential: sum, then count evens, then max (3 full passes, main thread only)");
    let t0 = Instant::now();
    let sum: i64 = data.iter().sum();
    log(format!("sequential sum in {:?} → {sum}", t0.elapsed()));
    let t1 = Instant::now();
    let evens = data.iter().filter(|&&x| x % 2 == 0).count();
    log(format!(
        "sequential evens in {:?} → {evens}",
        t1.elapsed()
    ));
    let t2 = Instant::now();
    let max = *data.iter().max().unwrap_or(&i64::MIN);
    log(format!("sequential max in {:?} → {max}", t2.elapsed()));
    (sum, evens, max)
}

/// **Iterations:** grow `n` so you see when parallelism wins vs thread overhead.
pub fn run_parallelism_iterations() {
    let sizes = [50_000_usize, 500_000, 2_000_000];

    for (i, &n) in sizes.iter().enumerate() {
        log(format!(
            "\n──────── iteration {} — n = {n} elements ────────",
            i + 1
        ));
        let data: Vec<i64> = (0..n as i64).map(|k| k.wrapping_mul(31) ^ 0x5A5A).collect();
        let arc = Arc::new(data);

        let t_seq = Instant::now();
        let seq = three_sequential_passes(&arc);
        let seq_total = t_seq.elapsed();

        let t_par = Instant::now();
        let par = three_parallel_tasks_on_slice(Arc::clone(&arc));
        let par_wall = t_par.elapsed();

        debug_assert_eq!(seq.0, par.0);
        debug_assert_eq!(seq.1, par.1);
        debug_assert_eq!(seq.2, par.2);

        log(format!(
            "RESULT sequential wall-clock: {seq_total:?} (3 passes stacked)"
        ));
        log(format!(
            "RESULT parallel wall-clock:   {par_wall:?} (~1 pass if cores available; plus join overhead)"
        ));
        if par_wall < seq_total {
            let pct = 100.0 * (1.0 - par_wall.as_secs_f64() / seq_total.as_secs_f64());
            log(format!(
                "benefit this iteration: parallel ~{pct:.1}% faster than sequential wall time"
            ));
        } else {
            log(
                "no wall-clock win here: overhead or saturated CPU — try larger n or release build",
            );
        }
    }
}

/// Entry: simple sum demo, then three-task parallelism + timing loop.
pub fn run() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║ Rust threads: simple sum → 3 parallel tasks → iterations  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // --- 1) Smallest meaningful example ---
    let small: Vec<i64> = (1..=10).collect();
    log("Part 1 — two threads summing 1..=10");
    let got = simple_parallel_sum_two_halves(&small);
    println!(
        "  expected {}, got {} (parallel chunking preserves total)\n",
        small.iter().sum::<i64>(),
        got
    );

    // --- 2) Three problems at once + repeated sizes ---
    log("Part 2 — same vector, 3 independent scans in parallel vs back-to-back");
    run_parallelism_iterations();

    log("Done.");
}
