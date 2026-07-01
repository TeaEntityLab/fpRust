use std::sync::{Arc, Mutex};

use fp_rust::cor::Cor;
use fp_rust::{cor_newmutex, cor_newmutex_and_start, cor_start, cor_yield, cor_yield_from, do_m};

fn main() {
    let v = Arc::new(Mutex::new(String::from("")));

    let v_capture = v.clone();
    do_m!(move |this| {
        println!("test_cor_do_m started");

        let cor_inner1 = cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("1")));
                println!("cor_inner1 {:?}", s);
            },
            String,
            i16
        );
        let cor_inner2 = cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("2")));
                println!("cor_inner2 {:?}", s);
            },
            String,
            i16
        );
        let cor_inner3 = cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("3")));
                println!("cor_inner3 {:?}", s);
            },
            String,
            i16
        );

        (*v_capture.lock().unwrap()) = [
            cor_yield_from!(this, cor_inner1, Some(1)).unwrap(),
            cor_yield_from!(this, cor_inner2, Some(2)).unwrap(),
            cor_yield_from!(this, cor_inner3, Some(3)).unwrap(),
        ]
        .join("");
    });

    assert_eq!("123", *v.lock().unwrap());
}
