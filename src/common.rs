use std::marker::PhantomData;

use std::sync::{
    Arc,
};

pub trait Subscription<X> {
    fn on_next(&mut self, x : X);
}

#[derive(Clone)]
pub struct SubscriptionFunc<T, F> {
    pub result : Arc<Option<T>>,
    pub receiver : RawReceiver<T, F>,
}

impl <T, F: FnMut(&mut Option<T>) + Send + Sync + 'static> SubscriptionFunc<T, F> {
    pub fn new(func: F) -> SubscriptionFunc<T, F> {
        return SubscriptionFunc {
            result : Arc::new(None),
            receiver: RawReceiver::new(func),
        };
    }
}

impl <T, F: FnMut(&mut Option<T>) + Send + Sync + 'static> Subscription<T> for SubscriptionFunc<T, F> {

    fn on_next(&mut self, x : T) {
        let mut val = Some(x);

        self.receiver.invoke(&mut val);

        let result = Arc::new(val);
        self.result = result.clone();

    }
}

#[derive(Clone)]
pub struct RawReceiver<T, F> {
    func: Arc<F>,
    _t: PhantomData<T>,
}

impl <T, F: FnMut(&mut Option<T>) + Send + Sync + 'static> RawReceiver<T, F> {
    pub fn new(func: F) -> RawReceiver<T, F> {
        return RawReceiver {
            func: Arc::new(func),
            _t: PhantomData,
        };
    }

    pub fn invoke(&mut self, x : &mut Option<T>) {
        (Arc::get_mut(&mut self.func).unwrap())(x);
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
