
use std::sync::{
    Arc,
};
use common::{
    Subscription,
    Observable,
    // get_mut,
};

struct Publisher<X, T> {
	val: Arc<X>,
	observers: Vec<Arc<T>>
}
impl<X, T : Subscription<X>+PartialEq+Clone> Publisher<X, T> {
	fn publish(&mut self, val: X) {
		self.val = Arc::new(val);
		self.notify_observers();
	}
}
impl<X, T: Subscription<X>+PartialEq+Clone> Observable<X, T> for Publisher<X, T> {
	fn add_observer(&mut self, observer: Arc<T>) {
		// println!("add_observer({});", observer);
		self.observers.push(observer);
	}
	fn delete_observer(&mut self, mut observer: Arc<T>) {
		let mut index = 0;
		let mut found = false;
		for obs in self.observers.iter() {
			if Arc::make_mut(&mut obs.clone()) == Arc::make_mut(&mut observer) {
				// println!("delete_observer({});", observer);
				found = true;
				break;
			}
			index += 1;
		}
		if found {
			self.observers.remove(index);
		}
	}
	fn notify_observers(&mut self) {
        let mut _observers = &mut self.observers;
        let val = self.val.clone();

        for (_, observer) in _observers.clone().iter_mut().enumerate() {
            // let observer = get_mut(observers, index).unwrap();
            let mut _observer = observer.clone();
            let observer = Arc::make_mut(&mut _observer);
            observer.on_next(val.clone());
        }
	}
}
