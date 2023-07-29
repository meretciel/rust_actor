#![allow(non_snake_case)]
#![allow(unused_parens)]

use std::thread;
use std::time::Duration;
use actor_framework::actor::{ActorBehavior, ActorSystem, ActorPath, Message, Context, UserMessage, };


struct AutoReply{}

impl ActorBehavior for AutoReply {

    fn onReceive(&self, message: &Message, context: &Context) {
        match message {
            Message::User(UserMessage::StringMessage(content)) => {
                println!("Received an message {:?}", content);
                thread::sleep(Duration::from_secs(5));
                context.reply(Message::User(UserMessage::StringMessage(String::from("hello from actor i1"))));
            },
            _ => (),
        };
    }

    fn start(&self, context: &Context) {
        if (context.getActorName() == "first-actor") {
            let destRef = context.resolvePathStr(String::from("second-actor"));
            if let Some(actorRef) = destRef {
                context.sendMsg(Message::User(UserMessage::StringMessage(String::from("hello from actor i1"))),
                                              &ActorPath::from("/second-actor"));
            }
        }
    }
}


fn main() {
    let mut actorSystem = ActorSystem::create();
    let autoReplyBehavior = AutoReply{};
    // Creates two auto reply actor.
    let firstActor = actorSystem.createActor(String::from("first-actor"), Box::new(AutoReply{}));
    let secondActor = actorSystem.createActor(String::from("second-actor"), Box::new(AutoReply{}));

    actorSystem.start();
}