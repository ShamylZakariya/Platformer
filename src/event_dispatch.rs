use crate::state::events::Event;

/// A Message to be sent to an Entity instance.
#[derive(Debug, Clone)]
pub struct Message {
    /// The entity that sent this message.
    /// If None, then the State sent the message.
    pub sender_entity_id: Option<u32>,

    /// The entity to which to route this Message.
    /// If None, the State will process the message
    pub recipient_entity_id: Option<u32>,
    /// The event payload describing whatever happened
    pub event: Event,
}

impl Message {
    /// Creates a message to be processed by the global handler
    pub fn entity_to_global(sender: u32, event: Event) -> Self {
        Message {
            sender_entity_id: Some(sender),
            recipient_entity_id: None,
            event,
        }
    }

    /// Creates a message to be routed from one entity to another
    pub fn entity_to_entity(sender: u32, recipient: u32, event: Event) -> Self {
        Message {
            sender_entity_id: Some(sender),
            recipient_entity_id: Some(recipient),
            event,
        }
    }

    pub fn global_to_entity(recipient: u32, event: Event) -> Self {
        Message {
            sender_entity_id: None,
            recipient_entity_id: Some(recipient),
            event,
        }
    }
}

pub trait MessageHandler {
    fn handle_message(&mut self, message: &Message);
}

pub struct Dispatcher {
    pub messages: Vec<Message>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Dispatcher { messages: vec![] }
    }
}

impl Dispatcher {
    pub fn enqueue(&mut self, message: Message) {
        self.messages.push(message);
    }

    // TODO: I would prefer dispatch to be a member fn, not static. But State owns
    // the dispatcher, and as such can't be a message handler too since
    pub fn dispatch(messages: &Vec<Message>, handler: &mut dyn MessageHandler) {
        for m in messages {
            handler.handle_message(m);
        }
    }

    /// Returns the current message buffer, and clears it.
    pub fn drain(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.messages)
    }
}
