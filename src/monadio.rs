
use std::panic;

type F<X, Y> = Fn(X) -> Y;

pub struct MonadIO<F> {
    effect : F,
}

impl <F> MonadIO<F> {

    // pub fn just<F2, Y>(r :Y) -> MonadIO<F2> where F2: Fn(bool) -> Y {
    //     return MonadIO::new(|x: bool| r);
    // }
    pub fn new<F2, X, Y>(effect: F2) -> MonadIO<F2> where F2: Fn(X) -> Y {
        return MonadIO {
            effect,
        }
    }
}
