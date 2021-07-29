use serde::{Serialize, Deserialize};

use toy_rpc::macros::Topic;

pub const ADDR: &str = "127.0.0.1:23333";

#[derive(Debug, Topic, Serialize, Deserialize)]
#[topic(rename="C", item="u32")]
// pub struct Count(pub u32);
pub struct Count { }

// impl Topic for Count {
//     type Item = Count;

//     fn topic() -> String {
//         "Count".into()
//     }
// }