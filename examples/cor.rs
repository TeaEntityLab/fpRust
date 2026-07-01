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
        let cor1 = cor1.clone();
        cor1.lock().unwrap().set_async(true);
    }
    {
        let cor2 = cor2.clone();
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
