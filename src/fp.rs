/*!
In this module there're implementations & tests
of general functional programming features.
*/

/**
Pipe functions.

*NOTE*: Credit https://stackoverflow.com/questions/45786955/how-to-compose-functions-in-rust
*/
#[macro_export]
macro_rules! pipe {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        compose_two($head, pipe!($($tail),+))
    };
}

/**
Compose functions.

*NOTE*: Credit https://stackoverflow.com/questions/45786955/how-to-compose-functions-in-rust
*/
#[macro_export]
macro_rules! compose {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        compose_two(compose!($($tail),+), $head)
    };
}

/**
`Spread` the variadic arguments and call the given funciton.
*/
#[macro_export]
macro_rules! spread_and_call {
    ($func:expr, $($x:expr), *) => {
        $func($($x), *);
    };
}

/**
`Partial` application macro with variadic arguments for a pre-defined function,
and currying the lastest one argument by returning closure.
*/
#[macro_export]
macro_rules! partial_left_last_one {
    ($func:expr, $($x:expr), *) => {
        |v| spread_and_call!($func, $($x), *, v)
    };
}

/**
`Map` macro for `Vec<T>`, in currying ways by `partial_left_last_one!`().
*/
#[macro_export]
macro_rules! map {
    ($func:expr) => {
        partial_left_last_one!(map, $func)
    };
}

/**
`Filter` macro for `Vec<T>`, in currying ways by `partial_left_last_one!`().
*/
#[macro_export]
macro_rules! filter {
    ($func:expr) => {
        partial_left_last_one!(filter, $func)
    };
}

/**
`Reduce` macro for `Vec<T>`, in currying ways by `partial_left_last_one!`().
*/
#[macro_export]
macro_rules! reduce {
    ($func:expr) => {
        partial_left_last_one!(reduce, $func)
    };
}

/**
`Foldl` macro for `Vec<T>`, in currying ways by `partial_left_last_one!`().
*/
#[macro_export]
macro_rules! foldl {
    ($func:expr, $second:expr) => {
        partial_left_last_one!(foldl, $func, $second)
    };
}

/**
`Foldr` macro for `Vec<T>`, in currying ways by `partial_left_last_one!`().
*/
#[macro_export]
macro_rules! foldr {
    ($func:expr, $second:expr) => {
        partial_left_last_one!(foldr, $func, $second)
    };
}

/**
`Contains` macro for `Vec<T>`, in currying ways by `partial_left_last_one!`().
*/
#[macro_export]
macro_rules! contains {
    ($x:expr) => {
        partial_left_last_one!(contains, $x)
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
#[inline]
pub fn compose_two<A, B, C, G, F>(f: F, g: G) -> impl FnOnce(A) -> C
where
    F: FnOnce(A) -> B,
    G: FnOnce(B) -> C,
{
    move |x| g(f(x))
}

#[inline]
pub fn map<T, B>(f: impl FnMut(T) -> B, v: Vec<T>) -> Vec<B> {
    v.into_iter().map(f).collect::<Vec<B>>()
}

#[inline]
pub fn filter<'r, T: 'r>(f: impl FnMut(&T) -> bool, v: Vec<T>) -> Vec<T> {
    v.into_iter().filter(f).into_iter().collect::<Vec<T>>()
}

#[inline]
pub fn foldl<T, B>(f: impl FnMut(B, T) -> B, initial: B, v: Vec<T>) -> B {
    v.into_iter().fold(initial, f)
}

#[inline]
pub fn foldr<T, B>(f: impl FnMut(B, T) -> B, initial: B, v: Vec<T>) -> B {
    v.into_iter().rev().fold(initial, f)
}

#[inline]
pub fn contains<T: PartialEq>(x: &T, v: Vec<T>) -> bool {
    v.contains(x)
}

/**
Implementations of `ECMASript`-like `reduce`()

# Arguments

* `T` - The generic type of data.

*NOTE*: Credit https://github.com/dtolnay/reduce
*/
pub trait Reduce<T> {
    fn reduce<F>(self, f: F) -> Option<T>
    where
        Self: Sized,
        F: FnMut(T, T) -> T;
}

impl<T, I> Reduce<T> for I
where
    I: Iterator<Item = T>,
{
    #[inline]
    fn reduce<F>(mut self, f: F) -> Option<T>
    where
        Self: Sized,
        F: FnMut(T, T) -> T,
    {
        self.next().map(|first| self.fold(first, f))
    }
}

#[inline]
pub fn reduce<'r, T: 'r>(f: impl FnMut(T, T) -> T, v: Vec<T>) -> Option<T> {
    v.into_iter().reduce(f)
}

#[test]
fn test_compose() {
    let add = |x| x + 2;
    let multiply = |x| x * 3;
    let divide = |x| x / 2;

    let result = (compose!(add, multiply, divide))(10);
    assert_eq!(17, result);
    println!("Composed FnOnce Result is {}", result);

    let result = (pipe!(add, multiply, divide))(10);
    assert_eq!(18, result);
    println!("Piped FnOnce Result is {}", result);
}

#[test]
fn test_map_reduce_filter() {
    let result =
        (compose!(reduce!(|a, b| a * b), filter!(|x| *x < 6), map!(|x| x * 2)))(vec![1, 2, 3, 4]);
    assert_eq!(Some(8), result);
    println!("test_map_reduce_filter Result is {:?}", result);
}

#[test]
fn test_foldl_foldr() {
    // foldl!(f, initial)
    let result = (compose!(
        foldl!(
            |a, b| {
                if a < 4 {
                    return a + b;
                }
                return a;
            },
            0
        ),
        filter!(|x| *x < 6),
        map!(|x| x * 2)
    ))(vec![1, 2, 3, 4]);
    assert_eq!(6, result);
    println!("foldl Result is {:?}", result);

    // foldr!(f, initial)
    let result = (compose!(
        foldr!(
            |a, b| {
                if a < 4 {
                    return a + b;
                }
                return a;
            },
            0
        ),
        filter!(|x| *x < 6),
        map!(|x| x * 2)
    ))(vec![1, 2, 3, 4]);
    assert_eq!(4, result);
    println!("foldr Result is {:?}", result);
}

#[test]
fn test_contains() {
    assert_eq!(true, contains!(&4)(vec!(1, 2, 3, 4)));
}
