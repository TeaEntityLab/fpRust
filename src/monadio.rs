use std::sync::{Arc, Mutex};

use handler::Handler;

use common::{RawFunc, Subscription, SubscriptionFunc};

#[derive(Clone)]
pub struct MonadIO<Y> {
    effect: Arc<Mutex<dyn FnMut() -> Y + Send + Sync + 'static>>,
    ob_handler: Option<Arc<Mutex<Handler>>>,
    sub_handler: Option<Arc<Mutex<Handler>>>,
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
        return MonadIO::new_with_handlers(effect, None, None);
    }

    pub fn new_with_handlers(
        effect: impl FnMut() -> Y + Send + Sync + 'static,
        ob: Option<Arc<Mutex<Handler + 'static>>>,
        sub: Option<Arc<Mutex<Handler + 'static>>>,
    ) -> MonadIO<Y> {
        return MonadIO {
            effect: Arc::new(Mutex::new(effect)),
            ob_handler: ob,
            sub_handler: sub,
        };
    }

    pub fn observe_on(&mut self, h: Option<Arc<Mutex<Handler + 'static>>>) {
        self.ob_handler = h;
    }

    pub fn subscribe_on(&mut self, h: Option<Arc<Mutex<Handler + 'static>>>) {
        self.sub_handler = h;
    }

    pub fn map<Z: 'static + Send + Sync + Clone>(
        self,
        func: impl FnMut(Y) -> Z + Send + Sync + 'static + Clone,
    ) -> MonadIO<Z> {
        let _func = Arc::new(func);
        let mut _effect = self.effect;

        return MonadIO::new_with_handlers(
            move || {
                let mut func = _func.clone();

                let effect = &mut *_effect.lock().unwrap();

                (Arc::make_mut(&mut func))((effect)())
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

            let effect = &mut *_effect.lock().unwrap();

            (effect)()
        });
    }
    pub fn subscribe(&self, s: Arc<impl Subscription<Y> + Clone>) {
        let mut _effect = self.effect.clone();
        let mut _do_ob = Arc::new(move || {
            let effect = &mut *_effect.lock().unwrap();

            return (effect)();
        });
        let mut _s = s.clone();
        let mut _do_sub = Arc::new(move |y: Y| {
            Arc::make_mut(&mut _s).on_next(Arc::new(y));
        });

        match &self.ob_handler {
            Some(ob_handler) => {
                let mut sub_handler_thread = Arc::new(self.sub_handler.clone());
                ob_handler.lock().unwrap().post(RawFunc::new(move || {
                    let mut do_ob_thread_ob = _do_ob.clone();
                    let mut do_sub_thread_ob = _do_sub.clone();
                    let ob = Arc::make_mut(&mut do_ob_thread_ob);
                    let sub = Arc::make_mut(&mut do_sub_thread_ob);

                    match Arc::make_mut(&mut sub_handler_thread) {
                        Some(ref mut sub_handler) => {
                            let mut do_ob_thread_sub = _do_ob.clone();
                            let mut do_sub_thread_sub = _do_sub.clone();

                            sub_handler.lock().unwrap().post(RawFunc::new(move || {
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
    pub fn subscribe_fn(&self, func: impl FnMut(Arc<Y>) + Send + Sync + 'static + Clone) {
        self.subscribe(Arc::new(SubscriptionFunc::new(func)))
    }
}

#[test]
fn test_monadio_new() {
    use common::SubscriptionFunc;
    use handler::HandlerThread;
    use std::sync::{Arc, Condvar, Mutex};
    use std::{thread, time};

    let mut monadio_simple = MonadIO::just(3);
    // let mut monadio_simple = MonadIO::just(3);
    {
        let effect = &mut *monadio_simple.effect.lock().unwrap();
        assert_eq!(3, (effect)());
    }
    let monadio_simple_map = monadio_simple.map(|x| x * 3);

    monadio_simple_map.subscribe_fn(move |x| {
        println!("monadio_simple_map {:?}", x);
        assert_eq!(9, *Arc::make_mut(&mut x.clone()));
    });

    // fmap & map (sync)
    let mut _subscription = Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
        println!("monadioSync {:?}", x); // monadioSync 36
        assert_eq!(36, *Arc::make_mut(&mut x.clone()));
    }));
    let mut subscription = _subscription.clone();
    let monadioSync = MonadIO::just(1)
        .fmap(|x| MonadIO::new(move || x * 4))
        .map(|x| x * 3)
        .map(|x| x * 3);
    monadioSync.subscribe(subscription);

    // fmap & map (async)
    let mut _handlerObserveOn = HandlerThread::new_with_mutex();
    let mut _handlerSubscribeOn = HandlerThread::new_with_mutex();
    let monadioAsync = MonadIO::new_with_handlers(
        || {
            println!("In string");
            String::from("ok")
        },
        Some(_handlerObserveOn.clone()),
        Some(_handlerSubscribeOn.clone()),
    );

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

    thread::sleep(time::Duration::from_millis(100));

    let subscription = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("monadioAsync {:?}", x); // monadioAsync ok

        let &(ref lock, ref cvar) = &*pair2;
        let mut started = lock.lock().unwrap();
        *started = true;

        cvar.notify_one(); // Unlock here
    }));
    monadioAsync.subscribe(subscription);
    monadioAsync.subscribe(Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("monadioAsync sub2 {:?}", x); // monadioAsync sub2 ok
    })));
    {
        let mut handlerObserveOn = _handlerObserveOn.lock().unwrap();
        let mut handlerSubscribeOn = _handlerSubscribeOn.lock().unwrap();

        println!("hh2");
        handlerObserveOn.start();
        handlerSubscribeOn.start();
        println!("hh2 running");

        handlerObserveOn.post(RawFunc::new(move || {}));
        handlerObserveOn.post(RawFunc::new(move || {}));
        handlerObserveOn.post(RawFunc::new(move || {}));
        handlerObserveOn.post(RawFunc::new(move || {}));
        handlerObserveOn.post(RawFunc::new(move || {}));
    }
    thread::sleep(time::Duration::from_millis(100));

    let &(ref lock, ref cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    // Waiting for being unlcoked
    while !*started {
        started = cvar.wait(started).unwrap();
    }
}
