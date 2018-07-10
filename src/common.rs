use std::marker::PhantomData;

use std::sync::{
    Arc,
};

pub trait Subscription<X> : Send + Sync + 'static {
    fn on_next(&mut self, x : X);
}

#[derive(Clone)]
pub struct SubscriptionFunc<T, F> {
    pub receiver : RawReceiver<T, F>,
}

impl <T : Send + Sync + 'static, F: FnMut(&mut Option<T>) + Send + Sync + 'static + Clone> SubscriptionFunc<T, F> {
    pub fn new(func: F) -> SubscriptionFunc<T, F> {
        return SubscriptionFunc {
            receiver: RawReceiver::new(func),
        };
    }
}

impl <T : Send + Sync + 'static, F: FnMut(&mut Option<T>) + Send + Sync + 'static + Clone> Subscription<T> for SubscriptionFunc<T, F> {

    fn on_next(&mut self, x : T) {
        let mut val = Some(x);

        self.receiver.invoke(&mut val);

    }
}

#[derive(Clone)]
pub struct RawReceiver<T, F> {
    func: Arc<F>,
    _t: PhantomData<T>,
}

impl <T, F: FnMut(&mut Option<T>) + Send + Sync + 'static + Clone> RawReceiver<T, F> {
    pub fn new(func: F) -> RawReceiver<T, F> {
        return RawReceiver {
            func: Arc::new(func),
            _t: PhantomData,
        };
    }

    pub fn invoke(&mut self, x : &mut Option<T>) {
        (Arc::make_mut(&mut self.func))(x);
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
