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
    fn from_val(r: T) -> Maybe<T> {
        return Maybe::just(Some(r));
    }

    fn present(self) -> bool {
        match self.r {
            Some(ref _x) => return true,
            None => return false,
        }
    }
    fn null(self) -> bool {
        return ! self.present();
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
    assert_eq!(true, Maybe::from_val(true).present());

    assert_eq!(true, Maybe::just(None::<bool>).null());
    assert_eq!(false, Maybe::from_val(true).null());
}
#[test]
fn test_maybe_unwrap() {

    assert_eq!(false, Maybe::just(None::<bool>).or(false));
    assert_eq!(true, Maybe::from_val(true).or(false));

    let none_unwrap = panic::catch_unwind(|| {
        Maybe::just(None::<bool>).unwrap();
    });
    assert_eq!(true, none_unwrap.is_err());
    assert_eq!(true, Maybe::from_val(true).unwrap());

    assert_eq!(true, match Maybe::from_val(true).option() {
        None => false,
        Some(_x) => true,
    });
    assert_eq!(false, match Maybe::just(None::<bool>).option() {
        None => false,
        Some(_x) => true,
    });
}
