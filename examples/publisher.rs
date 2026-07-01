use std::sync::Arc;

use fp_rust::common::{RawFunc, SubscriptionFunc};
use fp_rust::handler::{Handler, HandlerThread};
use fp_rust::publisher::Publisher;
use fp_rust::sync::CountDownLatch;

fn main() {
    let mut pub1 = Publisher::new();
    pub1.subscribe_fn(|x: Arc<u16>| {
        println!("pub1 {:?}", x);
        assert_eq!(9, *Arc::make_mut(&mut x.clone()));
    });
    pub1.publish(9);

    let handler = HandlerThread::new_with_mutex();

    let mut pub2 = Publisher::new_with_handlers(Some(handler.clone()));

    let latch = CountDownLatch::new(1);
    let latch2 = latch.clone();

    let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("pub2-s1 I got {:?}", x);
        latch2.countdown();
    }));
    pub2.subscribe(s.clone());
    pub2.map(move |x: Arc<String>| {
        println!("pub2-s2 I got {:?}", x);
    });

    {
        let h = &mut handler.lock().unwrap();
        h.start();

        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
        h.post(RawFunc::new(move || {}));
    }

    pub2.publish(String::from("OKOK"));
    pub2.publish(String::from("OKOK2"));

    pub2.unsubscribe(s);

    pub2.publish(String::from("OKOK3"));

    latch.wait();
}
