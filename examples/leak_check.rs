//! Memory-leak check.
//!
//! A counting global allocator tracks **live bytes** (allocated − freed) — RSS
//! is too noisy (the OS/allocator retains freed pages), but live bytes are
//! exact, so a real leak shows up as monotonic growth. Two phases:
//!
//!   1. **point codes** — parse (structured + decimal) and format point codes
//!      for many cycles (the `split` + integer parse + `Display` path).
//!   2. **MSU** — construct and clone an `Mtp3Msu` (the SAP-boundary struct,
//!      including its `Vec<u8>` payload) over and over.
//!
//! Each phase asserts live bytes return to a flat baseline. Exits non-zero on a
//! leak. Driven by `scripts/mem_leak_test.sh`.
//!
//! Run: `cargo run --release --example leak_check`

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, Ordering};

use mtp3::{Mtp3Msu, NetworkIndicator, PointCode, ServiceIndicator, Variant};

// ── Counting allocator ──────────────────────────────────────────────────────
static LIVE: AtomicI64 = AtomicI64::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        LIVE.fetch_sub(l.size() as i64, Ordering::Relaxed);
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        }
        p
    }
    unsafe fn realloc(&self, ptr: *mut u8, l: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, l, new_size);
        if !p.is_null() {
            LIVE.fetch_add(new_size as i64 - l.size() as i64, Ordering::Relaxed);
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> i64 {
    LIVE.load(Ordering::Relaxed)
}

// ── Phase 1: point-code parse/format churn ──────────────────────────────────
fn pointcode_cycle(iters: usize) {
    for _ in 0..iters {
        // Structured + decimal parse for both variants, then render back.
        let itu = PointCode::parse("2-1-3", Variant::Itu).unwrap();
        std::hint::black_box(itu.to_string());
        let ansi = PointCode::parse("1-1-5", Variant::Ansi).unwrap();
        std::hint::black_box(ansi.to_string());
        let dec = PointCode::parse("5687", Variant::Itu).unwrap();
        std::hint::black_box(dec.components());
    }
}

// ── Phase 2: MSU construct/clone churn ──────────────────────────────────────
fn msu_cycle(iters: usize) {
    let opc = PointCode::from_value(1, Variant::Itu).unwrap();
    let dpc = PointCode::from_value(2, Variant::Itu).unwrap();
    for _ in 0..iters {
        let msu = Mtp3Msu {
            si: ServiceIndicator::SCCP,
            ni: NetworkIndicator::International,
            mp: 0,
            opc,
            dpc,
            sls: 5,
            data: vec![0x09, 0x80, 0x03, 0x0A, 0x0B, 0x0C],
        };
        let cloned = msu.clone();
        std::hint::black_box(cloned.data.len());
        std::hint::black_box(msu);
    }
}

fn report(phase: &str, base: i64) -> i64 {
    let growth = live() - base;
    println!("  {phase}: live = {} bytes (Δ {:+})", live(), growth);
    growth
}

fn main() {
    const ITERS: usize = 200_000;
    const CYCLES: usize = 10;
    const BUDGET: i64 = 64 * 1024;

    // Phase 1: point codes.
    println!(
        "[point codes] {CYCLES} x {ITERS} parse+format cycles (ITU + ANSI, structured + decimal)"
    );
    pointcode_cycle(ITERS); // warm up
    let pc_base = live();
    for c in 1..=CYCLES {
        pointcode_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), pc_base);
    }
    let pc_growth = live() - pc_base;

    // Phase 2: MSU.
    println!("\n[msu] {CYCLES} x {ITERS} construct + clone");
    msu_cycle(ITERS); // warm up
    let msu_base = live();
    for c in 1..=CYCLES {
        msu_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), msu_base);
    }
    let msu_growth = live() - msu_base;

    // Verdict.
    println!();
    let mut ok = true;
    if pc_growth > BUDGET {
        eprintln!("FAIL: point-code live bytes grew {pc_growth} (> {BUDGET})");
        ok = false;
    }
    if msu_growth > BUDGET {
        eprintln!("FAIL: MSU live bytes grew {msu_growth} (> {BUDGET})");
        ok = false;
    }
    if !ok {
        std::process::exit(1);
    }
    println!("PASS: point-code Δ {pc_growth} ≤ {BUDGET}; MSU Δ {msu_growth} ≤ {BUDGET}");
}
