#![allow(non_snake_case)]
#![allow(unused_parens)]

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

pub mod message;
pub use message::{Message, UserMessage};

pub mod actor_reference;
pub use actor_reference::{ActorPath, ActorRef};
pub mod internal;
use internal::MessageWrapper;



pub struct Context<'a> {
    /// Expose the Sender type here is not a good idea.
    pub lastSender: Option<&'a Sender<MessageWrapper>>,

    /// Reference to the underlying actor system. A context is always associated with an ActorSystem
    /// because only ActorSystem can create a context.
    pub actorSystem: &'a ActorSystem<'a>,

    pub actorPath: &'a ActorPath,
    pub actorName: String,
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

    pub fn resolvePathStr(&self, actorPathStr: String) -> Option<&ActorRef> {
        self.actorSystem.pathToActorRef.get(&ActorPath::from(&actorPathStr))
    }

    pub fn reply(&self, message: Message) {
        let lastSender = self.getLastSender().unwrap();
        lastSender.send(MessageWrapper(message, self.actorPath.clone()))
            .expect("Failed to send message.");
    }

    pub fn sendMsg(&self, message: Message, actorPath: &ActorPath) {
        let actorRef = self.actorSystem.pathToActorRef.get(actorPath).unwrap();
        actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()))
            .expect("Failed to send message.");
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


pub struct Actor<'a, BoxedBehavior> {
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
    /// The attached actor system.
    pub actorSystem: Option<&'a ActorSystem<'a>>,
}

impl<'a, B> Actor<'a, B> where B: ActorBehavior + 'static {
    fn new(name: String, sender: Sender<MessageWrapper>, receiver: Receiver<MessageWrapper>,
        behavior: B, actorPath: ActorPath, lastSender: Option<Sender<MessageWrapper>>, actorSystem: Option<&'a ActorSystem<'a>>) -> Actor<'a, Box<dyn ActorBehavior>> {
        Actor{
            name: name,
            sender: sender,
            receiver: receiver,
            behavior: Box::new(behavior),
            actorPath: actorPath,
            lastSender: lastSender,
            actorSystem: actorSystem}
    }
}

impl<'a, B: ?Sized> Actor<'a, Box<B>> where B: ActorBehavior {

    fn getContext(&self) -> Context {
        let actorSystem = self.actorSystem.unwrap();

        match &self.lastSender {
            Some(lastSender) => Context{
                lastSender: Some(lastSender),
                actorSystem,
                actorPath: &self.actorPath,
                actorName: self.name.clone(),
            },

            None => Context{
                lastSender: None,
                actorSystem,
                actorPath: &self.actorPath,
                actorName: self.name.clone(),
            },
        }
    }

    pub fn start(&self) {
        let context = self.getContext();
        self.behavior.start(&context);
    }

    fn sendMsgToActorPath(&self, message: Message, actorPath: &ActorPath) -> Option<()> {
        let actorRef = self.actorSystem?.pathToActorRef.get(actorPath)?;
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



pub struct ActorSystem<'a>{
    rootPath: String,
    pathToActorRef: HashMap<ActorPath, ActorRef>,
    actors: Box<Vec<Box<Actor<'a, Box<dyn ActorBehavior>>>>>,
}


impl ActorSystem<'_> {
    pub fn create() -> Self {
        return ActorSystem{
            rootPath: String::from("/"),
            pathToActorRef: HashMap::new(),
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
        let pathStr = format!("{}/{}", self.rootPath, name);

        let (sender, receiver) = channel();
        // let actor = Actor{
        //     name: name,
        //     sender: sender,
        //     receiver: receiver,
        //     behavior: Box::new(behavior),
        //     actorPath: ActorPath::from(&pathStr),
        //     lastSender: None,
        //     actorSystem: Some(self)};
        let actor = Actor::new(name, sender, receiver, behavior, ActorPath::from(&pathStr), None, Some(self));
        self.pathToActorRef.insert(ActorPath(pathStr), actor.getActorRef());
        //
        self.actors.push(Box::new(actor));
    }
}






