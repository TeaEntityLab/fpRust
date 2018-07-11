use std::sync::Arc;

use handler::Handler;

use common::{RawFunc, Subscription, SubscriptionFunc};

#[derive(Clone)]
pub struct MonadIO<Y> {
    effect: Arc<dyn FnMut() -> Y + Send + Sync + 'static>,
    ob_handler: Option<Arc<Handler>>,
    sub_handler: Option<Arc<Handler>>,
}

pub fn of<Z: 'static + Send + Sync + Clone>(r: Z) -> impl FnMut() -> Z + Send + Sync + 'static {
    let _r = Box::new(r);

    return move || {
        let r = _r.clone();
        *r
    };
}

impl<Y: 'static + Send + Sync + Clone> MonadIO<Y> {
    pub fn just(r: Y) -> MonadIO<Y> {
        let _r = Box::new(r);

        return MonadIO::new(move || {
            let r = _r.clone();
            *r
        });
    }

    pub fn new(effect: impl FnMut() -> Y + Send + Sync + 'static) -> MonadIO<Y> {
        return MonadIO {
            effect: Arc::new(effect),
            ob_handler: None,
            sub_handler: None,
        };
    }

    pub fn new_with_handlers(
        effect: impl FnMut() -> Y + Send + Sync + 'static,
        ob: Option<Arc<Handler + 'static>>,
        sub: Option<Arc<Handler + 'static>>,
    ) -> MonadIO<Y> {
        return MonadIO {
            effect: Arc::new(effect),
            ob_handler: ob,
            sub_handler: sub,
        };
    }

    pub fn observe_on(mut self, h: Option<Arc<Handler + 'static>>) {
        self.ob_handler = h;
    }

    pub fn subscribe_on(mut self, h: Option<Arc<Handler + 'static>>) {
        self.sub_handler = h;
    }

    pub fn map<Z: 'static + Send + Sync + Clone>(
        self,
        func: impl FnMut(Y) -> Z + Send + Sync + 'static + Clone,
    ) -> MonadIO<Z> {
        let _func = Arc::new(func);
        let mut effect = self.effect;

        return MonadIO::new_with_handlers(
            move || {
                let mut func = _func.clone();
                (Arc::make_mut(&mut func))((Arc::get_mut(&mut effect).unwrap())())
            },
            self.ob_handler,
            self.sub_handler,
        );
    }
    pub fn fmap<Z: 'static + Send + Sync + Clone>(
        self,
        func: impl FnMut(Y) -> MonadIO<Z> + Send + Sync + 'static + Clone,
    ) -> MonadIO<Z> {
        let mut _func = Arc::new(func);

        return self.map(move |y: Y| {
            let mut func = _func.clone();
            let mut _effect = (Arc::make_mut(&mut func))(y).effect;
            (Arc::get_mut(&mut _effect).unwrap())()
        });
    }
    pub fn subscribe(self, s: Arc<impl Subscription<Y> + Clone>) {
        let mut _effect = self.effect;
        let mut _do_ob = Arc::new(move || {
            return (Arc::get_mut(&mut _effect).unwrap())();
        });
        let mut _s = s.clone();
        let mut _do_sub = Arc::new(move |y: Y| {
            Arc::make_mut(&mut _s).on_next(Arc::new(y));
        });

        match self.ob_handler {
            Some(mut ob_handler) => {
                let mut sub_handler_thread = Arc::new(self.sub_handler);
                Arc::get_mut(&mut ob_handler)
                    .unwrap()
                    .post(RawFunc::new(move || {
                        let mut do_ob_thread_ob = _do_ob.clone();
                        let mut do_sub_thread_ob = _do_sub.clone();
                        let ob = Arc::make_mut(&mut do_ob_thread_ob);
                        let sub = Arc::make_mut(&mut do_sub_thread_ob);

                        match Arc::make_mut(&mut sub_handler_thread) {
                            Some(ref mut sub_handler) => {
                                let mut do_ob_thread_sub = _do_ob.clone();
                                let mut do_sub_thread_sub = _do_sub.clone();
                                Arc::get_mut(sub_handler)
                                    .unwrap()
                                    .post(RawFunc::new(move || {
                                        let ob = Arc::make_mut(&mut do_ob_thread_sub);
                                        let sub = Arc::make_mut(&mut do_sub_thread_sub);

                                        (sub)((ob)());
                                    }));
                            }
                            None => {
                                (sub)((ob)());
                            }
                        }
                    }));
            }
            None => {
                let effect = Arc::make_mut(&mut _do_ob);
                let sub = Arc::make_mut(&mut _do_sub);
                sub(effect());
            }
        }
    }
    pub fn subscribe_fn(self, func: impl FnMut(Arc<Y>) + Send + Sync + 'static + Clone) {
        self.subscribe(Arc::new(SubscriptionFunc::new(func)))
    }
}

#[test]
fn test_monadio_new() {
    use common::SubscriptionFunc;
    use handler::HandlerThread;
    use std::sync::{Arc, Condvar, Mutex};
    use std::{thread, time};

    let mut f1 = MonadIO::just(3);
    // let mut f1 = MonadIO::just(3);
    assert_eq!(3, (Arc::get_mut(&mut f1.effect).unwrap())());
    let f2 = f1.map(|x| x * 3);

    f2.subscribe_fn(move |x| {
        println!("f2 {:?}", x);
        assert_eq!(9, *Arc::make_mut(&mut x.clone()));
    });

    let mut _s = Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
        println!("I'm here {:?}", x);
        assert_eq!(36, *Arc::make_mut(&mut x.clone()));
    }));
    let mut _s2 = _s.clone();
    let f3 = MonadIO::new(of(1))
        .fmap(|x| MonadIO::new(move || x * 4))
        .map(|x| x * 3)
        .map(|x| x * 3);
    f3.subscribe(_s2);

    let mut _h = HandlerThread::new();
    let mut _h2 = HandlerThread::new();
    let f4 = MonadIO::new_with_handlers(
        || {
            println!("In string");
            String::from("ok")
        },
        Some(_h.clone()),
        Some(_h2.clone()),
    );

    let h = Arc::make_mut(&mut _h);
    let h2 = Arc::make_mut(&mut _h2);

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

    thread::sleep(time::Duration::from_millis(100));

    println!("hh2");
    h.start();
    h2.start();
    println!("hh2 running");

    let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("I got {:?}", x);

        let &(ref lock, ref cvar) = &*pair2;
        let mut started = lock.lock().unwrap();
        *started = true;

        cvar.notify_one();
    }));
    f4.subscribe(s);

    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));

    thread::sleep(time::Duration::from_millis(100));

    let &(ref lock, ref cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }
}
