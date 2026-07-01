use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use fp_rust::actor::{Actor, ActorAsync, Handle};
use fp_rust::common::LinkedListAsync;
use fp_rust::sync::{BlockingQueue, Queue};

#[derive(Clone, Debug)]
enum Value {
    AskIntByLinkedListAsync((i32, LinkedListAsync<i32>)),
    AskIntByBlockingQueue((i32, BlockingQueue<i32>)),
}

fn main() {
    let mut root = ActorAsync::new(
        move |_: &mut ActorAsync<_, _>, msg: Value, _: &mut HashMap<String, Value>| match msg {
            Value::AskIntByLinkedListAsync(v) => {
                println!("Actor AskIntByLinkedListAsync");
                v.1.push_back(v.0 * 10);
            }
            Value::AskIntByBlockingQueue(mut v) => {
                println!("Actor AskIntByBlockingQueue");

                if v.0 < 0 {
                    return;
                }

                v.1.offer(v.0 * 10);
            }
        },
    );

    let mut root_handle = root.get_handle();
    root.start();

    let wait_pop = |q: &LinkedListAsync<i32>| -> Option<i32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(v) = q.pop_front() {
                return Some(v);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::yield_now();
        }
    };

    let result_i32 = LinkedListAsync::<i32>::new();
    root_handle.send(Value::AskIntByLinkedListAsync((1, result_i32.clone())));
    root_handle.send(Value::AskIntByLinkedListAsync((2, result_i32.clone())));
    root_handle.send(Value::AskIntByLinkedListAsync((3, result_i32.clone())));
    assert_eq!(Some(10), wait_pop(&result_i32));
    assert_eq!(Some(20), wait_pop(&result_i32));
    assert_eq!(Some(30), wait_pop(&result_i32));

    let mut result_i32 = BlockingQueue::<i32>::new();
    result_i32.timeout = Some(Duration::from_secs(5));
    root_handle.send(Value::AskIntByBlockingQueue((4, result_i32.clone())));
    root_handle.send(Value::AskIntByBlockingQueue((5, result_i32.clone())));
    root_handle.send(Value::AskIntByBlockingQueue((6, result_i32.clone())));
    assert_eq!(Some(40), result_i32.take());
    assert_eq!(Some(50), result_i32.take());
    assert_eq!(Some(60), result_i32.take());

    result_i32.timeout = Some(Duration::from_millis(1));
    root_handle.send(Value::AskIntByBlockingQueue((-1, result_i32.clone())));
    assert_eq!(None, result_i32.take());
}
