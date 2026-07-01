/*!
In this module, there're implementations & tests of `Publisher`
*/
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

#[cfg(feature = "for_futures")]
use super::common::LinkedListAsync;

use super::common::{Observable, RawFunc, Subscription, SubscriptionFunc, UniqueId};
use super::handler::Handler;
use super::sync::{BlockingQueue, Queue};

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
    // observers: Vec<Arc<dyn Subscription<X>>>,
    observers: Vec<Arc<SubscriptionFunc<X>>>,

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

impl<X> Publisher<X> {
    pub fn new() -> Publisher<X> {
        Default::default()
    }
    pub fn new_with_handlers(h: Option<Arc<Mutex<dyn Handler + 'static>>>) -> Publisher<X> {
        let mut new_one = Publisher::new();
        new_one.subscribe_on(h);
        new_one
    }

    pub fn subscribe_on(&mut self, h: Option<Arc<Mutex<dyn Handler + 'static>>>) {
        self.sub_handler = h;
    }
}
impl<X> Publisher<X>
where
    X: Send + Sync + 'static,
{
    pub fn publish(&mut self, val: X) {
        self.notify_observers(Arc::new(val));
    }

    pub fn subscribe(&mut self, s: Arc<SubscriptionFunc<X>>) -> Arc<SubscriptionFunc<X>> {
        self.add_observer(s.clone());
        s
    }
    pub fn subscribe_fn(
        &mut self,
        func: impl FnMut(Arc<X>) + Send + Sync + 'static,
    ) -> Arc<SubscriptionFunc<X>> {
        self.subscribe(Arc::new(SubscriptionFunc::new(func)))
    }
    pub fn map<Z: Send + Sync + 'static>(
        &mut self,
        func: impl FnMut(Arc<X>) -> Z + Send + Sync + 'static,
    ) -> Arc<SubscriptionFunc<X>> {
        let _func = Arc::new(Mutex::new(func));
        self.subscribe_fn(move |x: Arc<X>| {
            (_func.lock().unwrap())(x);
        })
    }
    pub fn unsubscribe(&mut self, s: Arc<SubscriptionFunc<X>>) {
        self.delete_observer(s);
    }

    pub fn subscribe_blocking_queue(
        &mut self,
        queue: &BlockingQueue<Arc<X>>,
    ) -> Arc<SubscriptionFunc<X>> {
        let mut queue_new = queue.clone();
        self.subscribe_fn(move |v| queue_new.put(v))
    }
    pub fn as_blocking_queue(&mut self) -> (Arc<SubscriptionFunc<X>>, BlockingQueue<Arc<X>>) {
        let queue = BlockingQueue::new();
        let subscription = self.subscribe_blocking_queue(&queue);

        (subscription, queue)
    }
}

#[cfg(feature = "for_futures")]
impl<X: Send + Sync + 'static + Unpin> Publisher<X> {
    pub fn subscribe_as_stream(&mut self, s: Arc<SubscriptionFunc<X>>) -> LinkedListAsync<Arc<X>> {
        self.subscribe(s).as_ref().clone().as_stream()
    }
    pub fn subscribe_fn_as_stream(
        &mut self,
        func: impl FnMut(Arc<X>) + Send + Sync + 'static,
    ) -> LinkedListAsync<Arc<X>> {
        self.subscribe_fn(func).as_ref().clone().as_stream()
    }
    pub fn as_stream(&mut self) -> LinkedListAsync<Arc<X>> {
        self.subscribe_fn_as_stream(|_| {})
    }
}
impl<X: Send + Sync + 'static> Observable<X, SubscriptionFunc<X>> for Publisher<X> {
    fn add_observer(&mut self, observer: Arc<SubscriptionFunc<X>>) {
        // println!("add_observer({});", observer);
        self.observers.push(observer);
    }
    fn delete_observer(&mut self, observer: Arc<SubscriptionFunc<X>>) {
        let id;
        {
            id = observer.get_id();

            #[cfg(feature = "for_futures")]
            observer.as_ref().clone().close_stream();
        }

        for (index, obs) in self.observers.iter().enumerate() {
            if obs.get_id() == id {
                // println!("delete_observer({});", observer);
                self.observers.remove(index);
                return;
            }
        }
    }
    fn notify_observers(&mut self, val: Arc<X>) {
        let observers = self.observers.clone();
        let mut _do_sub = Arc::new(move || {
            for (_, observer) in observers.iter().enumerate() {
                {
                    observer.on_next(val.clone());
                }
            }
        });

        match &mut self.sub_handler {
            Some(sub_handler) => {
                sub_handler.lock().unwrap().post(RawFunc::new(move || {
                    let sub = Arc::make_mut(&mut _do_sub);

                    (sub)();
                }));
            }
            None => {
                let sub = Arc::make_mut(&mut _do_sub);
                (sub)();
            }
        };
    }
}

