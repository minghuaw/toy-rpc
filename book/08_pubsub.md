# PubSub

A simple PubSub support is added in 0.8.0. A simple example can be found [here](https://github.com/minghuaw/toy-rpc/tree/main/examples/tokio_pubsub).

A publisher can be created on the server side or the client side using the `publisher::<T: Topic>()` method, and a subscriber can be created using the `subscriber::<T: Topic>(cap: usize)` method. They both take one type parameter `T` that must implements the `toy_rpc::pubsub::Topic` trait. You can use the provided derive macro `#[derive(toy_rpc::macros::Topic)]` to define a struct as the pubsub message or by manually implementing the `toy_rpc::pubsub::Topic` trait on a type.

```rust
use toy_rpc::macros::Topic;
use serde::{Serializer, Deserialize};

#[derive(Topic, Serialize, Deserialize)]
pub struct Count(pub u32);
```

Or manually implement the `Topic` trait

```rust,noplaypen
#[derive(Serialize, Deserialize)]
pub struct Count(pub u32);

impl toy_rpc::pubsub::Topic for Count {
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

## `Ack` of message delivery

As of version 0.8.0-beta.0, `Ack` is added in the cases where explicit `Ack` is needed. There are three different `AckMode`

  1. `AckModeNone`, which is the ***default*** mode for both the `Server` and `Client`. This mode is available on both the `Server` and the `Client` Under this mode, no `Ack` message will be required by the publisher or be sent by the subscriber.
  2. `AckModeAuto`. This mode is available on both the `Server` and `Client`. Under this mode, both the server and the client will automatically reply with an `Ack` to any `Publish` message they receive.
  3. `AckModeManual`. This mode is only available on `Client`. Under this mode, the subscriber needs to manually `.ack()` in order to get the published item. Please note that under the manual mode, the `Publisher` behaves the same as if it is under the `AckModeAuto` mode.


The behavior of publisher/subscriber will be discussed in different senarios below.

1. `Publisher` on the `Server` with `AckModeAuto`

    When a `Publisher` is created on the server side, the server's pubsub handler will wait for ***ALL*** `Ack`s from the subscribers, including that from `Subscriber` on the `Server`, in an asynchronous manner, meaning the publisher is able to continue publishing new messages even if some subscribers have not sent back `Ack` yet. Upon reaching the timeout, the server's pubsub handler will try to resend the same publish message (with the same sequence ID) to the `Subscriber`s that have not send back `Ack` messages. The server will stop retrying after the maximum number of retries is reached.
    
    

2. `Publisher` on the `Client` with `AckModeAuto` or `AckModeManual`

    When a `Publisher` is created on the client side, the client will wait for only ***ONE*** `Ack` message from the `Server` in an asynchronous manner, meaning the `Publisher` is able to continue publishing  new messages even if the `Ack` message from the `Server` has not arrived. If the `Ack` message from the `Server` does not arrive before the timeout expires, the client will attempt to publish the same message (with the same message ID). The client (`Publisher`) will stop retrying after the maximum number of retries is reached. 

    Once the `Publish` message is received by the `Server`, the message will be assigned a new sequence ID that is tracked only by the `Server`. The message will then be published to all subscribers under the topic, and the server will wait for ***ALL*** `Ack` messages from the subscribers in an asynchronous manner, meaning the server will be able to keep handling RPC requests or PubSub messages while waiting for `Ack` messages to come back. If not all `Ack` messages are sent back to the server before the timeout expires, the server will attempt to resend the same message with the same sequence ID number to the subscribers whose `Ack` messages are not received. The server will stop retrying after the maximum number of retries is reached.

3. `Subscriber` on the `Server` side with `AckModeAuto`

    
