/*!
In this module there're implementations & tests
of general functional programming features.
*/

/**
Compose functions.

*NOTE*: Credit https://stackoverflow.com/questions/45786955/how-to-compose-functions-in-rust
*/
#[macro_export]
macro_rules! pipe {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        compose_two($head, pipe!($($tail),+))
    };
}
#[macro_export]
macro_rules! compose {
    ( $last:expr ) => { $last };
    ( $head:expr, $($tail:expr), +) => {
        compose_two(compose!($($tail),+), $head)
    };
}
#[macro_export]
macro_rules! partial_vec {
    ($func:expr, $second:expr) => {
        |v| $func($second, v)
    };
}
#[macro_export]
macro_rules! map {
    ($func:expr) => {
        partial_vec!(map, $func)
    };
}
#[macro_export]
macro_rules! filter {
    ($func:expr) => {
        partial_vec!(filter, $func)
    };
}
#[macro_export]
macro_rules! reduce {
    ($func:expr) => {
        partial_vec!(reduce, $func)
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

pub fn map<T, B>(f: impl FnMut(T) -> B, v: Vec<T>) -> Vec<B> {
    v.into_iter().map(f).collect::<Vec<B>>()
}

pub fn filter<'r, T: 'r>(f: impl FnMut(&T) -> bool, v: Vec<T>) -> Vec<T> {
    v.into_iter().filter(f).into_iter().collect::<Vec<T>>()
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
    let result = (compose!(reduce!(|a, b| a * b), filter!(|x| *x < 6), map!(|x| x * 2)))(vec![1, 2, 3, 4]);
    assert_eq!(Some(8), result);
    println!("test_map_reduce_filter Result is {:?}", result);
}