#[cfg(feature = "for_futures")]
#[futures_test::test]
async fn test_publisher_stream() {
    use std::sync::Arc;

    use futures::StreamExt;

    use super::common::SubscriptionFunc;
    use super::handler::HandlerThread;

    // use super::sync::CountDownLatch;

    let mut _h = HandlerThread::new_with_mutex();
    let mut pub1 = Publisher::new_with_handlers(Some(_h.clone()));
    //*
    let s = pub1.subscribe_as_stream(Arc::new(SubscriptionFunc::new(move |x| {
        println!("test_publisher_stream {:?}: {:?}", "SS", x);
    })));
    // */
    {
        let h = &mut _h.lock().unwrap();

        println!("test_publisher_stream hh2");
        h.start();
        println!("test_publisher_stream hh2 running");
    }
    pub1.publish(1);
    pub1.publish(2);
    pub1.publish(3);
    pub1.publish(4);
    let mut got_list = Vec::<Arc<i32>>::new();
    {
        let mut result = s.clone();
        for n in 1..5 {
            println!("test_publisher_stream {:?}: {:?}", n, "Before");
            let item = result.next().await;
            if let Some(result) = item.clone() {
                (&mut got_list).push(result.clone());
            }
            println!("test_publisher_stream {:?}: {:?}", n, item);
        }
        // got_list = s.collect::<Vec<_>>().await;
    }
    assert_eq!(
        vec![Arc::new(1), Arc::new(2), Arc::new(3), Arc::new(4),],
        got_list
    );
    pub1.publish(5);
    pub1.publish(6);
    pub1.publish(7);
    pub1.publish(8);
    let s2 = pub1.as_stream();
    {
        let h = &mut _h.lock().unwrap();

        let mut s = s.clone();
        h.post(RawFunc::new(move || {
            s.close_stream();
        }));
    }
    pub1.publish(9);
    pub1.publish(10);
    pub1.publish(11);
    pub1.publish(12);
    {
        let h = &mut _h.lock().unwrap();

        let mut s2 = s2.clone();
        h.post(RawFunc::new(move || {
            s2.close_stream();
        }));
    }

    std::thread::sleep(std::time::Duration::from_millis(1));

    got_list = s.clone().collect::<Vec<_>>().await;
    assert_eq!(
        vec![Arc::new(5), Arc::new(6), Arc::new(7), Arc::new(8),],
        got_list
    );
    got_list = s2.clone().collect::<Vec<_>>().await;
    assert_eq!(
        vec![Arc::new(9), Arc::new(10), Arc::new(11), Arc::new(12),],
        got_list
    );
}

#[test]
fn test_publisher_new() {
    use std::sync::Arc;

    use super::common::SubscriptionFunc;
    use super::handler::HandlerThread;

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

    let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("pub2-s1 I got {:?}", x);

        assert_eq!(true, x != Arc::new(String::from("OKOK3")));

        latch2.countdown();
    }));
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

