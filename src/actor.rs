/*!
In this module there're implementations & tests of `Actor`.
*/

use std::collections::{self, HashMap};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use super::common::{generate_id, UniqueId};
use super::sync::{BlockingQueue, Queue};

/**
`Actor` defines concepts of `Actor`: Send/Receive Messages, States, Methods.

# Arguments

* `Msg` - The generic type of Message data
* `ContextValue` - The generic type of ContextValue

# Remarks

It defines simple and practical hehaviors of `Actor` model.

``
*/
pub trait Actor<Msg, ContextValue, HandleType, Functor>: UniqueId<String> {
    fn receive(
        &mut self,
        this: &mut Self,
        message: Msg,
        context: &mut HashMap<String, ContextValue>,
    );
    fn spawn_with_handle(&self, func: Functor) -> HandleType;

    fn get_handle(&self) -> HandleType;
    fn get_handle_child(&self, name: impl Into<String>) -> Option<HandleType>;
    fn get_handle_parent(&self) -> Option<HandleType>;
}

pub trait Handle<Msg>: UniqueId<String> {
    fn send(&mut self, message: Msg);
}

#[derive(Debug, Clone)]
pub struct HandleAsync<Msg>
where
    Msg: Send + 'static,
{
    id: String,
    queue: BlockingQueue<Msg>,
}

impl<Msg> Handle<Msg> for HandleAsync<Msg>
where
    Msg: Send + 'static,
{
    fn send(&mut self, message: Msg) {
        self.queue.offer(message);
    }
}
impl<Msg> UniqueId<String> for HandleAsync<Msg>
where
    Msg: Send + 'static,
{
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

// #[derive(Clone)]
pub struct ActorAsync<Msg, ContextValue>
where
    Msg: Send + 'static,
{
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    id: String,
    parent_handle: Option<HandleAsync<Msg>>,
    children_handle_map: Arc<Mutex<HashMap<String, HandleAsync<Msg>>>>,

    context: Arc<Mutex<Box<HashMap<String, ContextValue>>>>,
    queue: BlockingQueue<Msg>,
    effect: Arc<
        Mutex<
            dyn FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
                + Send
                + Sync
                + 'static,
        >,
    >,

    join_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}
impl<Msg, ContextValue> Clone for ActorAsync<Msg, ContextValue>
where
    Msg: Clone + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            started_alive: self.started_alive.clone(),

            id: self.id.clone(),
            parent_handle: self.parent_handle.clone(),
            children_handle_map: self.children_handle_map.clone(),

            context: self.context.clone(),
            queue: self.queue.clone(),
            effect: self.effect.clone(),
            join_handle: self.join_handle.clone(),
        }
    }
}

impl<Msg, ContextValue> ActorAsync<Msg, ContextValue>
where
    Msg: Send + 'static,
{
    pub fn new(
        effect: impl FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self::new_with_options(effect, None, BlockingQueue::new())
    }

    pub fn new_with_options(
        effect: impl FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
            + Send
            + Sync
            + 'static,
        parent_handle: Option<HandleAsync<Msg>>,
        queue: BlockingQueue<Msg>,
    ) -> Self {
        Self {
            queue,
            parent_handle,
            id: generate_id(),
            children_handle_map: Arc::new(Mutex::new(HashMap::new())),
            context: Arc::new(Mutex::new(Box::new(HashMap::new()))),
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),
            join_handle: Arc::new(Mutex::new(None)),
            effect: Arc::new(Mutex::new(effect)),
        }
    }

    pub fn for_each_child(
        &self,
        mut func: impl FnMut(collections::hash_map::IterMut<String, HandleAsync<Msg>>),
    ) {
        func(self.children_handle_map.lock().unwrap().iter_mut());
    }

    pub fn is_started(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    pub fn is_alive(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;

            if !started.load(Ordering::SeqCst) {
                return;
            }
            if !alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(false, Ordering::SeqCst);
        }

        // NOTE: Kill thread <- OS depending
        // let mut join_handle = self.join_handle.lock().unwrap();
        // join_handle
        //     .take()
        //     .expect("Called stop on non-running thread")
        //     .join()
        //     .expect("Could not join spawned thread");
    }
}

impl<Msg, ContextValue> ActorAsync<Msg, ContextValue>
where
    Msg: Clone + Send + 'static,
    ContextValue: Send + 'static,
{
    pub fn start(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;

            if started.load(Ordering::SeqCst) {
                return;
            }
            started.store(true, Ordering::SeqCst);
            if alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(true, Ordering::SeqCst);
        }

        let mut this = self.clone();
        let mut this_for_receive = self.clone();
        let this_for_context = self.clone();
        let started_alive_thread = self.started_alive.clone();
        self.join_handle = Arc::new(Mutex::new(Some(thread::spawn(move || {
            while {
                let started_alive = started_alive_thread.lock().unwrap();
                let &(_, ref alive) = &*started_alive;

                alive.load(Ordering::SeqCst)
            } {
                let v = this.queue.take();

                match v {
                    Some(m) => {
                        let mut context = this_for_context.context.lock().unwrap();
                        this.receive(&mut this_for_receive, m, context.as_mut());
                    }
                    None => {
                        let started_alive = started_alive_thread.lock().unwrap();
                        let &(_, ref alive) = &*started_alive;

                        alive.store(false, Ordering::SeqCst);
                    }
                }
            }

            this.stop();
        }))));
    }
}

