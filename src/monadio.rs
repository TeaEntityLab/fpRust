use std::sync::{
    Arc,
    Mutex,
};

use handler::{
    Handler,
};

use common::{
    Subscription,
    RawFunc,
};

pub struct MonadIO<Y, EFFECT : FnOnce()->Y + Send + Sync + 'static> {
    effect : EFFECT,
    ob_handler : Option<Arc<Handler>>,
    sub_handler : Option<Arc<Handler>>,
}

impl <Y: 'static, EFFECT : FnOnce()->Y + Send + Sync + 'static> MonadIO<Y, EFFECT> {

    // pub fn just<Z : 'static, F2 : FnOnce()->Z>(r : Z) -> MonadIO<Z, impl FnOnce()->Z> {
    //     return MonadIO::new(|| r);
    // }

    pub fn new(effect : EFFECT) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
            ob_handler: None,
            sub_handler: None,
        }
    }

    pub fn new_with_handlers(effect : EFFECT, ob : Option<Arc<Handler + 'static>>,  sub : Option<Arc<Handler + 'static>>) -> MonadIO<Y, EFFECT> {
        return MonadIO {
            effect,
            ob_handler: ob,
            sub_handler: sub,
        }
    }

    pub fn fmap<Z: 'static, F : FnOnce(Y)->Z + Send + Sync + 'static>(self, func : F) -> MonadIO<Z, impl FnOnce()->Z> {
        return MonadIO::new(move || func( (self.effect)() ));
    }
    pub fn subscribe(self, s : Arc<impl Subscription<Y> + Clone>) {

        let _effect = Arc::new(self.effect);
        let mut _do_ob = Arc::new(move ||{
            let effect = Arc::try_unwrap(_effect).ok().unwrap();
            return (effect)();
        });
        let mut _s = s.clone();
        let mut _do_sub = Arc::new(move |y : Y|{
            Arc::make_mut(&mut _s).on_next(y);
        });

        match self.ob_handler {

            Some(mut ob_handler) => {

                let mut sub_handler_thread = Arc::new(self.sub_handler);
                Arc::get_mut(&mut ob_handler).unwrap().post(RawFunc::new(move ||{
                        let do_ob_thread = _do_ob.clone();
                        let mut do_sub_thread = _do_sub.clone();
                        let ob = Arc::try_unwrap(do_ob_thread).ok().unwrap();
                        let sub = Arc::make_mut(&mut do_sub_thread);

                        match Arc::try_unwrap(sub_handler_thread.clone()).ok().unwrap() {
                            Some(mut sub_handler) => {
                                Arc::get_mut(&mut sub_handler).unwrap().post(RawFunc::new(||{}));
                            },
                            None => {
                                (sub)((ob)());
                                },
                        }
                    }));
            },
            None => {
                let effect = Arc::try_unwrap(_do_ob).ok().unwrap();
                let sub = Arc::make_mut(&mut _do_sub);
                sub(effect());
            },
        }
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

    let mut _s = Arc::new(SubscriptionFunc::new(move |x: &mut Option<u16>| {
        println!("I'm here {:?}", x);
    }));
    let mut _s2 = _s.clone();
    let mut s = Arc::make_mut(&mut _s);

    let f3 = MonadIO::new(|| 3).fmap(|x| x*3).fmap(|x| x*3);
    f3.subscribe(_s2);

    let mut _h = HandlerThread::new();
    let mut _h2 = HandlerThread::new();
    let f4 = MonadIO::new_with_handlers(|| "ok", Some(_h.clone()), Some(_h2.clone()));

    let h = Arc::make_mut(&mut _h);
    h.start();
    let h2 = Arc::make_mut(&mut _h2);
    h2.start();

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

}
