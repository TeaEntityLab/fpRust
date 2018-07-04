
use std::panic;
// use std::rc::Rc;

use common::Subscription;

type F<Y> = FnOnce() -> Y;

fn _just<Y: 'static>(r :Y) -> Box<F<Y>> {
    return Box::new(|| r);
}

pub struct MonadIO<F> {
    effect : F,
}

impl <F> MonadIO<F> {

    // pub fn just<Y: 'static>(r :Y) -> MonadIO<F> {
    //     return MonadIO {
    //         effect: || r,
    //     };
    // }
    pub fn new(effect: F) -> MonadIO<F> {
        return MonadIO {
            effect,
        }
    }
}

#[test]
fn test_monadio_new() {
    let f1 = MonadIO::new(|| 3).effect;
    assert_eq!(3, (f1)());
}
