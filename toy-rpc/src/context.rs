use std::sync::Arc;

use crate::message::MessageId;

pub struct Context {}

impl Context {
    pub fn id(&self) -> &MessageId {
        todo!()
    }

    pub fn current() -> Arc<Self> {
        todo!()
    }
}
