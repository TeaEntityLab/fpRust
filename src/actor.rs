//! Message-passing actors with optional parent/child handles.
//!
//! # Crate features
//!
//! Requires **`actor`** (implies **`sync`**). Built on
//! [`BlockingQueue`] mailboxes.
//!
//! # Shutdown behavior
//!
//! [`ActorAsync::stop`] sets the alive flag only. The consumer thread is **not**
//! joined; commented-out [`join`](std::thread::JoinHandle::join) code shows
//! join-based shutdown was deferred. A thread blocked in [`take`](crate::sync::BlockingQueue::take)
//! is not interrupted. README/examples that use `thread::sleep` after `stop` are
//! demo synchronization only—not a supported shutdown contract.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use super::common::{generate_id, UniqueId};
use super::sync::{BlockingQueue, Queue};

/// Actor model: mailbox delivery, shared context, and child spawning.
///
/// `HandleType` is typically [`HandleAsync`]; `Functor` is the child initializer.
pub trait Actor<Msg, ContextValue, HandleType, Functor>: UniqueId<String> {
    /// Dispatches one message (usually delegates to the user `effect`).
    fn receive(
        &mut self,
        this: &mut Self,
        message: Msg,
        context: &mut HashMap<String, ContextValue>,
    );
    /// Spawns a child actor and returns a handle to its mailbox.
    fn spawn_with_handle(&self, func: Functor) -> HandleType;

    /// Handle pointing at this actor's mailbox.
    fn get_handle(&self) -> HandleType;
    /// Lookup a child handle by id string.
    fn get_handle_child(&self, name: impl Into<String>) -> Option<HandleType>;
    /// Parent handle, if any.
    fn get_handle_parent(&self) -> Option<HandleType>;

    /// Iterates registered child handles.
    fn for_each_child(&self, func: impl FnMut(&String, &mut HandleType));
}

/// Send-only mailbox endpoint for actor messages.
pub trait Handle<Msg>: UniqueId<String> {
    /// Enqueues a message (non-blocking [`offer`](crate::sync::BlockingQueue::offer)).
    fn send(&mut self, message: Msg);
}

/// [`Handle`] backed by a shared [`BlockingQueue`].
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

