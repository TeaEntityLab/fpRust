
use std::panic;
use std::marker::PhantomData;

pub struct MonadIO<F, X, Y> where F: FnOnce(X) -> Y {
    effect : F,

    _p01: PhantomData<X>,
    _p02: PhantomData<Y>,
}

impl <F, X, Y> MonadIO<F, X, Y> where F: FnOnce(X) -> Y {

    // pub fn just(r :& Y) -> MonadIO<F, X, Y> {
    //     return MonadIO::new(|x| *r);
    // }
    pub fn new<F2>(effect: F2) -> MonadIO<F2, X, Y> where F2: FnOnce(X) -> Y {
        return MonadIO {
            effect,

            _p01:PhantomData,
            _p02:PhantomData,
        }
    }
}
