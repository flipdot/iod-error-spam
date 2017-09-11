extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rumqtt;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::sync::{mpsc, Mutex};
use rumqtt::{MqttCallback, MqttClient, MqttOptions, QoS};

const MQTT_HOST: &str = "localhost:1883";
const ERR_TOPIC: &str = "errors";
const IRC_TOPIC: &str = "actors/all/flipbot_send";

#[derive(Serialize, Deserialize)]
struct ErrorMessage {
    origin: String,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct IrcMessage {
    content: String,
}

fn main() {
    env_logger::init().expect("logger initialized twice (somehow)!");

    // TODO: command line args

    let client_options = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker(MQTT_HOST);

    let (tx, rx) = mpsc::channel::<ErrorMessage>();
    let tx = Mutex::new(tx);

    let mq_cbs = MqttCallback::new().on_message(move |msg| {
        if msg.topic.as_str() != ERR_TOPIC {
            error!(
                "message callback executed with topic '{}' instead of '{}'!",
                msg.topic.as_str(),
                ERR_TOPIC
            );
            return;
        }

        match serde_json::from_slice(&msg.payload) {
            Ok(err_msg) => tx.lock()
                .expect("[tx] lock fail!?")
                .send(err_msg)
                .expect("[tx] channel closed, this should never happen!"),
            Err(e) => info!(
                "deserialization of message payload failed! \n\
                 serde error: {:?}\n\
                 payload:\n\
                 {}",
                e,
                String::from_utf8_lossy(&msg.payload)
            ),
        }
    });

    // TODO: Handle MQTT errors more gracefully

    let mut client = MqttClient::start(client_options, Some(mq_cbs)).expect("Coudn't start");
    client.subscribe(vec![(ERR_TOPIC, QoS::Level0)]).unwrap();

    loop {
        let err_msg = rx.recv()
            .expect("[rx] channel closed, this should never happen!");

        let irc_msg = IrcMessage {
            content: format!("Error in {}: {}", err_msg.origin, err_msg.message),
        };

        client
            .publish(
                IRC_TOPIC,
                QoS::Level0,
                serde_json::to_vec(&irc_msg).expect("serializing irc message payload failed!"),
            )
            .unwrap();
    }
}
