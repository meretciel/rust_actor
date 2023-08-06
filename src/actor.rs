#![allow(non_snake_case)]
#![allow(unused_parens)]
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

pub mod message;
pub use message::{Message, UserMessage};

pub mod actor_reference;
pub use actor_reference::{ActorPath, ActorRef};
pub mod internal;
use internal::MessageWrapper;
use std::borrow::Borrow;
use std::thread::JoinHandle;


type SharedRoutingData = Arc<RwLock<HashMap<ActorPath, ActorRef>>>;


pub struct Context {
    /// Expose the Sender type here is not a good idea.
    pub lastSender: Option<Sender<MessageWrapper>>,
    pub actorPath: ActorPath,
    pub actorName: String,
    pub routingData: SharedRoutingData,
}

impl Context {
    pub fn getLastSender(&self) -> Option<Sender<MessageWrapper>> {
        match &self.lastSender {
            Some(lastSender) => Some(lastSender.clone()),
            None => None,
        }
    }

    pub fn getActorName(&self) -> &String {
        return &self.actorName
    }

    pub fn resolvePathStr(&self, actorPathStr: String) -> Option<ActorRef> {
        let lockGuard = self.routingData.read().unwrap();
        let v = lockGuard.get(&ActorPath::from(&actorPathStr));
        match v {
            Some(&ref actorRef) => Some(ActorRef::clone(&actorRef)),
            None => None,
        }
    }

    pub fn reply(&self, message: Message) {
        let lastSender = self.getLastSender().unwrap();
        lastSender.send(MessageWrapper(message, self.actorPath.clone()))
            .expect("Failed to send message.");
    }

    // pub fn sendMsg(&self, message: Message, actorPath: &ActorPath) {
    //     let lockGuard = self.routingData.read().unwrap();
    //     let v = lockGuard.get(actorPath);
    //     if let Some(&ref actorRef) = v {
    //         actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()))
    //             .expect("Failed to send message.");
    //     }
    // }

    pub fn sendMsg(&self, message: Message, actorRef: &ActorRef) -> Option<()> {
        println!("sending an message from {}", self.actorName);
        let _ = actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()));
        Some(())
    }
}



pub trait ActorBehavior: Send {
    /**
       Method that is invoked when the actor receives a message.
    */
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


impl<B> ActorInner<Box<B>> where B: ?Sized + ActorBehavior + Send {
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

pub struct Actor<BoxedBehavior> {
    inner: Arc<Mutex<ActorInner<BoxedBehavior>>>
}

unsafe impl<B> Send for ActorInner<B> {}
unsafe impl<B> Sync for ActorInner<B> {}

impl<B> Actor<B> where B: ActorBehavior + Send + 'static {
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

impl<B> Actor<Box<B>> where B: ?Sized + ActorBehavior + Send + 'static {

    // fn getContext(&self) -> Context {
    //     let inner = self.inner.lock().unwrap();
    //
    //     match &inner.lastSender {
    //         Some(lastSender) => Context{
    //             lastSender: Some(lastSender.clone()),
    //             actorPath: inner.actorPath.clone(),
    //             actorName: inner.name.clone(),
    //             routingData: Arc::clone(&inner.routingData),
    //         },
    //
    //         None => Context{
    //             lastSender: None,
    //             actorPath: inner.actorPath.clone(),
    //             actorName: inner.name.clone(),
    //             routingData: Arc::clone(&inner.routingData),
    //         },
    //     }
    // }

    /// Run the actor.
    /// The actor process received messages in this function.
    fn run(&self) -> JoinHandle<()> {
        let mut clonedSelf = self.inner.clone();
        return thread::spawn(move || {
            println!("The actor start to run.");
            loop {
                let mut inner = clonedSelf.lock().unwrap();
                println!("actor {} is in the loop", inner.name);
                let msg = inner.receiver.recv().unwrap();
                let message = msg.0;
                let senderActorPath = msg.1;
                let lg = inner.routingData.read().unwrap();
                let senderActorRefOp = lg.get(&senderActorPath);
                match senderActorRefOp {
                    Some(&ref senderActorRef) => {
                        // inner.lastSender = Some(senderActorRef.sender.clone());
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

    // fn sendMsgToActorPath(&self, message: Message, actorPath: &ActorPath) -> Option<()> {
    //     let lockGuard = self.routingData.read().unwrap();
    //     let actorRef = lockGuard.get(actorPath)?;
    //     let _ = actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()));
    //     Some(())
    // }
    //
    // fn sendMsgToActorRef(&self, message: Message, actorRef: &ActorRef) -> Option<()> {
    //     let _ = actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()));
    //     Some(())
    // }
    //
    pub fn getActorRef(&self) -> ActorRef {
        let inner = self.inner.lock().unwrap();
        return ActorRef{ sender: inner.sender.clone() };
    }
}

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

    /**
        Crates an actor in the thread of the actor system.
    */
    pub fn createActor<Behavior>(&mut self, name: String, behavior: Behavior)
    where Behavior: ActorBehavior + 'static + Send {
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






