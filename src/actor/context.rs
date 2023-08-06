use std::sync::mpsc::Sender;
use super::internal::{MessageWrapper, SharedRoutingData};
use super::actor_reference::{ActorPath, ActorRef};
use super::message::Message;

/// The Context struct represents the necessary context when the actor process a message.
///
pub struct Context {
    /// Expose the Sender type here is not a good idea.
    pub lastSender: Option<Sender<MessageWrapper>>,

    /// The actor path of the current actor.
    pub actorPath: ActorPath,

    /// The name of the current actor.
    pub actorName: String,

    /// The shared routing data. This data is shared in multiple threads.
    /// Because it's shared across the threads, locking is required for the access, which can cause
    /// performance issue.
    pub routingData: SharedRoutingData,
}

/// Implementation of the Context struct.
///
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

    pub fn sendMsg(&self, message: Message, actorRef: &ActorRef) -> Option<()> {
        let _ = actorRef.sender.send(MessageWrapper(message, self.actorPath.clone()));
        Some(())
    }
}