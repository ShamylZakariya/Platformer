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
    fn new(sender: Option<u32>, recipient: Option<u32>, event: Event) -> Self {
        Message {
            sender_entity_id: sender,
            recipient_entity_id: recipient,
            event,
        }
    }

    pub fn is_broadcast(&self) -> bool {
        self.sender_entity_id.is_none() && self.recipient_entity_id.is_none()
    }
}

pub trait MessageHandler {
    fn handle_message(&mut self, message: &Message);
}

#[derive(Default)]
pub struct Dispatcher {
    messages: Vec<Message>,
}

impl Dispatcher {
    /// Sends a message to every active Entity, as well as the GameState
    pub fn broadcast(&mut self, event: Event) {
        self.messages.push(Message::new(None, None, event));
    }

    /// Sends a message to every active Entity, as well as the GameState
    pub fn entity_to_global(&mut self, sender: u32, event: Event) {
        self.messages.push(Message::new(Some(sender), None, event));
    }

    /// Sends a message from one entity to another
    pub fn entity_to_entity(&mut self, sender: u32, recipient: u32, event: Event) {
        self.messages
            .push(Message::new(Some(sender), Some(recipient), event));
    }

    /// Sends a message to a single Entity, with no sender. This is generally reserved
    /// for use by GameState since it has no entity id.
    pub fn global_to_entity(&mut self, recipient: u32, event: Event) {
        self.messages
            .push(Message::new(None, Some(recipient), event));
    }

    // TODO: I would prefer dispatch to be a member fn, not static. But GameState owns
    // the dispatcher, and as such can't be a message handler too since GameState's handle_message
    // implementation is necessarily mutating.
    pub fn dispatch(messages: &[Message], handler: &mut dyn MessageHandler) {
        for m in messages {
            handler.handle_message(m);
        }
    }

    /// Transfers ownership of message buffer to caller, and clears internal storage. This is meant
    /// to be used in partnership with
    pub fn drain(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.messages)
    }
}
