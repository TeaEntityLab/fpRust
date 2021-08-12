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
        maybe_func.chain(|f| self.map(f.clone().unwrap()))
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