#[test]
fn test_publisher_subscribe_fn_sync() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    // Without a handler, publish notifies observers synchronously.
    let captured = Arc::new(AtomicI32::new(0));
    let captured_thread = captured.clone();
    let mut p = Publisher::new();
    p.subscribe_fn(move |x: Arc<i32>| {
        captured_thread.store(*x, Ordering::SeqCst);
    });
    p.publish(9);
    assert_eq!(9, captured.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_multiple_subscribers_all_notified() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let total = Arc::new(AtomicI32::new(0));
    let mut p = Publisher::new();
    for _ in 0..3 {
        let total_thread = total.clone();
        p.subscribe_fn(move |x: Arc<i32>| {
            total_thread.fetch_add(*x, Ordering::SeqCst);
        });
    }
    p.publish(10);
    // Three subscribers each add 10.
    assert_eq!(30, total.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_publish_multiple_values() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let sum = Arc::new(AtomicI32::new(0));
    let sum_thread = sum.clone();
    let mut p = Publisher::new();
    p.subscribe_fn(move |x: Arc<i32>| {
        sum_thread.fetch_add(*x, Ordering::SeqCst);
    });
    p.publish(1);
    p.publish(2);
    p.publish(3);
    assert_eq!(6, sum.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_unsubscribe_stops_delivery() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    use super::common::SubscriptionFunc;

    let count = Arc::new(AtomicI32::new(0));
    let count_thread = count.clone();
    let mut p = Publisher::new();
    let s = Arc::new(SubscriptionFunc::new(move |_x: Arc<i32>| {
        count_thread.fetch_add(1, Ordering::SeqCst);
    }));
    p.subscribe(s.clone());

    p.publish(1); // delivered
    assert_eq!(1, count.load(Ordering::SeqCst));

    p.unsubscribe(s);
    p.publish(2); // not delivered
    assert_eq!(1, count.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_unsubscribe_removes_correct_observer() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    use super::common::SubscriptionFunc;

    // Two subscriptions constructed back-to-back (same tick). Before
    // generate_id gained a monotonic counter their ids collided, so
    // delete_observer matched by id and removed the FIRST equal id (a),
    // not the one requested (b). Separate counters detect which survived.
    let count_a = Arc::new(AtomicI32::new(0));
    let count_b = Arc::new(AtomicI32::new(0));
    let ca = count_a.clone();
    let cb = count_b.clone();

    let mut p = Publisher::new();
    let a = Arc::new(SubscriptionFunc::new(move |_x: Arc<i32>| {
        ca.fetch_add(1, Ordering::SeqCst);
    }));
    let b = Arc::new(SubscriptionFunc::new(move |_x: Arc<i32>| {
        cb.fetch_add(1, Ordering::SeqCst);
    }));
    p.subscribe(a.clone());
    p.subscribe(b.clone());

    p.publish(1); // both fire
    assert_eq!(1, count_a.load(Ordering::SeqCst));
    assert_eq!(1, count_b.load(Ordering::SeqCst));

    p.unsubscribe(b); // must remove b, keep a
    p.publish(2);
    // a still subscribed -> fires again; b gone -> unchanged.
    assert_eq!(2, count_a.load(Ordering::SeqCst));
    assert_eq!(1, count_b.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_subscribe_returns_same_handle() {
    use std::sync::Arc;

    use super::common::{SubscriptionFunc, UniqueId};

    let mut p = Publisher::new();
    let s = Arc::new(SubscriptionFunc::new(|_x: Arc<i32>| {}));
    let returned = p.subscribe(s.clone());
    // subscribe hands back the same subscription (same id).
    assert_eq!(s.get_id(), returned.get_id());
}

#[test]
fn test_publisher_map_runs_on_publish() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    // map registers a transforming side-effect that runs on each publish.
    let seen = Arc::new(AtomicI32::new(0));
    let seen_thread = seen.clone();
    let mut p = Publisher::new();
    p.map(move |x: Arc<i32>| {
        let doubled = *x * 2;
        seen_thread.store(doubled, Ordering::SeqCst);
        doubled
    });
    p.publish(21);
    assert_eq!(42, seen.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_as_blocking_queue() {
    use std::sync::Arc;

    let mut p = Publisher::new();
    let (_sub, mut queue) = p.as_blocking_queue();
    p.publish(5);
    p.publish(6);
    // Published values arrive (as Arc<X>) in FIFO order.
    assert_eq!(Some(Arc::new(5)), queue.poll());
    assert_eq!(Some(Arc::new(6)), queue.poll());
    assert_eq!(None, queue.poll());
}

#[test]
fn test_publisher_subscribe_blocking_queue() {
    use std::sync::Arc;

    let queue = BlockingQueue::<Arc<i32>>::new();
    let mut p = Publisher::new();
    p.subscribe_blocking_queue(&queue);
    p.publish(100);

    let mut drained = queue.clone();
    assert_eq!(Some(Arc::new(100)), drained.poll());
}

#[test]
fn test_publisher_delete_observer_directly() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    use super::common::SubscriptionFunc;

    // Observable::delete_observer is the lower-level form of unsubscribe.
    let count = Arc::new(AtomicI32::new(0));
    let count_thread = count.clone();
    let mut p = Publisher::new();
    let s = Arc::new(SubscriptionFunc::new(move |_x: Arc<i32>| {
        count_thread.fetch_add(1, Ordering::SeqCst);
    }));
    p.add_observer(s.clone());
    p.publish(1);
    p.delete_observer(s);
    p.publish(1);
    assert_eq!(1, count.load(Ordering::SeqCst));
}

#[test]
fn test_publisher_default_has_no_observers() {
    use std::sync::Arc;

    // A default Publisher with no subscribers simply drops published values.
    let mut p: Publisher<i32> = Default::default();
    p.publish(1); // no panic, no observers
    // Adding one afterwards works as normal.
    let queue = BlockingQueue::<Arc<i32>>::new();
    p.subscribe_blocking_queue(&queue);
    p.publish(2);
    let mut drained = queue.clone();
    assert_eq!(Some(Arc::new(2)), drained.poll());
}
