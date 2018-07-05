
pub trait Subscription<X> {
    fn onNext(&mut self, x : X);
}
