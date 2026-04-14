//! Threading demos: minimal spawn/join, vector scans, then mixed blocking + CPU with four threads.

use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Reset all SGR attributes (color, bold, …).
const RESET: &str = "\x1b[0m";

/// 256-color ANSI foreground per thread name (Windows Terminal, VS Code, and most modern consoles).
fn color_for_thread(name: &str) -> &'static str {
    match name {
        "main" => "\x1b[38;5;255m",           // bright gray / “white”
        "worker-sum-left" => "\x1b[38;5;204m", // pink-red
        "worker-sum-right" => "\x1b[38;5;83m", // green
        "task-sum" => "\x1b[38;5;45m",        // cyan
        "task-count-evens" => "\x1b[38;5;220m", // gold
        "task-max" => "\x1b[38;5;141m",       // violet
        "sim-disk-read" => "\x1b[38;5;33m",   // blue
        "sim-http-get" => "\x1b[38;5;27m",    // deeper blue
        "cpu-fibonacci" => "\x1b[38;5;118m",  // lime
        "cpu-scalar-mix" => "\x1b[38;5;208m", // orange
        "race-writer-1" => "\x1b[38;5;199m",  // magenta
        "race-writer-2" => "\x1b[38;5;39m",   // light blue
        "race-writer-3" => "\x1b[38;5;154m",  // spring green
        _ => "\x1b[38;5;245m",                // unknown thread
    }
}

