#![allow(non_snake_case)]
#![allow(unused_parens)]
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

pub mod message;
pub use message::{Message, UserMessage};
pub mod context;
pub use context::Context;
pub mod actor_reference;
pub use actor_reference::{ActorPath, ActorRef};
pub mod internal;
use internal::{MessageWrapper, SharedRoutingData};
use std::thread::JoinHandle;

/// Trait that represents the actor behavior.
///
/// Note that this trait acquires that the Send trait is implemented because we need to send
/// the behavior object to a separate thread.
pub trait ActorBehavior: Send {

    /// Method that is invoked when the actor receives a message.
    fn onReceive(&self, message: &Message, context: &Context);

    /// Method that is invoked when automatically after the actor is constructed.
    fn start(&self, context: &Context) {}
}


pub struct ActorInner<BoxedBehavior> {
    /// The name of the actor.
    pub name: String,
    /// The sender that can send message to this actor.
    pub sender: Sender<MessageWrapper>,
    /// The receiver of the message.
    pub receiver: Receiver<MessageWrapper>,
    /// Actor behavior.
    pub behavior: BoxedBehavior,
    /// The actor path of this actor.
    pub actorPath: ActorPath,
    /// The last sender.
    pub lastSender: Option<Sender<MessageWrapper>>,
    /// The shared routing data.
    pub routingData: SharedRoutingData,
}


impl<B> ActorInner<Box<B>> where B: ?Sized + ActorBehavior {
    fn getContext(&self, lastSender: Option<&Sender<MessageWrapper>>) -> Context {
        match lastSender {
            Some(lastSender) => Context{
                lastSender: Some(lastSender.clone()),
                actorPath: self.actorPath.clone(),
                actorName: self.name.clone(),
                routingData: Arc::clone(&self.routingData),
            },

            None => Context{
                lastSender: None,
                actorPath: self.actorPath.clone(),
                actorName: self.name.clone(),
                routingData: Arc::clone(&self.routingData),
            },
        }
    }
}
/// Marks it's unsafe to send and share the ActorInner.
/// The cause of the issue is that in the ActorInner, we have the Sender<MessageWrapper> field.
/// Sharing the Sender in multiple threads is not safe. In our implementation, we are ok because
/// Once we move the ActorInner to the thread, we don't access it from the outside anymore.
/// However, the previous statement will not be verified by the compiler due to the following code
/// block.
unsafe impl<B> Send for ActorInner<B> {}
unsafe impl<B> Sync for ActorInner<B> {}


/// This is the actor class.
///
/// This is a thin wrapper of the ActorInner struct. It's needed because in the current implementation
/// we start the new thread within the Actor struct and the thread could out live the Actor object.
/// This makes sense, because the ActorInner instance is shared by the actor thread and the Actor
/// instance in the main thread.
pub struct Actor<BoxedBehavior> {
    inner: Arc<Mutex<ActorInner<BoxedBehavior>>>
}

impl<B> Actor<B> where B: ActorBehavior + 'static {
    /// This method is needed to explicitly specify the class.
    /// The problem we had is related to B and Box<B>.
    fn new(name: String, sender: Sender<MessageWrapper>, receiver: Receiver<MessageWrapper>,
        behavior: B, actorPath: ActorPath, lastSender: Option<Sender<MessageWrapper>>, routingData: SharedRoutingData) -> Actor<Box<dyn ActorBehavior>> {
        let inner: ActorInner<Box<dyn ActorBehavior>> = ActorInner{
            name: name,
            sender: sender,
            receiver: receiver,
            behavior: Box::new(behavior),
            actorPath: actorPath,
            lastSender: lastSender,
            routingData: routingData};

        Actor{inner: Arc::new(Mutex::new(inner))}
    }
}


/// The implementation of the Actor struct.
///     ?Sized: Here we have a dynamic behavior so the size of the Behavior cannot be determined
///             during compile time.
///     ActorBehavior: The type B needs to be a ActorBehavior.
///     'static: TODO: Note quire sure on this one. It seems it means the object that implements
///                    the trait only contains static reference. To some extent, this makes sense,
///                    because the behavior is just funtion and it should be static. We cannot have
///                    a function that can become invalid in the middle of the program execution.
impl<B> Actor<Box<B>> where B: ?Sized + ActorBehavior + 'static {
    /// Run the actor.
    /// The actor process received messages in this function.
    fn run(&self) -> JoinHandle<()> {
        let clonedSelf = self.inner.clone();
        return thread::spawn(move || {
            println!("The actor start to run.");
            loop {
                let inner = clonedSelf.lock().unwrap();
                println!("actor {} is in the loop", inner.name);
                let msg = inner.receiver.recv().unwrap();
                let message = msg.0;
                let senderActorPath = msg.1;
                let lg = inner.routingData.read().unwrap();
                let senderActorRefOp = lg.get(&senderActorPath);
                match senderActorRefOp {
                    Some(senderActorRef) => {
                        let context = inner.getContext(Some(&senderActorRef.sender));
                        inner.behavior.onReceive(&message, &context);
                    },
                    None => println!("Unknown sender."),
                }
            }
        });
    }

    /// Starts the actor in a separate thread.
    /// In the current implementation, each actor has its own thread..
    pub fn start(&self) {
        let inner = self.inner.lock().unwrap();
        let context = inner.getContext(None);
        inner.behavior.start(&context);
    }

    pub fn getActorRef(&self) -> ActorRef {
        let inner = self.inner.lock().unwrap();
        return ActorRef{ sender: inner.sender.clone() };
    }
}

/// Struct the represents the actor system.
/// The actor system tracks the shared data used by actors.
pub struct ActorSystem{
    rootPath: String,
    pathToActorRef: SharedRoutingData,
    actors: Box<Vec<Box<Actor<Box<dyn ActorBehavior>>>>>,
}


impl ActorSystem {
    pub fn create() -> Self {
        return ActorSystem{
            rootPath: String::from("/"),
            pathToActorRef: Arc::new(RwLock::new(HashMap::new())),
            actors: Box::new(vec![])};
    }

    pub fn start(&self) {
        let mut joinHandles = vec![];
        for actor in self.actors.iter() {
            actor.start();
            joinHandles.push(actor.run());
        }

        for item in joinHandles {
            item.join().expect("Thread failed.");
        }
    }

    /// Crates an actor in the thread of the actor system.
    /// Actor will be moved to a dedicated thread when the actor system starts them.
    pub fn createActor<Behavior>(&mut self, name: String, behavior: Behavior)
    where Behavior: ActorBehavior + 'static {
        let pathStr = format!("/{}", name);
        println!("Create actor {} with path {}", name, pathStr);

        let (sender, receiver) = channel();
        let actor = Actor::new(name, sender, receiver, behavior, ActorPath::from(&pathStr), None, Arc::clone(&self.pathToActorRef));
        let mut underlyingData = self.pathToActorRef.write().unwrap();
        underlyingData.insert(ActorPath(pathStr), actor.getActorRef());
        //
        self.actors.push(Box::new(actor));
        println!("routing data main: {}", underlyingData.len());
    }
}

