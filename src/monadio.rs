
use common::Subscription;

pub struct MonadIO<Y, EFFECT : FnOnce()->Y> {
    effect : EFFECT,
}

impl <Y, EFFECT : FnOnce()->Y> MonadIO<Y, EFFECT> {

    // pub fn just<Z : 'static, F2 : FnOnce()->Z>(r : Z) -> MonadIO<Z, impl FnOnce()->Z> {
    //     return MonadIO::new(|| r);
    // }

    pub fn new(effect : EFFECT) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
        }
    }

    pub fn fmap<Z, F : FnOnce(Y)->Z>(self, func : F) -> MonadIO<Z, impl FnOnce()->Z> {
        return MonadIO::new(move || func( (self.effect)() ));
    }
    pub fn subscribe(self, s : &mut impl Subscription<Y>) {
        s.on_next( (self.effect)() )
    }
    pub fn subscribe_fn(self, func : impl FnOnce(Y)) {
        (func)( (self.effect)() )
    }
}

#[test]
fn test_monadio_new() {
    use std::sync::Arc;
    use common::SubscriptionFunc;

    let f1 = MonadIO::new(|| 3);
    assert_eq!(3, (f1.effect)());
    let f2 = f1.fmap(|x| x*3);

    let mut v = 0;
    f2.subscribe_fn(|x| v = x);
    assert_eq!(9, v);

    let mut s = SubscriptionFunc::new(move |x: &mut Option<u16>| {
        println!("I'm here {:?}", x);
    });
    let f3 = MonadIO::new(|| 3).fmap(|x| x*3).fmap(|x| x*3);
    f3.subscribe(&mut s);
    assert_eq!(27, Arc::try_unwrap(s.result).ok().unwrap().unwrap());
}
