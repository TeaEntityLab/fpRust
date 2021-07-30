/*!
In this module, there're implementations & tests of `Publisher`
*/
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use super::common::{Observable, RawFunc, Subscription, SubscriptionFunc, UniqueId};
use super::handler::Handler;

/**
The `Publisher` implements *PubSub-like* features.

# Arguments

* `X` - The generic type of yielded/yielding data
* `T` - In order to pass compilations, `T` must be `SubscriptionFunc<X>` (for instantiation)

# Remarks

It could be sync or async up to your usages,
and it could be mapped just as Rx-like APIs.

*/
#[derive(Clone)]
pub struct Publisher<X> {
    observers: Vec<Arc<Mutex<dyn Subscription<X>>>>,

    sub_handler: Option<Arc<Mutex<dyn Handler>>>,

    _x: PhantomData<X>,
}

impl<X> Default for Publisher<X> {
    fn default() -> Self {
        Publisher {
            observers: vec![],
            sub_handler: None,
            _x: PhantomData,
        }
    }
}

impl<X: Send + Sync + 'static> Publisher<X> {
    pub fn new() -> Publisher<X> {
        Default::default()
    }
    pub fn new_with_handlers(h: Option<Arc<Mutex<dyn Handler + 'static>>>) -> Publisher<X> {
        let mut new_one = Publisher::new();
        new_one.subscribe_on(h);
        new_one
    }

    pub fn publish(&mut self, val: X) {
        self.notify_observers(Arc::new(val));
    }

    pub fn subscribe(
        &mut self,
        s: Arc<Mutex<SubscriptionFunc<X>>>,
    ) -> Arc<Mutex<SubscriptionFunc<X>>> {
        self.add_observer(s.clone());
        s
    }
    pub fn subscribe_fn(
        &mut self,
        func: impl FnMut(Arc<X>) + Send + Sync + 'static,
    ) -> Arc<Mutex<SubscriptionFunc<X>>> {
        self.subscribe(Arc::new(Mutex::new(SubscriptionFunc::new(func))))
    }
    pub fn map<Z: Send + Sync + 'static>(
        &mut self,
        func: impl FnMut(Arc<X>) -> Z + Send + Sync + 'static,
    ) -> Arc<Mutex<SubscriptionFunc<X>>> {
        let _func = Arc::new(Mutex::new(func));
        self.subscribe_fn(move |x: Arc<X>| {
            let _func = _func.clone();
            let func = &mut *_func.lock().unwrap();
            (func)(x);
        })
    }
    pub fn unsubscribe(&mut self, s: Arc<Mutex<SubscriptionFunc<X>>>) {
        self.delete_observer(s);
    }

    pub fn subscribe_on(&mut self, h: Option<Arc<Mutex<dyn Handler + 'static>>>) {
        self.sub_handler = h;
    }
}
impl<X: Send + Sync + 'static> Observable<X, SubscriptionFunc<X>> for Publisher<X> {
    fn add_observer(&mut self, observer: Arc<Mutex<SubscriptionFunc<X>>>) {
        // println!("add_observer({});", observer);
        self.observers.push(observer);
    }
    fn delete_observer(&mut self, observer: Arc<Mutex<SubscriptionFunc<X>>>) {
        let id;
        {
            let _observer = &*observer.lock().unwrap();
            id = _observer.get_id();
        }

        for (index, obs) in self.observers.clone().iter().enumerate() {
            let _obs = obs.clone();
            let obs = &*_obs.lock().unwrap();
            if obs.get_id() == id {
                // println!("delete_observer({});", observer);
                self.observers.remove(index);
                return;
            }
        }
    }
    fn notify_observers(&mut self, val: Arc<X>) {
        let _observers = self.observers.clone();
        let observers = Arc::new(_observers);
        let mut _do_sub = Arc::new(move || {
            let mut _observers = observers.clone();
            let observers = Arc::make_mut(&mut _observers);

            for (_, observer) in observers.iter().enumerate() {
                let mut _observer = observer.lock().unwrap();
                _observer.on_next(val.clone());
            }
        });

        let sub_handler_thread = &mut self.sub_handler;
        let mut do_sub_thread_ob = _do_sub.clone();

        match sub_handler_thread {
            Some(ref mut sub_handler) => {
                let mut do_sub_thread_sub = _do_sub.clone();

                sub_handler.lock().unwrap().post(RawFunc::new(move || {
                    let sub = Arc::make_mut(&mut do_sub_thread_sub);

                    (sub)();
                }));
            }
            None => {
                let sub = Arc::make_mut(&mut do_sub_thread_ob);
                (sub)();
            }
        };
    }
}

#[test]
fn test_publisher_new() {
    use super::common::SubscriptionFunc;
    use super::handler::HandlerThread;
    use std::sync::Arc;

    use super::sync::CountDownLatch;

    let mut pub1 = Publisher::new();
    pub1.subscribe_fn(|x: Arc<u16>| {
        println!("pub1 {:?}", x);
        assert_eq!(9, *Arc::make_mut(&mut x.clone()));
    });
    pub1.publish(9);

    let mut _h = HandlerThread::new_with_mutex();

    let mut pub2 = Publisher::new_with_handlers(Some(_h.clone()));

    let latch = CountDownLatch::new(1);
    let latch2 = latch.clone();

    let s = Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("pub2-s1 I got {:?}", x);

        assert_eq!(true, x != Arc::new(String::from("OKOK3")));

        latch2.countdown();
    })));
    pub2.subscribe(s.clone());
    pub2.map(move |x: Arc<String>| {
        println!("pub2-s2 I got {:?}", x);
    });

    {
        let h = &mut _h.lock().unwrap();

        println!("hh2");
        h.start();
        println!("hh2 running");

        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
    }

    pub2.publish(String::from("OKOK"));
    pub2.publish(String::from("OKOK2"));

    pub2.unsubscribe(s.clone());

    pub2.publish(String::from("OKOK3"));

    latch.clone().wait();
}
