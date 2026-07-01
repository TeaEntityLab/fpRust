// Cor coroutines: `yield` / `yield_from` with the sync/async deadlock invariant.
//
// INVARIANT (see src/cor.rs): a `yield_from` TARGET must be async. Two
// coroutines that yield to each other while BOTH sync deadlock — each blocks
// its thread waiting for the other. The safe split, shown below:
//   - the yield TARGET (cor1) is async  -> set_async(true)
//   - the ENTRY driver (cor2) stays sync -> set_async(false)
//
// Run: cargo run --example cor --features=test_runtime

use std::thread;
use std::time::{Duration, Instant};

use fp_rust::cor::Cor;
use fp_rust::{cor_newmutex, cor_start, cor_yield, cor_yield_from};

fn main() {
    println!("test_cor_new");

    let cor1 = cor_newmutex!(
        |this| {
            println!("cor1 started");

            let s = cor_yield!(this, Some(String::from("given_to_outside")));
            println!("cor1 {:?}", s);
        },
        String,
        i16
    );
    let cor1_for_yield = cor1.clone();

    let cor2 = cor_newmutex!(
        move |this| {
            println!("cor2 started");

            println!("cor2 yield_from before");

            let s = cor_yield_from!(this, cor1_for_yield, Some(3));
            println!("cor2 {:?}", s);
        },
        i16,
        i16
    );

    {
        // Target of yield_from: MUST be async, or the pair deadlocks.
        cor1.lock().unwrap().set_async(true);
    }
    {
        // Entry driver: sync is fine because its target (cor1) is async.
        cor2.lock().unwrap().set_async(false);
    }
    cor_start!(cor1);
    cor_start!(cor2);

    let deadline = Instant::now() + Duration::from_secs(5);
    while cor2.lock().unwrap().is_alive() {
        if Instant::now() >= deadline {
            panic!("cor2 did not finish within deadline");
        }
        thread::yield_now();
    }
}
