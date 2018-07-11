use std::cmp::PartialEq;
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::{Arc, Mutex};

// pub trait FnMutReceiveThreadSafe<X>: FnMut(Arc<X>) + Send + Sync + 'static {}
// pub trait FnMutReturnThreadSafe<X>: FnMut() -> X + Send + Sync + 'static {}

pub fn get_mut<'a, T>(v: &'a mut Vec<T>, index: usize) -> Option<&'a mut T> {
    let mut i = 0;
    for elem in v {
        if index == i {
            return Some(elem);
        }
        i += 1;
    }

    None
}

// NOTE: From https://github.com/eliovir/rust-examples/blob/master/design_pattern-observer.rs
// Observer
// pub trait Observer<X> {
// 	fn on_next(&mut self, x : Arc<X>);
// }
// Observable memorizes all Observers and send notifications
pub trait Observable<X, T: Subscription<X>> {
    fn add_observer(&mut self, observer: Arc<T>);
    fn delete_observer(&mut self, observer: Arc<T>);
    fn notify_observers(&mut self, x: Arc<X>);
}

pub trait Subscription<X>: Send + Sync + 'static + PartialEq + Clone {
    fn on_next(&mut self, x: Arc<X>);
}

#[derive(Clone)]
pub struct SubscriptionFunc<T> {
    id: String,
    pub receiver: RawReceiver<T>,
}

impl<T: Send + Sync + 'static + Clone> SubscriptionFunc<T> {
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> SubscriptionFunc<T> {
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        return SubscriptionFunc {
            id: format!("{:?}", since_the_epoch),
            receiver: RawReceiver::new(func),
        };
    }
}

impl<T: Send + Sync + 'static + Clone> PartialEq for SubscriptionFunc<T> {
    fn eq(&self, other: &SubscriptionFunc<T>) -> bool {
        self.id == other.id
    }
}

impl<T: Send + Sync + 'static + Clone> Subscription<T> for SubscriptionFunc<T> {
    fn on_next(&mut self, x: Arc<T>) {
        self.receiver.invoke(x);
    }
}

#[derive(Clone)]
pub struct RawReceiver<T> {
    func: Arc<Mutex<dyn FnMut(Arc<T>) + Send + Sync + 'static>>,
    _t: PhantomData<T>,
}

impl<T> RawReceiver<T> {
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> RawReceiver<T> {
        return RawReceiver {
            func: Arc::new(Mutex::new(func)),
            _t: PhantomData,
        };
    }

    pub fn invoke(&mut self, x: Arc<T>) {
        let func = &mut *self.func.lock().unwrap();
        (func)(x);
    }
}

#[derive(Clone)]
pub struct RawFunc {
    func: Arc<FnMut() + Send + Sync + 'static>,
}

impl RawFunc {
    pub fn new<T>(func: T) -> RawFunc
    where
        T: FnMut() + Send + Sync + 'static,
    {
        return RawFunc {
            func: Arc::new(func),
        };
    }

    pub fn invoke(mut self) {
        (Arc::get_mut(&mut self.func).unwrap())();
    }
}
