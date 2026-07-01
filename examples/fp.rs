#![allow(unused_imports)]

use fp_rust::fp::{compose_two, filter, map, reduce};
use fp_rust::{compose, partial_left_last_one, pipe, spread_and_call};

fn main() {
    let add = |x| x + 2;
    let multiply = |x| x * 3;
    let divide = |x| x / 2;

    let result = (compose!(add, multiply, divide))(10);
    assert_eq!(17, result);
    println!("Composed FnOnce Result is {}", result);

    let result = (pipe!(add, multiply, divide))(10);
    assert_eq!(18, result);
    println!("Piped FnOnce Result is {}", result);

    let result = (compose!(
        fp_rust::reduce!(|a, b| a * b),
        fp_rust::filter!(|x| *x < 6),
        fp_rust::map!(|x| x * 2)
    ))(vec![1, 2, 3, 4]);
    assert_eq!(Some(8), result);
    println!("test_map_reduce_filter Result is {:?}", result);
}
