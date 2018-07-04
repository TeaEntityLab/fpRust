
use std::panic;
// use std::rc::Rc;

use common::Subscription;

// type F<Y> = FnOnce() -> Y;
//
// fn _just<Y: 'static>(r :Y) -> Box<F<Y>> {
//     return Box::new(|| r);
// }

pub struct MonadIO<Y, F: FnOnce()->Y> {
    effect : F,
}

impl <Y, EFFECT: FnOnce()->Y> MonadIO<Y, EFFECT> {

    // pub fn just<Y2: 'static, F2: FnOnce()->Y2>(r :Y2) -> MonadIO<Y2, F2> {
    //     return MonadIO::new(|| r);
    // }
    pub fn new(effect: EFFECT) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
        }
    }

    // pub fn fmap<Z, F: FnOnce(Y)->Z, EFFECT2: FnOnce()->Z>(self, func: F) -> MonadIO<Z, EFFECT2> {
    //     return MonadIO::new(|| func((self.effect)()));
    // }
}

#[test]
fn test_monadio_new() {
    let f1 = MonadIO::new(|| 3).effect;
    assert_eq!(3, (f1)());
}
