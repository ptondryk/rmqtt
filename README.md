# rmqtt

work in progress

## example

```
fn main() {
    match MqttSessionBuilder::new("test-client-01", "localhost:1883")
            .credentials("user", "password")
            .keep_alive(120)
            .connect() {
        Ok(ref mut mqtt_session) => {
            let qos: u8 = 0;
            mqtt_session.subscribe("testTopic1", qos);
            match mqtt_connection.await_new_message() {
                Ok(message) => {
                    println!("topic = {:?}, payload = {:?}", message.topic,
                        String::from_utf8(message.payload).unwrap());
                }, Err(error_message) => {
                    println!("{:?}", error_message);
                }
            }
        },
        Err(message) => {
            println!("Connection failed, cause: {:?}", message);
        }
    }
}
```
