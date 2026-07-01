//! Functional programming helpers: function composition, curried macros,
//! and `Vec` utilities (`map`, `filter`, `fold`, `reduce`, and friends).

/// Pipe functions left-to-right: `pipe!(f, g)(x)` applies `f` then `g`.
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::compose_two, pipe};
///
/// let add = |x: i32| x + 2;
/// let multiply = |x: i32| x * 3;
/// assert_eq!(36, pipe!(add, multiply)(10));
/// ```
///
#[macro_export]
macro_rules! pipe {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        {
            let head_fn = $head;
            let tail_fn = pipe!($($tail),+);
            move |x| tail_fn(head_fn(x))
        }
    };
}

/// Compose functions right-to-left: `compose!(f, g)(x)` applies `g` then `f`.
///
/// # Examples
///
/// ```
/// use fp_rust::{compose, fp::compose_two};
///
/// let add = |x: i32| x + 2;
/// let multiply = |x: i32| x * 3;
/// assert_eq!(32, compose!(add, multiply)(10));
/// ```
///
#[macro_export]
macro_rules! compose {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        {
            let tail_fn = compose!($($tail),+);
            let head_fn = $head;
            move |x| head_fn(tail_fn(x))
        }
    };
}

/// Spreads variadic arguments into a function call.
///
/// # Examples
///
/// ```
/// use fp_rust::spread_and_call;
///
/// fn add3(a: i32, b: i32, c: i32) -> i32 { a + b + c }
/// assert_eq!(6, spread_and_call!(add3, 1, 2, 3));
/// ```
#[macro_export]
macro_rules! spread_and_call {
    ($func:expr, $($x:expr), *) => {
        $func($($x), *)
    };
}

/// Partially applies a function, currying the last argument into a closure.
///
/// # Examples
///
/// ```
/// use fp_rust::{partial_left_last_one, spread_and_call};
///
/// fn sub(a: i32, b: i32) -> i32 { a - b }
/// let from_ten = partial_left_last_one!(sub, 10);
/// assert_eq!(7, from_ten(3));
/// ```
#[macro_export]
macro_rules! partial_left_last_one {
    ($func:expr, $($x:expr), *) => {
        |v| spread_and_call!($func, $($x), *, v)
    };
}

/// Curried `map` for `Vec<T>` (built on [`partial_left_last_one`]).
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::map, map, partial_left_last_one, spread_and_call};
///
/// let doubler = map!(|x| x * 2);
/// assert_eq!(vec![2, 4, 6], doubler(vec![1, 2, 3]));
/// ```
#[macro_export]
macro_rules! map {
    ($func:expr) => {
        partial_left_last_one!(map, $func)
    };
}

/// Curried `filter` for `Vec<T>` (built on [`partial_left_last_one`]).
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::filter, filter, partial_left_last_one, spread_and_call};
///
/// let evens = filter!(|x| *x % 2 == 0);
/// assert_eq!(vec![2, 4], evens(vec![1, 2, 3, 4]));
/// ```
#[macro_export]
macro_rules! filter {
    ($func:expr) => {
        partial_left_last_one!(filter, $func)
    };
}

/// Curried `reduce` for `Vec<T>` (built on [`partial_left_last_one`]).
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::reduce, partial_left_last_one, reduce, spread_and_call};
///
/// let product = reduce!(|a, b| a * b);
/// assert_eq!(Some(24), product(vec![1, 2, 3, 4]));
/// ```
#[macro_export]
macro_rules! reduce {
    ($func:expr) => {
        partial_left_last_one!(reduce, $func)
    };
}

/// Curried left fold for `Vec<T>` (built on [`partial_left_last_one`]).
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::foldl, foldl, partial_left_last_one, spread_and_call};
///
/// let sum = foldl!(|acc, x| acc + x, 0);
/// assert_eq!(6, sum(vec![1, 2, 3]));
/// ```
#[macro_export]
macro_rules! foldl {
    ($func:expr, $second:expr) => {
        partial_left_last_one!(foldl, $func, $second)
    };
}

/// Curried right fold for `Vec<T>` (built on [`partial_left_last_one`]).
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::foldr, foldr, partial_left_last_one, spread_and_call};
///
/// let sum = foldr!(|acc, x| acc + x, 0);
/// assert_eq!(6, sum(vec![1, 2, 3]));
/// ```
#[macro_export]
macro_rules! foldr {
    ($func:expr, $second:expr) => {
        partial_left_last_one!(foldr, $func, $second)
    };
}

