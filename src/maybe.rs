use std::panic;

struct Maybe<T> {
    r : Option<T>
}

impl <T> Maybe<T> {
    fn just(r: Option<T>) -> Maybe<T> {
        return Maybe {
            r: r,
        };
    }
    fn val(r: T) -> Maybe<T> {
        return Maybe::just(Some(r));
    }

    fn present(self) -> bool {
        match self.r {
            Some(_x) => return true,
            None => return false,
        }
    }
    fn null(self) -> bool {
        match self.r {
            Some(_x) => return false,
            None => return true,
        }
    }
    fn let_do<F>(self, mut func: F) where F : FnMut (T) {
        match self.r {
            Some(mut _x) => func(_x),
            None => (),
        }
    }

    fn fmap(self, func: fn (Option<T>) -> Maybe<T> ) -> Maybe<T> {
        return func(self.r);
    }
    fn map(self, func: fn (Option<T>) -> Option<T> ) -> Maybe<T> {
        return Maybe::just(func(self.r));
    }

    fn option(self) -> Option<T> {
        return self.r;
    }
    fn unwrap(self) -> T {
        return self.r.unwrap();
    }
    fn or(self, val: T) -> T {
        return self.r.unwrap_or(val);
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
    Maybe::just(None::<bool>).let_do(|x| {val = x});
    assert_eq!(false, val);

    val = false;
    Maybe::val(true).let_do(|x| {val = x});
    assert_eq!(true, val);
}
#[test]
fn test_maybe_flatmap() {
    assert_eq!(false, Maybe::val(true).fmap(|x| {return Maybe::val(!x.unwrap())}).unwrap());
    assert_eq!(true, Maybe::val(false).fmap(|x| {return Maybe::val(!x.unwrap())}).unwrap());

    assert_eq!(false, Maybe::val(true).map(|x| {return Some(!x.unwrap())}).unwrap());
    assert_eq!(true, Maybe::val(false).map(|x| {return Some(!x.unwrap())}).unwrap());
}
#[test]
fn test_maybe_unwrap() {

    assert_eq!(false, Maybe::just(None::<bool>).or(false));
    assert_eq!(true, Maybe::val(true).or(false));

    let none_unwrap = panic::catch_unwind(|| {
        Maybe::just(None::<bool>).unwrap();
    });
    assert_eq!(true, none_unwrap.is_err());
    assert_eq!(true, Maybe::val(true).unwrap());

    assert_eq!(true, match Maybe::val(true).option() {
        None => false,
        Some(_x) => true,
    });
    assert_eq!(false, match Maybe::just(None::<bool>).option() {
        None => false,
        Some(_x) => true,
    });
}
