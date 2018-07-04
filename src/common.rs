
pub struct Subscription<X> {
    onNext : FnOnce(X),
}