/// Curried `reverse` for `Vec<T>`.
///
/// # Examples
///
/// ```
/// use fp_rust::{fp::reverse, reverse};
///
/// let rev = reverse!();
/// assert_eq!(vec![3, 2, 1], rev(vec![1, 2, 3]));
/// ```
#[macro_export]
macro_rules! reverse {
    () => {
        |v| reverse(v)
    };
}

/// Curried `contains` for `Vec<T>` (built on [`partial_left_last_one`]).
///
/// # Examples
///
/// ```
/// use fp_rust::{contains, fp::contains, partial_left_last_one, spread_and_call};
///
/// let has_two = contains!(&2);
/// assert_eq!(true, has_two(vec![1, 2, 3]));
/// ```
#[macro_export]
macro_rules! contains {
    ($x:expr) => {
        partial_left_last_one!(contains, $x)
    };
}

/// Composes two functions: the returned closure applies `f` then `g`.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::compose_two;
///
/// let add_one = |x: i32| x + 1;
/// let double = |x: i32| x * 2;
/// assert_eq!(8, compose_two(add_one, double)(3));
/// ```
///
#[inline]
pub fn compose_two<A, B, C, G, F>(f: F, g: G) -> impl FnOnce(A) -> C
where
    F: FnOnce(A) -> B,
    G: FnOnce(B) -> C,
{
    move |input| {
        let mid = f(input);
        g(mid)
    }
}

/// Maps `f` over every element of `v`.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::map;
///
/// assert_eq!(vec![2, 4, 6], map(|x| x * 2, vec![1, 2, 3]));
/// ```
#[inline]
pub fn map<T, B>(f: impl FnMut(T) -> B, v: Vec<T>) -> Vec<B> {
    v.into_iter().map(f).collect::<Vec<B>>()
}

/// Keeps elements of `v` for which `f` returns `true`.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::filter;
///
/// assert_eq!(vec![2, 4], filter(|x| *x % 2 == 0, vec![1, 2, 3, 4, 5]));
/// ```
#[inline]
pub fn filter<'r, T: 'r>(f: impl FnMut(&T) -> bool, v: Vec<T>) -> Vec<T> {
    v.into_iter().filter(f).collect::<Vec<T>>()
}

/// Left-associative fold over `v`.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::foldl;
///
/// assert_eq!(10, foldl(|acc, x| acc + x, 0, vec![1, 2, 3, 4]));
/// ```
#[inline]
pub fn foldl<T, B>(f: impl FnMut(B, T) -> B, initial: B, v: Vec<T>) -> B {
    v.into_iter().fold(initial, f)
}

/// Right-associative fold over `v` (iterates from the end).
///
/// # Examples
///
/// ```
/// use fp_rust::fp::foldr;
///
/// assert_eq!(10, foldr(|acc, x| acc + x, 0, vec![1, 2, 3, 4]));
/// ```
#[inline]
pub fn foldr<T, B>(f: impl FnMut(B, T) -> B, initial: B, v: Vec<T>) -> B {
    v.into_iter().rev().fold(initial, f)
}

/// Reverses `v`.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::reverse;
///
/// assert_eq!(vec![3, 2, 1], reverse(vec![1, 2, 3]));
/// ```
#[inline]
pub fn reverse<T>(v: Vec<T>) -> Vec<T> {
    v.into_iter().rev().collect::<Vec<T>>()
}

/// Returns whether `v` contains `x`.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::contains;
///
/// assert_eq!(true, contains(&3, vec![1, 2, 3]));
/// assert_eq!(false, contains(&9, vec![1, 2, 3]));
/// ```
#[inline]
pub fn contains<T: PartialEq>(x: &T, v: Vec<T>) -> bool {
    v.contains(x)
}

/// ECMAScript-like `reduce` for iterators.
///
/// *NOTE*: Credit <https://github.com/dtolnay/reduce>.
pub trait Reduce<T> {
    /// Folds adjacent pairs with `f`, returning `None` on an empty iterator.
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

/// Reduces `v` with `f`, or returns `None` when `v` is empty.
///
/// # Examples
///
/// ```
/// use fp_rust::fp::reduce;
///
/// assert_eq!(Some(24), reduce(|a, b| a * b, vec![1, 2, 3, 4]));
/// assert_eq!(None, reduce(|a, b| a + b, Vec::<i32>::new()));
/// ```
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
                a
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
                a
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