impl<Msg, ContextValue> UniqueId<String> for ActorAsync<Msg, ContextValue>
where
    Msg: Send + 'static,
{
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl<Msg, ContextValue>
    Actor<
        Msg,
        ContextValue,
        HandleAsync<Msg>,
        Box<
            dyn FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
                + Send
                + Sync
                + 'static,
        >,
    > for ActorAsync<Msg, ContextValue>
where
    Msg: Clone + Send + 'static,
    ContextValue: Send + 'static,
{
    fn receive(
        &mut self,
        this: &mut Self,
        message: Msg,
        context: &mut HashMap<String, ContextValue>,
    ) {
        {
            self.effect.lock().unwrap()(this, message, context);
        }
    }
    fn spawn_with_handle(
        &self,
        func: Box<
            dyn FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
                + Send
                + Sync
                + 'static,
        >,
    ) -> HandleAsync<Msg> {
        let mut new_one = Self::new(func);
        new_one.parent_handle = Some(self.get_handle());
        {
            self.children_handle_map
                .lock()
                .unwrap()
                .insert(new_one.get_id(), new_one.get_handle());
        }
        new_one.start();
        return new_one.get_handle();
    }
    fn get_handle(&self) -> HandleAsync<Msg> {
        HandleAsync {
            id: self.id.clone(),
            queue: self.queue.clone(),
        }
    }
    fn get_handle_child(&self, name: impl Into<String>) -> Option<HandleAsync<Msg>> {
        match self.children_handle_map.lock().unwrap().get(&name.into()) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }
    fn get_handle_parent(&self) -> Option<HandleAsync<Msg>> {
        return self.parent_handle.clone();
    }
}

#[test]
fn test_actor_common() {
    use std::time::Duration;

    use super::common::LinkedListAsync;

    #[derive(Clone, Debug)]
    enum Value {
        // Str(String),
        Int(i32),
        VecStr(Vec<String>),
        Spawn,
        Shutdown,
    }

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

                    this.for_each_child(move |iter| {
                        for (id, handle) in iter {
                            println!("Actor Shutdown id {:?}", id);
                            handle.send(Value::Shutdown);
                        }
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

    // One child
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(10));
    // Two children
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(20));
    // Three children
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(30));

    // Send Shutdown
    root_handle.send(Value::Shutdown);

    thread::sleep(Duration::from_millis(10));
    // 3 children Actors
    assert_eq!(3, result_string.pop_front().unwrap().len());

    let mut v = Vec::<Option<i32>>::new();
    for _ in 1..7 {
        let i = result_i32.pop_front();
        println!("Actor {:?}", i);
        v.push(i);
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
    )
}

#[test]
fn test_actor_ask() {
    use std::time::Duration;

    use super::common::LinkedListAsync;

    #[derive(Clone, Debug)]
    enum Value {
        AskIntByLinkedListAsync((i32, LinkedListAsync<i32>)),
        AskIntByBlockingQueue((i32, BlockingQueue<i32>)),
    }

    let mut root = ActorAsync::new(
        move |_: &mut ActorAsync<_, _>, msg: Value, _: &mut HashMap<String, Value>| match msg {
            Value::AskIntByLinkedListAsync(v) => {
                println!("Actor AskIntByLinkedListAsync");
                v.1.push_back(v.0 * 10);
            }
            Value::AskIntByBlockingQueue(mut v) => {
                println!("Actor AskIntByBlockingQueue");

                // NOTE If negative, hanging for testing timeout
                if v.0 < 0 {
                    return;
                }

                // NOTE General Cases
                v.1.offer(v.0 * 10);
            } // _ => {}
        },
    );

    let mut root_handle = root.get_handle();
    root.start();

    // LinkedListAsync<i32>
    let result_i32 = LinkedListAsync::<i32>::new();
    root_handle.send(Value::AskIntByLinkedListAsync((1, result_i32.clone())));
    root_handle.send(Value::AskIntByLinkedListAsync((2, result_i32.clone())));
    root_handle.send(Value::AskIntByLinkedListAsync((3, result_i32.clone())));
    thread::sleep(Duration::from_millis(5));
    let i = result_i32.pop_front();
    assert_eq!(Some(10), i);
    let i = result_i32.pop_front();
    assert_eq!(Some(20), i);
    let i = result_i32.pop_front();
    assert_eq!(Some(30), i);

    // BlockingQueue<i32>
    let mut result_i32 = BlockingQueue::<i32>::new();
    result_i32.timeout = Some(Duration::from_millis(1));
    root_handle.send(Value::AskIntByBlockingQueue((4, result_i32.clone())));
    root_handle.send(Value::AskIntByBlockingQueue((5, result_i32.clone())));
    root_handle.send(Value::AskIntByBlockingQueue((6, result_i32.clone())));
    thread::sleep(Duration::from_millis(5));
    let i = result_i32.take();
    assert_eq!(Some(40), i);
    let i = result_i32.take();
    assert_eq!(Some(50), i);
    let i = result_i32.take();
    assert_eq!(Some(60), i);

    // Timeout case:
    root_handle.send(Value::AskIntByBlockingQueue((-1, result_i32.clone())));
    let i = result_i32.take();
    assert_eq!(None, i);
}
