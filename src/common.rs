
pub trait Subscription<X> {
    fn on_next(&mut self, x : X);
}