fn log(msg: impl AsRef<str>) {
    let label = thread::current()
        .name()
        .unwrap_or("main")
        .to_string();
    let prefix = color_for_thread(label.as_str());
    println!(
        "{prefix}[{label:>20}] {msg}{reset}",
        msg = msg.as_ref(),
        reset = RESET,
    );
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

fn fib_u128(n: u32) -> Option<u128> {
    if n > 128 {
        return None; 
    }

    let mut a = 0u128;
    let mut b = 1u128;
    for _ in 0..n {
        let t = a.saturating_add(b);
        a = b;
        b = t;
    }
    Some(a)
}

/// Deterministic “pure CPU” work without allocating or walking a slice — just a hot scalar loop.
fn scalar_mix_iterations(rounds: u32) -> u64 {
    let mut x: u64 = 0xC0FFEE;
    for _ in 0..rounds {
        x = x
            .rotate_left(13)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(1);
    }
    x
}

/// **Four threads, no arrays:** overlap *blocking-style* waits (`sleep` ≈ disk/network) with unrelated CPU.
/// Sequential wall time stacks every wait and every compute; parallel wall time is closer to the longest lane.
fn four_threads_blocking_plus_cpu_demo() {
    log("\n──────── Part 3 — four workers (no array traversal) ────────");
    log("Each thread does a different *kind* of work: two simulated I/O waits, Fibonacci, scalar crunch.");
    log("While one thread sleeps, the OS can run others — that is the thread story for blocking workloads.\n");

    const MIX_ROUNDS: u32 = 55_000_000;

    log("--- baseline: run all four jobs back-to-back on main ---");
    let t_seq = Instant::now();
    log("(seq) simulated disk read…");
    thread::sleep(Duration::from_millis(220));
    log("(seq) simulated HTTP request…");
    thread::sleep(Duration::from_millis(280));
    let t_fib = Instant::now();
    let fib_option = fib_u128(80);
    let fib_v: u128 = fib_option.unwrap_or(0u128);
    log(format!("(seq) fib(80) = {fib_v} in {:?}", t_fib.elapsed()));
    let t_mix = Instant::now();
    let mix_v = scalar_mix_iterations(MIX_ROUNDS);
    log(format!("(seq) scalar mix → {mix_v} in {:?}", t_mix.elapsed()));
    let seq_total = t_seq.elapsed();
    log(format!("SEQUENTIAL wall-clock: {seq_total:?} (sleeps + CPU add up)\n"));

    log("--- four named threads: same jobs, started together ---");
    let t_par = Instant::now();

    let h_disk = thread::Builder::new()
        .name("sim-disk-read".into())
        .spawn(|| {
            log("waiting on fake disk I/O…");
            thread::sleep(Duration::from_millis(220));
            log("disk path unblocked");
        })
        .expect("spawn disk");

    let h_net = thread::Builder::new()
        .name("sim-http-get".into())
        .spawn(|| {
            log("waiting on fake network…");
            thread::sleep(Duration::from_millis(280));
            log("network path unblocked");
        })
        .expect("spawn net");

    let h_fib = thread::Builder::new()
        .name("cpu-fibonacci".into())
        .spawn(move || {
            let t0 = Instant::now();
            let v = fib_u128(80);
            let fib_v: u128 = v.unwrap_or(0u128);
            log(format!("fib(80) = {fib_v} (compute) in {:?}", t0.elapsed()));
            v
        })
        .expect("spawn fib");

    let h_mix = thread::Builder::new()
        .name("cpu-scalar-mix".into())
        .spawn(move || {
            let t0 = Instant::now();
            let v = scalar_mix_iterations(MIX_ROUNDS);
            log(format!("scalar mix → {v} (compute) in {:?}", t0.elapsed()));
            v
        })
        .expect("spawn mix");

    h_disk.join().expect("disk panicked");
    h_net.join().expect("net panicked");
    let _fib = h_fib.join().expect("fib panicked");
    let _mix = h_mix.join().expect("mix panicked");

    let par_total = t_par.elapsed();
    log(format!("PARALLEL wall-clock: {par_total:?} (sleeps overlap; CPU runs alongside waits if cores allow)"));

    if par_total < seq_total {
        let pct = 100.0 * (1.0 - par_total.as_secs_f64() / seq_total.as_secs_f64());
        log(format!("→ parallel saved about {pct:.0}% wall time vs stacking the same four jobs"));
    } else {
        log("→ if parallel wasn’t faster, CPU work may dominate; try `cargo run --release` or raise MIX_ROUNDS");
    }
}

/// Part 4: shared mutable state with 3 threads, synchronized by Mutex.
/// This demonstrates race-condition prevention and lock contention.
fn race_condition_handled_with_mutex_demo() {
    log("\n──────── Part 4 — race condition handling with Mutex (3 threads) ────────");
    log("Scenario: three workers append to the same shared journal.");
    log("Without a lock this is a race; with Mutex each write section becomes atomic.\n");

    let shared_log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    // Turn coordinator: enforce writer order 1 -> 2 -> 3 for lock entry.
    let turn: Arc<(Mutex<usize>, Condvar)> = Arc::new((Mutex::new(1), Condvar::new()));
    let mut handles = Vec::new();

    for worker_id in 1..=3 {
        let shared = Arc::clone(&shared_log);
        let turn_state = Arc::clone(&turn);
        let name = format!("race-writer-{worker_id}");

        let handle = thread::Builder::new()
            .name(name.clone())
            .spawn(move || {
                for step in 1..=3 {
                    // Different preparation delays make completion order non-deterministic.
                    let prep_ms = match worker_id {
                        1 => 35,
                        2 => 10,
                        _ => 22,
                    } * step as u64;
                    thread::sleep(Duration::from_millis(prep_ms));

                    log(format!(
                        "step {step}: finished prep in {prep_ms}ms, now waiting to acquire lock..."
                    ));

                    let (turn_lock, turn_cv) = &*turn_state;
                    let mut current = turn_lock.lock().expect("turn mutex poisoned");
                    while *current != worker_id {
                        current = turn_cv.wait(current).expect("turn wait poisoned");
                    }

                    let mut guard = shared.lock().expect("mutex poisoned");
                    log(format!("step {step}: acquired lock, writing shared state"));

                    guard.push(format!(
                        "writer-{worker_id} wrote step-{step} (prep {prep_ms}ms)"
                    ));

                    // Hold lock briefly to make waiting visible.
                    thread::sleep(Duration::from_millis(45));
                    log(format!("step {step}: releasing lock"));
                    drop(guard);

                    *current = if worker_id == 3 { 1 } else { worker_id + 1 };
                    turn_cv.notify_all();
                }
            })
            .expect("spawn race demo worker");

        handles.push(handle);
    }

    for h in handles {
        h.join().expect("race worker panicked");
    }

    let final_journal = shared_log.lock().expect("mutex poisoned");
    log(format!(
        "all threads joined; shared journal length = {}",
        final_journal.len()
    ));
    for (i, line) in final_journal.iter().enumerate() {
        println!("  {:>2}. {line}", i + 1);
    }
    log("Notice: a fast thread can reach the lock first, but must wait if another thread is inside.");
}

/// **Iterations:** grow `n` so you see when parallelism wins vs thread overhead (two sizes only).
pub fn run_parallelism_iterations() {
    let sizes = [50_000_usize, 500_000];

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

/// Entry: simple sum → vector parallelism (2 iters) → four-thread blocking/CPU mix.
pub fn run() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║ Rust threads: sum → 3-way scan (×2) → 4 threads (no arrays) ║");
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

    four_threads_blocking_plus_cpu_demo();
    race_condition_handled_with_mutex_demo();

    log("Done.");
}
