
pub trait Subscription<X> {
    fn on_next(&mut self, x : X);
}

pub struct RawFunc {
    data: Box<Fn() + Send + Sync + 'static>,
}

impl RawFunc {
    pub fn new<T>(data: T) -> RawFunc
    where
        T: Fn() + Send + Sync + 'static,
    {
        return RawFunc {
            data: Box::new(data),
        };
    }

    pub fn invoke(self) {
        (self.data)()
    }
}
