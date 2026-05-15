use serde::{Deserialize, Serialize};

use crate::message::Message;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scenario {
    pub messages: Vec<Message>,
}

impl Scenario {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, msg: Message) {
        self.messages.push(msg);
    }
}
