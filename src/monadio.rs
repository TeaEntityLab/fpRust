use std::sync::{
    Arc,
};

use handler::{
    Handler,
};

use common::Subscription;

pub struct MonadIO<Y, EFFECT : FnOnce()->Y> {
    effect : EFFECT,
    obHandler : Option<Arc<Handler>>,
    subHandler : Option<Arc<Handler>>,
}

impl <Y, EFFECT : FnOnce()->Y> MonadIO<Y, EFFECT> {

    // pub fn just<Z : 'static, F2 : FnOnce()->Z>(r : Z) -> MonadIO<Z, impl FnOnce()->Z> {
    //     return MonadIO::new(|| r);
    // }

    pub fn new(effect : EFFECT) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
            obHandler: None,
            subHandler: None,
        }
    }

    pub fn new_with_handlers(effect : EFFECT, ob : Arc<impl Handler + 'static>,  sub : Arc<impl Handler + 'static>) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
            obHandler: Some(ob),
            subHandler: Some(sub),
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
    use std::sync::{
        Arc,
        Mutex,
        Condvar,
    };
    use handler::{
        HandlerThread,
    };
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

    let mut _h = HandlerThread::new();
    let mut _h2 = HandlerThread::new();
    let f4 = MonadIO::new_with_handlers(|| "ok", _h.clone(), _h2.clone());

    let h = Arc::make_mut(&mut _h);
    h.start();
    let h2 = Arc::make_mut(&mut _h2);
    h2.start();

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

}
