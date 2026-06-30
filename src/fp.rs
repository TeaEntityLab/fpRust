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
        $func($($x), *)
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
`Reverse` macro for `Vec<T>`, in currying ways.
*/
#[macro_export]
macro_rules! reverse {
    () => {
        |v| reverse(v)
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
pub fn reverse<T>(v: Vec<T>) -> Vec<T> {
    v.into_iter().rev().collect::<Vec<T>>()
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
    Reduce::reduce(v.into_iter(), f)
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
    assert_eq!(true, contains!(&4)(vec![1, 2, 3, 4]));
}

#[test]
fn test_reverse() {
    assert_eq!(vec![4, 3, 2, 1], reverse!()(vec![1, 2, 3, 4]));
}

#[test]
fn test_fp_fn_map() {
    assert_eq!(vec![2, 4, 6], map(|x| x * 2, vec![1, 2, 3]));
    assert_eq!(
        vec![String::from("1"), String::from("2")],
        map(|x: i32| x.to_string(), vec![1, 2])
    );
    assert_eq!(Vec::<i32>::new(), map(|x: i32| x * 2, Vec::<i32>::new()));
}

#[test]
fn test_fp_fn_filter() {
    assert_eq!(vec![2, 4], filter(|x| *x % 2 == 0, vec![1, 2, 3, 4, 5]));
    assert_eq!(Vec::<i32>::new(), filter(|x| *x > 100, vec![1, 2, 3]));
    assert_eq!(vec![1, 2, 3], filter(|_| true, vec![1, 2, 3]));
}

#[test]
fn test_fp_fn_foldl() {
    assert_eq!(10, foldl(|acc, x| acc + x, 0, vec![1, 2, 3, 4]));
    // foldl is left-associative: (((""+"a")+"b")+"c")
    assert_eq!(
        String::from("abc"),
        foldl(|acc, x| acc + x, String::from(""), vec!["a", "b", "c"])
    );
    assert_eq!(42, foldl(|acc, x| acc + x, 42, Vec::<i32>::new()));
}

#[test]
fn test_fp_fn_foldr() {
    assert_eq!(10, foldr(|acc, x| acc + x, 0, vec![1, 2, 3, 4]));
    // foldr walks in reverse: (((""+"c")+"b")+"a")
    assert_eq!(
        String::from("cba"),
        foldr(|acc, x| acc + x, String::from(""), vec!["a", "b", "c"])
    );
    assert_eq!(42, foldr(|acc, x| acc + x, 42, Vec::<i32>::new()));
}

#[test]
fn test_fp_fn_reduce() {
    assert_eq!(Some(24), reduce(|a, b| a * b, vec![1, 2, 3, 4]));
    assert_eq!(Some(7), reduce(|a, b| a + b, vec![7]));
    assert_eq!(None, reduce(|a, b| a + b, Vec::<i32>::new()));
}

#[test]
fn test_fp_fn_reverse() {
    assert_eq!(vec![3, 2, 1], reverse(vec![1, 2, 3]));
    assert_eq!(vec![1], reverse(vec![1]));
    assert_eq!(Vec::<i32>::new(), reverse(Vec::<i32>::new()));
}

#[test]
fn test_fp_fn_contains() {
    assert_eq!(true, contains(&3, vec![1, 2, 3]));
    assert_eq!(false, contains(&9, vec![1, 2, 3]));
    assert_eq!(false, contains(&1, Vec::<i32>::new()));
}

#[test]
fn test_fp_compose_two() {
    let add_one = |x: i32| x + 1;
    let double = |x: i32| x * 2;
    // compose_two(f, g) == |x| g(f(x)); f runs first.
    let f = compose_two(add_one, double);
    assert_eq!(8, f(3));
}

#[test]
fn test_fp_reduce_trait() {
    // Exercise the Reduce trait directly via fully-qualified syntax
    // (avoids clashing with the std Iterator::reduce method).
    assert_eq!(
        Some(10),
        Reduce::reduce(vec![1, 2, 3, 4].into_iter(), |a, b| a + b)
    );
    assert_eq!(
        None,
        Reduce::reduce(Vec::<i32>::new().into_iter(), |a, b| a + b)
    );
}

#[test]
fn test_fp_compose_single() {
    // A single function passes through unchanged.
    let triple = |x: i32| x * 3;
    assert_eq!(15, (compose!(triple))(5));
    let triple = |x: i32| x * 3;
    assert_eq!(15, (pipe!(triple))(5));
}

#[test]
fn test_fp_compose_pipe_are_mirror() {
    // compose! is right-to-left, pipe! is left-to-right;
    // with the same function list they should be reverse orders.
    let add = |x: i32| x + 2;
    let multiply = |x: i32| x * 3;
    let divide = |x: i32| x / 2;

    // compose: divide -> multiply -> add
    assert_eq!(17, (compose!(add, multiply, divide))(10));
    // pipe: add -> multiply -> divide
    assert_eq!(18, (pipe!(add, multiply, divide))(10));
}

#[test]
fn test_fp_macro_map_filter_curried() {
    // The macros build curried closures (|v| ...).
    let doubler = map!(|x| x * 2);
    assert_eq!(vec![2, 4, 6], doubler(vec![1, 2, 3]));

    let evens = filter!(|x| *x % 2 == 0);
    assert_eq!(vec![2, 4], evens(vec![1, 2, 3, 4]));

    let product = reduce!(|a, b| a * b);
    assert_eq!(Some(24), product(vec![1, 2, 3, 4]));
}

#[test]
fn test_fp_macro_foldl_foldr_curried() {
    let sum_l = foldl!(|a, b| a + b, 0);
    assert_eq!(6, sum_l(vec![1, 2, 3]));

    let sum_r = foldr!(|a, b| a + b, 0);
    assert_eq!(6, sum_r(vec![1, 2, 3]));
}

#[test]
fn test_fp_macro_reverse_contains_curried() {
    let rev = reverse!();
    assert_eq!(vec![3, 2, 1], rev(vec![1, 2, 3]));

    let has_two = contains!(&2);
    assert_eq!(true, has_two(vec![1, 2, 3]));
    let has_two = contains!(&2);
    assert_eq!(false, has_two(vec![3, 4, 5]));
}

#[test]
fn test_fp_macro_empty_inputs() {
    let doubler = map!(|x: i32| x * 2);
    assert_eq!(Vec::<i32>::new(), doubler(Vec::<i32>::new()));

    let product = reduce!(|a, b| a * b);
    assert_eq!(None, product(Vec::<i32>::new()));
}

#[test]
fn test_fp_spread_and_call() {
    // spread_and_call! must expand as an expression and return the value.
    fn add3(a: i32, b: i32, c: i32) -> i32 {
        a + b + c
    }
    let r = spread_and_call!(add3, 1, 2, 3);
    assert_eq!(6, r);
}

#[test]
fn test_fp_partial_left_last_one() {
    // partial_left_last_one! curries the final argument.
    fn sub(a: i32, b: i32) -> i32 {
        a - b
    }
    let from_ten = partial_left_last_one!(sub, 10);
    assert_eq!(7, from_ten(3));
}

#[test]
fn test_fp_map_reduce_filter_pipeline_strings() {
    // Build a longer pipeline over a different element type.
    let result = (compose!(
        reduce!(|a: String, b: String| a + &b),
        map!(|x: i32| x.to_string()),
        filter!(|x: &i32| *x % 2 == 1)
    ))(vec![1, 2, 3, 4, 5]);
    assert_eq!(Some(String::from("135")), result);
}
