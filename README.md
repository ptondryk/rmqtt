# rmqtt

Rust implementation of MQTT client.

`rmqtt` implements version 3.1.1 of the MQTT protocol. It offers a
synchronous, single-thread, blocking API.

## Usage

Following example shows how to connect to the mqtt broker,
subscribe to a topic and receive a message.

Add to `Cargo.toml` following lines:
```toml
[dependencies.rmqtt]
git = "https://github.com/ptondryk/rmqtt.git"
```

main.rs
```rust
extern crate rmqtt;

use rmqtt::*;

fn main() {
    match MqttSessionBuilder::new("test-client-01", "localhost:1883")
            .credentials("user", "password")
            .keep_alive(120)
            .connect() {
        Ok(ref mut mqtt_session) => {
            let qos: u8 = 0;
            mqtt_session.subscribe("test-topic-1", qos);
            match mqtt_session.await_new_message(None) {
                Ok(message) => {
                    println!("topic = {:?}, payload = {:?}", message.topic,
                        String::from_utf8(message.payload).unwrap());
                }, Err(error) => {
                    println!("No message received");
                }
            }
        },
        Err(error) => {
            println!("Connection failed");
        }
    }
}
```

You can find more examples in wiki.

https://github.com/ptondryk/rmqtt/wiki/examples

## Documentation

<http://ptondryk.github.io/rmqtt/rmqtt/>

## References

* <http://docs.oasis-open.org/mqtt/mqtt/v3.1.1/mqtt-v3.1.1.html>
* <https://www.rust-lang.org>
