/*!
In this module there're implementations & tests of `MonadIO`.
It's inspired by `Rx` & `MonadIO` in `Haskell`
*/
use std::sync::{Arc, Mutex};

use handler::Handler;

use common::{RawFunc, Subscription, SubscriptionFunc};

/**
`MonadIO` implements basic `Rx`/`MonadIO` APIs.
The `observe` and `subscribe` actions could be sync/async,
and `observe` & `subscribe` could be on other `thread`s
(by setting up `observe_on` and `subscribe_on`).

# Arguments

* `Y` - The generic type of data

# Remarks

It's inspired by `Rx` & `MonadIO` in `Haskell`
, and easily run it on sync/async scenaios.

``
*/
#[derive(Clone)]
pub struct MonadIO<Y> {
    effect: Arc<Mutex<dyn FnMut() -> Y + Send + Sync + 'static>>,
    ob_handler: Option<Arc<Mutex<Handler>>>,
    sub_handler: Option<Arc<Mutex<Handler>>>,
}

pub fn of<Z: 'static + Send + Sync + Clone>(r: Z) -> impl FnMut() -> Z + Send + Sync + 'static {
    let _r = Box::new(r);

    return move || {
        *_r.clone()
    };
}

impl<Y: 'static + Send + Sync + Clone> MonadIO<Y> {
    pub fn just(r: Y) -> MonadIO<Y> {
        return MonadIO::new(of(r));
    }
}

impl<Y: 'static + Send + Sync> MonadIO<Y> {

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
        &self,
        func: impl FnMut(Y) -> Z + Send + Sync + 'static,
    ) -> MonadIO<Z> {
        let _func = Arc::new(Mutex::new(func));
        let mut _effect = self.effect.clone();

        return MonadIO::new_with_handlers(
            move || {
                let _func = _func.clone();
                let func = &mut *_func.lock().unwrap();

                let effect = &mut *_effect.lock().unwrap();

                (func)((effect)())
            },
            self.ob_handler.clone(),
            self.sub_handler.clone(),
        );
    }
    pub fn fmap<Z: 'static + Send + Sync + Clone>(
        &self,
        func: impl FnMut(Y) -> MonadIO<Z> + Send + Sync + 'static,
    ) -> MonadIO<Z> {
        let mut _func = Arc::new(Mutex::new(func));

        return self.map(move |y: Y| {
            let _func = _func.clone();
            let func = &mut *_func.lock().unwrap();
            let mut _effect = (func)(y).effect;

            let effect = &mut *_effect.lock().unwrap();

            (effect)()
        });
    }
    pub fn subscribe(&self, s: Arc<Mutex<impl Subscription<Y>>>) {
        let mut _effect = self.effect.clone();
        let mut _do_ob = Arc::new(move || {
            let effect = &mut *_effect.lock().unwrap();

            return (effect)();
        });
        let mut _do_sub = s;

        match &self.ob_handler {
            Some(ob_handler) => {
                let mut sub_handler_thread = Arc::new(self.sub_handler.clone());
                ob_handler.lock().unwrap().post(RawFunc::new(move || {

                    let mut do_ob_thread_ob = _do_ob.clone();
                    let ob = Arc::make_mut(&mut do_ob_thread_ob);

                    match Arc::make_mut(&mut sub_handler_thread) {
                        Some(ref mut sub_handler) => {
                            let do_sub = _do_sub.clone();
                            let result = Arc::new(ob());
                            sub_handler.lock().unwrap().post(RawFunc::new(move || {
                                let result = result.clone();
                                let mut sub = do_sub.lock().unwrap();

                                sub.on_next(result);
                            }));
                        }
                        None => {
                            let mut sub = _do_sub.lock().unwrap();
                            sub.on_next(Arc::new(ob()));
                        }
                    }
                }));
            }
            None => {
                let effect = Arc::make_mut(&mut _do_ob);
                let mut sub = _do_sub.lock().unwrap();

                sub.on_next(Arc::new(effect()));
            }
        }
    }
    pub fn subscribe_fn(&self, func: impl FnMut(Arc<Y>) + Send + Sync + 'static) {
        self.subscribe(Arc::new(Mutex::new(SubscriptionFunc::new(func))))
    }
}

#[test]
fn test_monadio_new() {
    use common::SubscriptionFunc;
    use handler::HandlerThread;
    use std::sync::Arc;
    use std::{thread, time};

    use sync::CountDownLatch;

    let monadio_simple = MonadIO::just(3);
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
    let mut _subscription = Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<u16>| {
        println!("monadio_sync {:?}", x); // monadio_sync 36
        assert_eq!(36, *Arc::make_mut(&mut x.clone()));
    })));
    let subscription = _subscription.clone();
    let monadio_sync = MonadIO::just(1)
        .fmap(|x| MonadIO::new(move || x * 4))
        .map(|x| x * 3)
        .map(|x| x * 3);
    monadio_sync.subscribe(subscription);

    // fmap & map (async)
    let mut _handler_observe_on = HandlerThread::new_with_mutex();
    let mut _handler_subscribe_on = HandlerThread::new_with_mutex();
    let monadio_async = MonadIO::new_with_handlers(
        || {
            println!("In string");
            String::from("ok")
        },
        Some(_handler_observe_on.clone()),
        Some(_handler_subscribe_on.clone()),
    );

    let latch = CountDownLatch::new(1);
    let latch2 = latch.clone();

    thread::sleep(time::Duration::from_millis(100));

    let subscription = Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("monadio_async {:?}", x); // monadio_async ok

        latch2.countdown(); // Unlock here
    })));
    monadio_async.subscribe(subscription);
    monadio_async.subscribe(Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("monadio_async sub2 {:?}", x); // monadio_async sub2 ok
    }))));
    {
        let mut handler_observe_on = _handler_observe_on.lock().unwrap();
        let mut handler_subscribe_on = _handler_subscribe_on.lock().unwrap();

        println!("hh2");
        handler_observe_on.start();
        handler_subscribe_on.start();
        println!("hh2 running");

        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
    }
    thread::sleep(time::Duration::from_millis(100));

    // Waiting for being unlcoked
    latch.clone().wait();
}
