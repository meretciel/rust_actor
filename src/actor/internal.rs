use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::message::Message;
use super::actor_reference::{ActorPath, ActorRef};


/// This message wrapper is used for sending message around inside the actor system.
pub struct MessageWrapper(pub Message, pub ActorPath);

pub type SharedRoutingData = Arc<RwLock<HashMap<ActorPath, ActorRef>>>;