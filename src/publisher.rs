use std::marker::PhantomData;
use std::sync::{
    Arc,
};
use common::{
    Subscription,
    Observable,
    // get_mut,
};
use handler::{
    Handler,
};

struct Publisher<X, T> {
	observers: Vec<Arc<T>>,

    sub_handler : Option<Arc<Handler>>,

    _x : PhantomData<X>,
}
impl<X, T : Subscription<X>> Publisher<X, T> {

    pub fn new() -> Publisher<X, T> {
        return Publisher {
            observers: vec!(),
            sub_handler: None,
            _x: PhantomData,
        }
    }

	fn publish(&mut self, val: X) {
		self.notify_observers(Arc::new(val));
	}

    pub fn subscribe_on(&mut self, h : Option<Arc<Handler + 'static>>) {
        self.sub_handler = h;
    }
}
impl<X, T: Subscription<X>> Observable<X, T> for Publisher<X, T> {
	fn add_observer(&mut self, observer: Arc<T>) {
		// println!("add_observer({});", observer);
		self.observers.push(observer);
	}
	fn delete_observer(&mut self, mut observer: Arc<T>) {
		for (index, obs) in self.observers.clone().iter().enumerate() {
			if Arc::make_mut(&mut obs.clone()) == Arc::make_mut(&mut observer) {
				// println!("delete_observer({});", observer);
				self.observers.remove(index);
                return;
			}
		}
	}
	fn notify_observers(&mut self, val : Arc<X>) {
        let mut _observers = &mut self.observers;

        for (_, observer) in _observers.clone().iter_mut().enumerate() {
            // let observer = get_mut(observers, index).unwrap();
            let mut _observer = observer.clone();
            let observer = Arc::make_mut(&mut _observer);
            observer.on_next(val.clone());
        }
	}
}
