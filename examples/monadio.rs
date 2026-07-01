use std::sync::Arc;

use fp_rust::common::{RawFunc, SubscriptionFunc};
use fp_rust::handler::{Handler, HandlerThread};
use fp_rust::monadio::MonadIO;
use fp_rust::sync::CountDownLatch;

fn main() {
    // fmap & map (sync)
    let subscription = Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
        println!("monadio_sync {:?}", x);
        assert_eq!(36, *Arc::make_mut(&mut x.clone()));
    }));
    let monadio_sync = MonadIO::just(1)
        .fmap(|x| MonadIO::new(move || x * 4))
        .map(|x| x * 3)
        .map(|x| x * 3);
    monadio_sync.subscribe(subscription);

    // fmap & map (async)
    let handler_observe_on = HandlerThread::new_with_mutex();
    let handler_subscribe_on = HandlerThread::new_with_mutex();
    let monadio_async = MonadIO::new_with_handlers(
        || {
            println!("In string");
            String::from("ok")
        },
        Some(handler_observe_on.clone()),
        Some(handler_subscribe_on.clone()),
    );

    let latch = CountDownLatch::new(1);
    let latch2 = latch.clone();

    let subscription = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("monadio_async {:?}", x);
        latch2.countdown();
    }));
    monadio_async.subscribe(subscription);
    monadio_async.subscribe(Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
        println!("monadio_async sub2 {:?}", x);
    })));

    {
        let mut handler_observe_on = handler_observe_on.lock().unwrap();
        let mut handler_subscribe_on = handler_subscribe_on.lock().unwrap();

        handler_observe_on.start();
        handler_subscribe_on.start();

        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
        handler_observe_on.post(RawFunc::new(move || {}));
    }

    latch.wait();
}
