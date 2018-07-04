
use std::panic;
// use std::rc::Rc;

use common::Subscription;
use common::FnNext;

pub struct MonadIO<Y, F: FnOnce()->Y> {
    effect : F,
}

impl <Y, EFFECT: FnOnce()->Y> MonadIO<Y, EFFECT> {

    // pub fn just<Z: 'static, F2: FnOnce()->Z>(r :Z) -> MonadIO<Z, impl FnOnce()->Z> {
    //     return MonadIO::new(|| r);
    // }

    pub fn new(effect: EFFECT) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
        }
    }

    pub fn fmap<Z, F: FnOnce(Y)->Z>(self, func: F) -> MonadIO<Z, impl FnOnce()->Z> {
        return MonadIO::new(move || func( (self.effect)() ));
    }
    pub fn subscribe(self, func: impl FnOnce(Y)) {
        (func)( (self.effect)() )
    }
}

#[test]
fn test_monadio_new() {
    let f1 = MonadIO::new(|| 3);
    assert_eq!(3, (f1.effect)());
    let f2 = f1.fmap(|x| x*3);

    let mut v = 0;
    f2.subscribe(|x| v = x);
    assert_eq!(9, v);
}
