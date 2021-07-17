# PubSub

A simple PubSub support is added in 0.8.0. A simple example can be found [here](https://github.com/minghuaw/toy-rpc/tree/main/examples/tokio_pubsub).

A publisher can be created on the server side or the client side using the `publisher::<T: Topic>()` method, and a subscriber can be created using the `subscriber::<T: Topic>(cap: usize)` method. They both take one type parameter `T` that must implements the `toy_rpc::pubsub::Topic` trait. (TODO: add derive trait). 

A topic can be defined by implementing the `Topic` trait.

```rust,noplaypen
#[derive(Serialize, Deserialize)]
pub struct Count(pub u32);

impl Topic for Count {
    type Item = Count; // The Item type must implement `Serialize` and `Deserialize`

    // A String identifier for the topic. The user must ensure it is unique
    fn topic() -> String {
        "Count"
    }
}
```

A publisher can be created by specifying the topic in the type parameter.

```rust,noplaypen 
let publisher = client.publisher::<Count>(); // on client side
// let publisher = server.publisher::<Count>(); // on server side
```

The `Publisher` implements the `futures::Sink<T>` trait where `T` is the type parameter representing the topic. In order to publish message to the topic, the `futures::SinkExt` trait must be imported.

```rust,noplaypen  
use futures::SinkExt;

publisher.send(Count(7)).await.unwrap();
```

A subscriber can be created by specifying the topic in the type parameter and the capacity of its local buffer. Here we will create a subscriber on the client side listening to messages on the topic `Count` with a local capacity of 10.

```rust,noplaypen 
let subscriber = client.subscirber::<Count>(10).unwrap(); // on the client side
// let subscriber = server.subscriber::<Count>(10).unwrap(); // on the server side (except for `actix-web`)
```

The `Subscriber` implements the `futures::Stream<Item = Result<T, toy_rpc::Error>>` trait where `T` is the type parameter representing the topic. In order to process incoming messages, the `futures::StreamExt` trait must be imported.

```rust,noplaypen 
use futures::StreamExt;

if let Some(result) = subscriber.next().await {
    let item = result.unwrap(); // There could be errors recving incoming messages
    // do something with the item
}
```

## Example

[GitHub repo](https://github.com/minghuaw/toy-rpc/tree/main/examples/tokio_pubsub)