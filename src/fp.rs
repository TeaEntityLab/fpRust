/*!
In this module there're implementations & tests
of general functional programming features.
*/
#[macro_export]

/**
Compose functions.

*NOTE*: Credit https://stackoverflow.com/questions/45786955/how-to-compose-functions-in-rust
*/
macro_rules! compose {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        compose_two($head, compose!($($tail),+))
    };
}

/**
Compose two functions into one.
Return `f(g(x))`

# Arguments

* `f` - The given `FnOnce`.
* `g` - The given `FnOnce`.

*NOTE*: Credit https://stackoverflow.com/questions/45786955/how-to-compose-functions-in-rust
*/
pub fn compose_two<A, B, C, G, F>(f: F, g: G) -> impl FnOnce(A) -> C
where
    F: FnOnce(A) -> B,
    G: FnOnce(B) -> C,
{
    move |x| g(f(x))
}

#[test]
fn test_compose() {
    let add = |x| x + 2;
    let multiply = |x| x * 2;
    let divide = |x| x / 2;

    let result = (compose!(add, multiply, divide))(10);
    assert_eq!(12, result);
    println!("Composed FnOnce Result is {}", result);
}
