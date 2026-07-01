/*!
In this module there're implementations & tests of `Maybe`.
*/

use std::sync::Arc;

/**
`Maybe` wraps built-in `Option<T>`,
and implements `Applicative` and `Chain` of `fantasy-land`.

# Arguments

* `T` - The generic type of data

# Remarks

It implements `Applicative` and `Chain` of `fantasy-land`,
and use the same interface as `fpEs` & `fpGo`(sister libraries :P)

``
*/
#[derive(Clone)]
pub struct Maybe<T> {
    r: Arc<Option<T>>,
}

impl<T: Clone + 'static> Maybe<T> {
    pub fn option(&self) -> Option<T> {
        self.r.as_ref().clone()
    }
    pub fn unwrap(&self) -> T {
        self.r.as_ref().clone().unwrap()
    }
    pub fn or(&self, val: T) -> T {
        self.r.as_ref().clone().unwrap_or(val)
    }
}

impl<T: 'static> From<T> for Maybe<T> {
    fn from(r: T) -> Self {
        Maybe::just(Some(r))
    }
}

impl<T: 'static> Maybe<T> {
    pub fn just(r: Option<T>) -> Maybe<T> {
        Maybe { r: Arc::new(r) }
    }
    pub fn of(r: Option<T>) -> Maybe<T> {
        Maybe::just(r)
    }
    pub fn val(r: T) -> Maybe<T> {
        Maybe::just(Some(r))
    }

    pub fn present(&self) -> bool {
        match self.r.as_ref() {
            Some(_x) => true,
            None => false,
        }
    }
    pub fn null(&self) -> bool {
        match self.r.as_ref() {
            Some(_x) => false,
            None => true,
        }
    }
    pub fn let_do<F>(&self, func: F)
    where
        F: FnOnce(&T),
    {
        match self.r.as_ref() {
            Some(_x) => func(&_x),
            None => (),
        }
    }

    pub fn fmap<F, G>(&self, func: F) -> Maybe<G>
    where
        F: FnOnce(&Option<T>) -> Maybe<G>,
    {
        func(self.r.as_ref())
    }
    pub fn map<F, G>(&self, func: F) -> Maybe<G>
    where
        F: FnOnce(&Option<T>) -> Option<G>,
        G: 'static,
    {
        Maybe::just(func(self.r.as_ref()))
    }
    pub fn bind<F, G>(&self, func: F) -> Maybe<G>
    where
        F: FnOnce(&Option<T>) -> Option<G>,
        G: 'static,
    {
        self.map(func)
    }
    pub fn then<F, G>(&self, func: F) -> Maybe<G>
    where
        F: FnOnce(&Option<T>) -> Option<G>,
        G: 'static,
    {
        self.map(func)
    }
    pub fn chain<F, G>(&self, func: F) -> Maybe<G>
    where
        F: FnOnce(&Option<T>) -> Maybe<G>,
    {
        self.fmap(func)
    }
    pub fn ap<F, G>(&self, maybe_func: &Maybe<F>) -> Maybe<G>
    where
        F: FnOnce(&Option<T>) -> Option<G> + Clone + 'static,
        G: 'static,
    {
        maybe_func.chain(|f| match f {
            Some(func) => self.map(func.clone()),
            None => Maybe::just(None::<G>),
        })
    }
}

#[test]
fn test_maybe_present() {
    assert_eq!(false, Maybe::just(None::<bool>).present());
    assert_eq!(true, Maybe::val(true).present());

    assert_eq!(true, Maybe::just(None::<bool>).null());
    assert_eq!(false, Maybe::val(true).null());

    let mut val;

    val = false;
    Maybe::just(None::<bool>).let_do(|x| val = *x);
    assert_eq!(false, val);

    val = false;
    Maybe::val(true).let_do(|x| val = *x);
    assert_eq!(true, val);
}
#[test]
fn test_maybe_flatmap() {
    assert_eq!(
        false,
        Maybe::val(true)
            .fmap(|x| return Maybe::val(!x.unwrap()))
            .unwrap()
    );
    assert_eq!(
        true,
        Maybe::val(false)
            .fmap(|x| return Maybe::val(!x.unwrap()))
            .unwrap()
    );

    assert_eq!(
        false,
        Maybe::val(true).map(|x| return Some(!x.unwrap())).unwrap()
    );
    assert_eq!(
        true,
        Maybe::val(false).map(|x| return Some(!x.unwrap())).unwrap()
    );

    assert_eq!(
        true,
        Maybe::val(1)
            .ap(&Maybe::val(|x: &Option<i16>| if x.unwrap() > 0 {
                return Some(true);
            } else {
                return Some(false);
            }))
            .unwrap()
    );
}
#[test]
fn test_maybe_unwrap() {
    assert_eq!(false, Maybe::just(None::<bool>).or(false));
    assert_eq!(true, Maybe::val(true).or(false));
    use std::panic;

    let none_unwrap = panic::catch_unwind(|| {
        Maybe::just(None::<bool>).unwrap();
    });
    assert_eq!(true, none_unwrap.is_err());
    assert_eq!(true, Maybe::val(true).unwrap());

    assert_eq!(
        true,
        match Maybe::val(true).option() {
            None => false,
            Some(_x) => true,
        }
    );
    assert_eq!(
        false,
        match Maybe::just(None::<bool>).option() {
            None => false,
            Some(_x) => true,
        }
    );
}

