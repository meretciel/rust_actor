#![allow(non_snake_case)]
#![allow(unused_parens)]
use std::borrow::Borrow;

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};

pub mod message;
pub use message::{Message, UserMessage};

pub mod actor_reference;
pub use actor_reference::{ActorPath, ActorRef};
pub mod internal;
use internal::MessageWrapper;


type SharedRoutingData = Arc<RwLock<HashMap<ActorPath, ActorRef>>>;


pub struct Context<'a> {
    /// Expose the Sender type here is not a good idea.
    pub lastSender: Option<&'a Sender<MessageWrapper>>,
    pub actorPath: &'a ActorPath,
    pub actorName: String,
    pub routingData: SharedRoutingData,
}

impl<'a> Context<'a> {
    pub fn getLastSender(&self) -> Option<Sender<MessageWrapper>> {
        match self.lastSender {
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



pub trait ActorBehavior {
    /**
       Method that is invoked when the actor receives a message.
    */
    fn onReceive(&self, message: &Message, context: &Context);

    /// Method that is invoked when automatically after the actor is constructed.
    fn start(&self, context: &Context) {}
}




pub struct Actor<BoxedBehavior> {
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

impl<B> Actor<B> where B: ActorBehavior + 'static {
    fn new(name: String, sender: Sender<MessageWrapper>, receiver: Receiver<MessageWrapper>,
        behavior: B, actorPath: ActorPath, lastSender: Option<Sender<MessageWrapper>>, routingData: SharedRoutingData) -> Actor<Box<dyn ActorBehavior>> {
        Actor{
            name: name,
            sender: sender,
            receiver: receiver,
            behavior: Box::new(behavior),
            actorPath: actorPath,
            lastSender: lastSender,
            routingData: routingData}
    }
}

impl<B: ?Sized> Actor<Box<B>> where B: ActorBehavior {

    fn getContext(&self) -> Context {
        match &self.lastSender {
            Some(lastSender) => Context{
                lastSender: Some(lastSender),
                actorPath: &self.actorPath,
                actorName: self.name.clone(),
                routingData: Arc::clone(&self.routingData),
            },

            None => Context{
                lastSender: None,
                actorPath: &self.actorPath,
                actorName: self.name.clone(),
                routingData: Arc::clone(&self.routingData),
            },
        }
    }

    pub fn start(&self) {
        let context = self.getContext();
        self.behavior.start(&context);
    }

    fn sendMsgToActorPath(&self, message: Message, actorPath: &ActorPath) -> Option<()> {
        let lockGuard = self.routingData.read().unwrap();
        let actorRef = lockGuard.get(actorPath)?;
        let _ = actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()));
        Some(())
    }

    fn sendMsgToActorRef(&self, message: Message, actorRef: &ActorRef) -> Option<()> {
        let _ = actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()));
        Some(())
    }

    pub fn getActorRef(&self) -> ActorRef {
        return ActorRef{ sender: self.sender.clone() };
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
        for actor in self.actors.iter() {
            actor.start();
        }
    }

    /**
        Crates an actor in the thread of the actor system.
    */
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
        // let v = self.actors.get(0).unwrap().routingData.read().unwrap();
        // let v = v.len();
        // println!("routing data actor: {}", v);
    }
}






