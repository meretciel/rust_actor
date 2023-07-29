use std::sync::mpsc::Sender;
use super::internal::MessageWrapper;

pub struct ActorRef {
    pub sender: Sender<MessageWrapper>
}

#[derive(Clone)]
#[derive(Eq, Hash, PartialEq)]
pub struct ActorPath(pub String);

/// This is the actor path. It seems that this should be the main identifier of the actor.
/// The problem with Rust is that passing object around is not so convenient.
///
/// Although technically the reference of an actor does not change that often unless we want to
/// implement the full lifecycle of actors. In the sense, a reference should be enough. Anyway,
/// for the initial implementation, use this object as a shortcut.
impl ActorPath {
    pub fn from(pathStr: &str) -> ActorPath {
        ActorPath(pathStr.to_string())
    }
}
