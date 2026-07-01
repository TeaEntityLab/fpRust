use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use fp_rust::actor::{Actor, ActorAsync, Handle};
use fp_rust::common::{LinkedListAsync, UniqueId};

#[derive(Clone, Debug)]
enum Value {
    Int(i32),
    VecStr(Vec<String>),
    Spawn,
    Shutdown,
}

fn main() {
    let result_i32 = LinkedListAsync::<i32>::new();
    let result_i32_thread = result_i32.clone();
    let result_string = LinkedListAsync::<Vec<String>>::new();
    let result_string_thread = result_string.clone();
    let mut root = ActorAsync::new(
        move |this: &mut ActorAsync<_, _>, msg: Value, context: &mut HashMap<String, Value>| {
            match msg {
                Value::Spawn => {
                    println!("Actor Spawn");
                    let result_i32_thread = result_i32_thread.clone();
                    let spawned = this.spawn_with_handle(Box::new(
                        move |this: &mut ActorAsync<_, _>, msg: Value, _| {
                            match msg {
                                Value::Int(v) => {
                                    println!("Actor Child Int");
                                    result_i32_thread.push_back(v * 10);
                                }
                                Value::Shutdown => {
                                    println!("Actor Child Shutdown");
                                    this.stop();
                                }
                                _ => {}
                            };
                        },
                    ));
                    let list = context.get("children_ids").cloned();
                    let mut list = match list {
                        Some(Value::VecStr(list)) => list,
                        _ => Vec::new(),
                    };
                    list.push(spawned.get_id());
                    context.insert("children_ids".into(), Value::VecStr(list));
                }
                Value::Shutdown => {
                    println!("Actor Shutdown");
                    if let Some(Value::VecStr(ids)) = context.get("children_ids") {
                        result_string_thread.push_back(ids.clone());
                    }

                    this.for_each_child(move |id, handle| {
                        println!("Actor Shutdown id {:?}", id);
                        handle.send(Value::Shutdown);
                    });
                    this.stop();
                }
                Value::Int(v) => {
                    println!("Actor Int");
                    if let Some(Value::VecStr(ids)) = context.get("children_ids") {
                        for id in ids {
                            println!("Actor Int id {:?}", id);
                            if let Some(mut handle) = this.get_handle_child(id) {
                                handle.send(Value::Int(v));
                            }
                        }
                    }
                }
                _ => {}
            }
        },
    );

    let mut root_handle = root.get_handle();
    root.start();

    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(10));
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(20));
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(30));
    root_handle.send(Value::Shutdown);

    let ids_deadline = Instant::now() + Duration::from_secs(5);
    let ids = loop {
        if let Some(v) = result_string.pop_front() {
            break Some(v);
        }
        if Instant::now() >= ids_deadline {
            break None;
        }
        thread::yield_now();
    };
    assert_eq!(Some(3), ids.map(|ids| ids.len()));

    let mut v = Vec::<Option<i32>>::new();
    let v_deadline = Instant::now() + Duration::from_secs(5);
    while v.len() < 6 {
        if let Some(i) = result_i32.pop_front() {
            v.push(Some(i));
        } else if Instant::now() >= v_deadline {
            break;
        } else {
            thread::yield_now();
        }
    }
    v.sort();
    assert_eq!(
        [
            Some(100),
            Some(200),
            Some(200),
            Some(300),
            Some(300),
            Some(300)
        ],
        v.as_slice()
    );
}
