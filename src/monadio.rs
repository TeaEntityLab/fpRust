
use std::panic;
// use std::rc::Rc;

use common::Subscription;

type F<X, Y> = FnOnce(X) -> Y;

// fn _just<Y: 'static>(r :Y) -> Box<F<Option<bool>, Y>> {
//     return Box::new(|_x: Option<bool>| r);
// }

pub struct MonadIO<F> {
    effect : F,
}

impl <F> MonadIO<F> {

    // pub fn just<X, Y: 'static>(r :Y) -> MonadIO<F> {
    //     return MonadIO::new(*_just(r));
    // }
    pub fn new(effect: F) -> MonadIO<F> {
        return MonadIO {
            effect,
        }
    }
}

#[test]
fn test_monadio_new() {
    let f1 = MonadIO::new(|x| 3).effect;
    let f2 = MonadIO::new(|x| x*3).effect;
    assert_eq!(9, (f2)((f1)(0)));
}
