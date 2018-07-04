
pub type FnNext<X> = FnOnce(X);

pub struct Subscription<X> {
    pub onNext : FnNext<X>,
}
