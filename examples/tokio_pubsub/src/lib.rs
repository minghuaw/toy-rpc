use toy_rpc::pubsub::Topic;

pub const ADDR: &str = "127.0.0.1:23333";

pub struct Count(u32);

impl Topic for Count {
    type Item = u32;

    fn topic() -> String {
        "Count".into()
    }
}