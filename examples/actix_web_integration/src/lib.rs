use serde::{Serialize, Deserialize};
use toy_rpc::pubsub::Topic;

pub mod rpc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Count(pub u32);

impl Topic for Count {
    type Item = Count;

    fn topic() -> String {
        "Count".into()
    }
}