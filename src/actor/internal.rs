use super::message::Message;
use super::actor_reference::ActorPath;

/// This message wrapper is used for sending message around inside the actor system.
pub struct MessageWrapper(pub Message, pub ActorPath);