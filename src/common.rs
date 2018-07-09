
use std::sync::{Arc};

pub trait Subscription<X> {
    fn on_next(&mut self, x : X);
}

#[derive(Clone)]
pub struct RawFunc {
    data: Arc<Fn() + Send + Sync + 'static>,
}

impl RawFunc {
    pub fn new<T>(data: T) -> RawFunc
    where
        T: Fn() + Send + Sync + 'static,
    {
        return RawFunc {
            data: Arc::new(data),
        };
    }

    pub fn invoke(self) {
        (self.data)()
    }
}
