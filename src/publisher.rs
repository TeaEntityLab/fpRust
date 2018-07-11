use common::{
    Observable,
    RawFunc,
    // get_mut,
    Subscription,
    SubscriptionFunc,
};
use handler::Handler;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone)]
pub struct Publisher<X, T> {
    observers: Vec<Arc<T>>,

    sub_handler: Option<Arc<Handler>>,

    _x: PhantomData<X>,
}
impl<X: Send + Sync + 'static + Clone> Publisher<X, SubscriptionFunc<X>> {
    pub fn new() -> Publisher<X, SubscriptionFunc<X>> {
        return Publisher {
            observers: vec![],
            sub_handler: None,
            _x: PhantomData,
        };
    }
    pub fn new_with_handlers(
        h: Option<Arc<Handler + 'static>>,
    ) -> Publisher<X, SubscriptionFunc<X>> {
        let mut new_one = Publisher::new();
        new_one.subscribe_on(h);
        return new_one;
    }

    fn publish(&mut self, val: X) {
        self.notify_observers(Arc::new(val));
    }

    pub fn subscribe(&mut self, s: Arc<SubscriptionFunc<X>>) {
        self.add_observer(s);
    }
    pub fn subscribe_fn(&mut self, func: impl FnMut(Arc<X>) + Send + Sync + 'static) {
        self.subscribe(Arc::new(SubscriptionFunc::new(func)))
    }

    pub fn subscribe_on(&mut self, h: Option<Arc<Handler + 'static>>) {
        self.sub_handler = h;
    }
}
impl<X: Send + Sync + 'static + Clone> Observable<X, SubscriptionFunc<X>>
    for Publisher<X, SubscriptionFunc<X>>
{
    fn add_observer(&mut self, observer: Arc<SubscriptionFunc<X>>) {
        // println!("add_observer({});", observer);
        self.observers.push(observer);
    }
    fn delete_observer(&mut self, mut observer: Arc<SubscriptionFunc<X>>) {
        for (index, obs) in self.observers.clone().iter().enumerate() {
            if Arc::make_mut(&mut obs.clone()) == Arc::make_mut(&mut observer) {
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
                let mut _observer = observer.clone();
                let observer = Arc::make_mut(&mut _observer);
                observer.on_next(val.clone());
            }
        });

        let sub_handler_thread = &mut self.sub_handler;
        let mut do_sub_thread_ob = _do_sub.clone();

        match sub_handler_thread {
            Some(ref mut sub_handler) => {
                let mut do_sub_thread_sub = _do_sub.clone();
                Arc::get_mut(sub_handler)
                    .unwrap()
                    .post(RawFunc::new(move || {
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
    use common::SubscriptionFunc;
    use handler::HandlerThread;
    use std::sync::{Arc, Condvar, Mutex};
    use std::{thread, time};

    let mut pub1 = Publisher::new();
    pub1.subscribe_fn(|x: Arc<u16>| {
        println!("pub1 {:?}", x);
        assert_eq!(9, *Arc::make_mut(&mut x.clone()));
    });
    pub1.publish(9);

    let mut _h = HandlerThread::new();
    let mut pub2 = Publisher::new_with_handlers(Some(_h.clone()));

    let h = Arc::make_mut(&mut _h);

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

    thread::sleep(time::Duration::from_millis(100));

    println!("hh2");
    h.start();
    println!("hh2 running");

    let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("pub2-s1 I got {:?}", x);

        let &(ref lock, ref cvar) = &*pair2;
        let mut started = lock.lock().unwrap();
        *started = true;

        cvar.notify_one();
    }));
    pub2.subscribe(s);
    pub2.subscribe(Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("pub2-s2 I got {:?}", x);
    })));

    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));

    pub2.publish(String::from("OKOK"));
    pub2.publish(String::from("OKOK2"));
    thread::sleep(time::Duration::from_millis(100));

    let &(ref lock, ref cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }
}
