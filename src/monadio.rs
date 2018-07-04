
use std::panic;

type F<X, Y> = Fn(X) -> Y;

pub struct MonadIO<F> {
    effect : F,
}

impl <F> MonadIO<F> {

    // pub fn just<Y>(r :Y) -> MonadIO<F> {
    //     return MonadIO::new(|x| r);
    // }
    pub fn new(effect: F) -> MonadIO<F> {
        return MonadIO {
            effect,
        }
    }
}

#[test]
fn test_monadio_new() {
    let f = MonadIO::new(|x| x*3).effect;
    assert_eq!(9, (f)(3));
}
