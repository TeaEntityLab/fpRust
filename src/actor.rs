/*!
In this module there're implementations & tests of `Actor`.
*/

use std::collections::HashMap;
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
    fn receive(&mut self, message: Msg, context: &mut HashMap<String, ContextValue>);
    fn spawn_with_handle(&self, func: Functor) -> HandleType;

    fn get_handle(&self) -> HandleType;
    fn get_handle_child(&self, name: impl Into<String>) -> Option<HandleType>;
    fn get_handle_parent(&self) -> Option<HandleType>;
}

pub trait Handle<Msg, ContextValue>: UniqueId<String> {
    fn send(&self, message: Msg);
}

#[derive(Debug, Clone)]
pub struct HandleAsync<Msg>
where
    Msg: Send + 'static,
{
    id: String,
    queue: Arc<Mutex<BlockingQueue<Msg>>>,
}

impl<Msg, ContextValue> Handle<Msg, ContextValue> for HandleAsync<Msg>
where
    Msg: Send + 'static,
{
    fn send(&self, message: Msg) {
        self.queue.lock().unwrap().offer(message);
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
pub struct ActorAsync<Msg, ContextValue> {
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    id: String,
    parent_handle: Option<Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static>>,
    children_handle_map:
        Arc<Mutex<HashMap<String, Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static>>>>,

    context: Arc<Mutex<Box<HashMap<String, ContextValue>>>>,
    queue: Arc<Mutex<BlockingQueue<Msg>>>,
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
impl<Msg, ContextValue> Clone for ActorAsync<Msg, ContextValue> {
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

impl<Msg, ContextValue> ActorAsync<Msg, ContextValue> {
    pub fn new(
        effect: impl FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self::new_with_options(effect, None, Arc::new(Mutex::new(BlockingQueue::new())))
    }

    pub fn new_with_options(
        effect: impl FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
            + Send
            + Sync
            + 'static,
        parent_handle: Option<Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static>>,
        queue: Arc<Mutex<BlockingQueue<Msg>>>,
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

    pub fn is_started(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    pub fn is_alive(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        {
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
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
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
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
        let started_alive_thread = self.started_alive.clone();
        self.join_handle = Arc::new(Mutex::new(Some(thread::spawn(move || {
            let mut queue = { this.queue.lock().unwrap().clone() };

            while {
                let started_alive = started_alive_thread.lock().unwrap();
                let &(_, ref alive) = &*started_alive;

                alive.load(Ordering::SeqCst)
            } {
                let v = queue.take();

                match v {
                    Some(m) => {
                        this.receive(m, this.context.clone().lock().unwrap().as_mut());
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
        Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static>,
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
    fn receive(&mut self, message: Msg, context: &mut HashMap<String, ContextValue>) {
        {
            let effect = self.effect.clone();
            effect.lock().unwrap()(self, message, context);
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
    ) -> Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static> {
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
    fn get_handle(&self) -> Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static> {
        return Arc::new(HandleAsync {
            id: self.id.clone(),
            queue: self.queue.clone(),
        });
    }
    fn get_handle_child(
        &self,
        name: impl Into<String>,
    ) -> Option<Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static>> {
        match self.children_handle_map.lock().unwrap().get(&name.into()) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }
    fn get_handle_parent(
        &self,
    ) -> Option<Arc<dyn Handle<Msg, ContextValue> + Send + Sync + 'static>> {
        return self.parent_handle.clone();
    }
}

#[test]
fn test_actor_common() {
    // assert_eq!(false, Maybe::just(None::<bool>).or(false));

    #[derive(Clone, Debug)]
    enum Value {
        // Str(String),
        Int(i32),
        VecStr(Vec<String>),
        Spawn,
        Shutdown,
    }

    let mut queue = BlockingQueue::<i32>::new();
    let queue_thread = queue.clone();
    let mut root = ActorAsync::new(
        move |this: &mut ActorAsync<_, _>, msg: Value, context: &mut HashMap<String, Value>| {
            match msg {
                Value::Spawn => {
                    println!("Actor Spawn");
                    let mut queue_thread = queue_thread.clone();
                    let spawned = this.spawn_with_handle(Box::new(
                        move |this: &mut ActorAsync<_, _>, msg: Value, _| {
                            match msg {
                                Value::Int(v) => {
                                    println!("Actor Child Int");
                                    queue_thread.put(v * 10);
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
                        for id in ids {
                            println!("Actor Shutdown id {:?}", id);
                            if let Some(handle) = this.get_handle_child(id) {
                                handle.send(Value::Shutdown);
                            }
                        }
                    }
                    this.stop();
                }
                Value::Int(v) => {
                    println!("Actor Int");
                    if let Some(Value::VecStr(ids)) = context.get("children_ids") {
                        for id in ids {
                            println!("Actor Int id {:?}", id);
                            if let Some(handle) = this.get_handle_child(id) {
                                handle.send(Value::Int(v));
                            }
                        }
                    }
                }
                _ => {}
            }
        },
    );

    let root_handle = root.get_handle();
    root.start();

    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(10));
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(20));
    root_handle.send(Value::Spawn);
    root_handle.send(Value::Int(30));

    root_handle.send(Value::Shutdown);

    let mut v = Vec::<i32>::new();
    for _ in 1..7 {
        let i = queue.take().unwrap();
        println!("Actor {:?}", i);
        v.push(i);
    }
    v.sort();

    assert_eq!([100, 200, 200, 300, 300, 300], v.as_slice())
}
