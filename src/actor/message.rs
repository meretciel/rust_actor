#[derive(Debug)]
pub enum SystemMessage {
    StringMessage(String)
}

#[derive(Debug)]
pub enum UserMessage {
    StringMessage(String),
}

#[derive(Debug)]
pub enum Message {
    System(SystemMessage),
    User(UserMessage),
}