/// Default [`Actor`] implementation: one thread draining a mailbox.
///
/// Cloning shares mailboxes and lifecycle atomics; use [`ActorAsync::get_handle`] for senders.
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
    /// Root actor with a fresh mailbox and empty context.
    pub fn new(
        effect: impl FnMut(&mut ActorAsync<Msg, ContextValue>, Msg, &mut HashMap<String, ContextValue>)
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self::new_with_options(effect, None, BlockingQueue::new())
    }

    /// Actor with explicit parent link and mailbox (used for children).
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

    /// See [`ActorAsync`] module docs for lifecycle semantics.
    pub fn is_started(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let (started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    /// `true` while the mailbox thread may still run.
    pub fn is_alive(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let (_, alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    /// Clears alive flag only; does not join the mailbox thread (see module docs).
    pub fn stop(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
            let (started, alive) = &*started_alive;

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
    /// Spawns the mailbox consumer thread (idempotent if already started+alive).
    /// Start the mailbox loop on a background thread (idempotent).
    pub fn start(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
            let (started, alive) = &*started_alive;

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
                let (_, alive) = &*started_alive;

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
                        let (_, alive) = &*started_alive;

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
    /// Dispatch `message` to the user `effect` with the shared context map.
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
    /// Spawn a child actor, register its handle, and `start` it immediately.
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
        new_one.get_handle()
    }
    /// Handle that shares this actor's id and mailbox.
    fn get_handle(&self) -> HandleAsync<Msg> {
        HandleAsync {
            id: self.id.clone(),
            queue: self.queue.clone(),
        }
    }
    fn get_handle_child(&self, name: impl Into<String>) -> Option<HandleAsync<Msg>> {
        self.children_handle_map
            .lock()
            .unwrap()
            .get(&name.into())
            .cloned()
    }
    fn get_handle_parent(&self) -> Option<HandleAsync<Msg>> {
        self.parent_handle.clone()
    }

    fn for_each_child(&self, mut func: impl FnMut(&String, &mut HandleAsync<Msg>)) {
        for (id, handle) in self.children_handle_map.lock().unwrap().iter_mut() {
            func(id, handle);
        }
    }
}

#[test]
fn test_actor_common() {
    use std::time::{Duration, Instant};

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

    // result_string receives the children-ids vec once the root handles Shutdown;
    // the 3 child actors push 6 ints across their own threads. Wait deterministically
    // rather than racing a fixed sleep (the 5s ceilings only bound a genuine hang).
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
    // 3 children Actors
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
    )
}

#[test]
fn test_actor_ask() {
    use std::time::{Duration, Instant};

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

    // LinkedListAsync<i32> exposes only a non-blocking pop_front(), so wait-poll
    // until the actor has pushed each value. This replaces a fixed sleep with a
    // deterministic wait; the deadline only guards against a genuine hang.
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

    // BlockingQueue<i32>: take() with a timeout blocks until the actor offers,
    // so the data path is deterministic under load. The generous 5s ceiling only
    // bounds a genuine hang rather than racing a fixed sleep.
    let mut result_i32 = BlockingQueue::<i32>::new();
    result_i32.timeout = Some(Duration::from_secs(5));
    root_handle.send(Value::AskIntByBlockingQueue((4, result_i32.clone())));
    root_handle.send(Value::AskIntByBlockingQueue((5, result_i32.clone())));
    root_handle.send(Value::AskIntByBlockingQueue((6, result_i32.clone())));
    assert_eq!(Some(40), result_i32.take());
    assert_eq!(Some(50), result_i32.take());
    assert_eq!(Some(60), result_i32.take());

    // Timeout case: the actor returns early for negatives (never offers), so a
    // short timeout makes take() return None deterministically.
    result_i32.timeout = Some(Duration::from_millis(1));
    root_handle.send(Value::AskIntByBlockingQueue((-1, result_i32.clone())));
    assert_eq!(None, result_i32.take());
}

#[test]
fn test_actor_receives_in_order() {
    use super::common::LinkedListAsync;
    use super::sync::CountDownLatch;

    let sink = LinkedListAsync::<i32>::new();
    let sink_t = sink.clone();
    let latch = CountDownLatch::new(3);
    let latch_t = latch.clone();

    let mut actor = ActorAsync::new(
        move |_this: &mut ActorAsync<_, _>, msg: i32, _ctx: &mut HashMap<String, i32>| {
            sink_t.push_back(msg * 2);
            latch_t.countdown();
        },
    );
    let mut handle = actor.get_handle();
    actor.start();

    handle.send(1);
    handle.send(2);
    handle.send(3);
    latch.wait();

    // A single consumer thread drains the queue in FIFO order.
    assert_eq!(Some(2), sink.pop_front());
    assert_eq!(Some(4), sink.pop_front());
    assert_eq!(Some(6), sink.pop_front());
}

#[test]
fn test_actor_handle_id_and_no_parent() {
    let actor = ActorAsync::new(
        |_this: &mut ActorAsync<i32, i32>, _msg: i32, _ctx: &mut HashMap<String, i32>| {},
    );
    let handle = actor.get_handle();

    // The handle shares the actor's identity.
    assert_eq!(actor.get_id(), handle.get_id());
    // A root actor has no parent and no children yet.
    assert_eq!(true, actor.get_handle_parent().is_none());
    assert_eq!(true, actor.get_handle_child("missing").is_none());
}

#[test]
fn test_actor_context_accumulates_state() {
    use super::common::LinkedListAsync;
    use super::sync::CountDownLatch;

    #[derive(Clone)]
    enum Msg {
        Add(i32),
        Report,
    }

    let sink = LinkedListAsync::<i32>::new();
    let sink_t = sink.clone();
    let latch = CountDownLatch::new(1);
    let latch_t = latch.clone();

    let mut actor = ActorAsync::new(
        move |_this: &mut ActorAsync<_, _>, msg: Msg, ctx: &mut HashMap<String, i32>| match msg {
            Msg::Add(v) => {
                let cur = ctx.get("sum").cloned().unwrap_or(0);
                ctx.insert("sum".into(), cur + v);
            }
            Msg::Report => {
                sink_t.push_back(ctx.get("sum").cloned().unwrap_or(0));
                latch_t.countdown();
            }
        },
    );
    let mut handle = actor.get_handle();
    actor.start();

    handle.send(Msg::Add(1));
    handle.send(Msg::Add(2));
    handle.send(Msg::Add(3));
    handle.send(Msg::Report);
    latch.wait();

    // Context persists across messages: 1 + 2 + 3.
    assert_eq!(Some(6), sink.pop_front());
}

#[test]
fn test_actor_spawn_child_and_forward() {
    use super::common::LinkedListAsync;
    use super::sync::CountDownLatch;

    #[derive(Clone)]
    enum Msg {
        Spawn,
        Forward(i32),
    }

    let sink = LinkedListAsync::<i32>::new();
    let latch = CountDownLatch::new(1);
    let sink_root = sink.clone();
    let latch_root = latch.clone();

    let mut root = ActorAsync::new(
        move |this: &mut ActorAsync<_, _>, msg: Msg, _ctx: &mut HashMap<String, Msg>| match msg {
            Msg::Spawn => {
                let sink_child = sink_root.clone();
                let latch_child = latch_root.clone();
                this.spawn_with_handle(Box::new(move |_this: &mut ActorAsync<_, _>, m: Msg, _| {
                    if let Msg::Forward(v) = m {
                        sink_child.push_back(v * 10);
                        latch_child.countdown();
                    }
                }));
            }
            Msg::Forward(v) => {
                // Spawn is processed before Forward (FIFO), so the child exists.
                this.for_each_child(move |_id, handle| {
                    handle.send(Msg::Forward(v));
                });
            }
        },
    );
    let mut root_handle = root.get_handle();
    root.start();

    root_handle.send(Msg::Spawn);
    root_handle.send(Msg::Forward(5));
    latch.wait();

    assert_eq!(Some(50), sink.pop_front());
}

#[test]
fn test_actor_lifecycle() {
    let mut actor = ActorAsync::new(
        |_this: &mut ActorAsync<i32, i32>, _msg: i32, _ctx: &mut HashMap<String, i32>| {},
    );
    assert_eq!(false, actor.is_started());
    assert_eq!(false, actor.is_alive());

    actor.start();
    assert_eq!(true, actor.is_started());
    assert_eq!(true, actor.is_alive());

    actor.stop();
    assert_eq!(false, actor.is_alive());
    // Once started, the started flag stays set even after stop.
    assert_eq!(true, actor.is_started());
}

#[test]
fn test_actor_double_start_is_safe() {
    use super::common::LinkedListAsync;
    use super::sync::CountDownLatch;

    let sink = LinkedListAsync::<i32>::new();
    let sink_t = sink.clone();
    let latch = CountDownLatch::new(1);
    let latch_t = latch.clone();

    let mut actor = ActorAsync::new(
        move |_this: &mut ActorAsync<_, _>, msg: i32, _ctx: &mut HashMap<String, i32>| {
            sink_t.push_back(msg);
            latch_t.countdown();
        },
    );
    let mut handle = actor.get_handle();
    actor.start();
    // Second start must be ignored (no second consumer racing the queue).
    actor.start();

    handle.send(42);
    latch.wait();
    assert_eq!(Some(42), sink.pop_front());
}