#[test]
fn test_maybe_constructors() {
    // just / of / val all wrap the same way.
    assert_eq!(Some(5), Maybe::just(Some(5)).option());
    assert_eq!(Some(5), Maybe::of(Some(5)).option());
    assert_eq!(Some(5), Maybe::val(5).option());
    assert_eq!(None, Maybe::just(None::<i32>).option());
    // of is an alias of just.
    assert_eq!(
        Maybe::just(Some(7)).option(),
        Maybe::of(Some(7)).option()
    );
}

#[test]
fn test_maybe_from_trait() {
    // From<T> wraps as Some(T).
    let m = Maybe::from(42);
    assert_eq!(true, m.present());
    assert_eq!(42, m.unwrap());

    // .into() routes through From as well.
    let m2: Maybe<i32> = 99.into();
    assert_eq!(Some(99), m2.option());
}

#[test]
fn test_maybe_option_clones_inner() {
    let m = Maybe::val(String::from("hello"));
    assert_eq!(Some(String::from("hello")), m.option());
    // option() returns an owned clone; the Maybe is still usable.
    assert_eq!(Some(String::from("hello")), m.option());
}

#[test]
fn test_maybe_or_unwrap_or_semantics() {
    // or() falls back on None, keeps value on Some.
    assert_eq!(10, Maybe::just(None::<i32>).or(10));
    assert_eq!(3, Maybe::val(3).or(10));
}

#[test]
fn test_maybe_let_do_short_circuits_on_none() {
    use std::cell::Cell;

    let called = Cell::new(0);
    Maybe::just(None::<i32>).let_do(|_| called.set(called.get() + 1));
    assert_eq!(0, called.get());

    Maybe::val(7).let_do(|x| called.set(called.get() + *x));
    assert_eq!(7, called.get());
}

#[test]
fn test_maybe_map_bind_then_equivalent() {
    // map / bind / then share the same implementation; results must match.
    let mapped = Maybe::val(4).map(|x| Some(x.unwrap() * 2));
    let bound = Maybe::val(4).bind(|x| Some(x.unwrap() * 2));
    let thened = Maybe::val(4).then(|x| Some(x.unwrap() * 2));
    assert_eq!(Some(8), mapped.option());
    assert_eq!(Some(8), bound.option());
    assert_eq!(Some(8), thened.option());
}

#[test]
fn test_maybe_map_changes_type() {
    // i32 -> String through map.
    let m = Maybe::val(123).map(|x| Some(x.unwrap().to_string()));
    assert_eq!(Some(String::from("123")), m.option());
}

#[test]
fn test_maybe_map_can_produce_none() {
    let m = Maybe::val(5).map(|x| {
        if x.unwrap() > 10 {
            Some(x.unwrap())
        } else {
            None
        }
    });
    assert_eq!(true, m.null());
}

#[test]
fn test_maybe_chain_equals_fmap() {
    // chain delegates to fmap; both receive the inner Option directly.
    let via_fmap = Maybe::val(2).fmap(|x| Maybe::val(x.unwrap() + 1));
    let via_chain = Maybe::val(2).chain(|x| Maybe::val(x.unwrap() + 1));
    assert_eq!(via_fmap.option(), via_chain.option());
    assert_eq!(Some(3), via_chain.option());
}

#[test]
fn test_maybe_fmap_observes_none() {
    // fmap gets &Option<T>; it can branch on None and still return a Maybe.
    let m = Maybe::just(None::<i32>).fmap(|x| match x {
        Some(v) => Maybe::val(*v),
        None => Maybe::val(-1),
    });
    assert_eq!(Some(-1), m.option());
}

#[test]
fn test_maybe_ap_applies_wrapped_fn() {
    // ap applies a wrapped function to the wrapped value.
    let value = Maybe::val(5);
    let wrapped_fn = Maybe::val(|x: &Option<i32>| Some(x.unwrap() * 10));
    let result = value.ap(&wrapped_fn);
    assert_eq!(Some(50), result.option());
}

#[test]
fn test_maybe_ap_none_function_yields_none() {
    // Applicative law: applying an absent function (Nothing) must yield
    // Nothing, NOT panic. `ap` internally unwrapped the function option,
    // so a None function crashed instead of short-circuiting to None.
    let value = Maybe::val(5);
    let no_fn: Maybe<fn(&Option<i32>) -> Option<i32>> = Maybe::just(None);
    let result: Maybe<i32> = value.ap(&no_fn);
    assert_eq!(true, result.null());
    assert_eq!(None, result.option());
}

#[test]
fn test_maybe_present_null_are_opposite() {
    let some = Maybe::val(1);
    assert_eq!(true, some.present());
    assert_eq!(false, some.null());

    let none = Maybe::just(None::<i32>);
    assert_eq!(false, none.present());
    assert_eq!(true, none.null());
}

#[test]
fn test_maybe_chained_pipeline() {
    // A multi-step transform pipeline.
    let result = Maybe::val(3)
        .map(|x| Some(x.unwrap() + 1)) // 4
        .fmap(|x| Maybe::val(x.unwrap() * 2)) // 8
        .map(|x| Some(x.unwrap().to_string())); // "8"
    assert_eq!(Some(String::from("8")), result.option());
}
