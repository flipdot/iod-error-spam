extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate maplit;
extern crate rumqtt;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::collections::HashMap;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use rumqtt::{MqttCallback, MqttClient, MqttOptions, QoS};

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

    let matches = clap::App::new("iod-error-spam")
        .version("0.1.0")
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::with_name("HOST")
                .short("h")
                .long("host")
                .help("The host the MQTT broker runs on")
                .default_value("localhost"),
        )
        .arg(
            clap::Arg::with_name("PORT")
                .short("p")
                .long("port")
                .help("The port used by the MQTT broker")
                .default_value("1883"),
        )
        .get_matches();

    let client_options = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker(
            // unwrap is safe here because the arguments have default values.
            &format!(
                "{}:{}",
                matches.value_of("HOST").unwrap(),
                matches.value_of("PORT").unwrap(),
            ),
        );

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
                "deserialization of message payload failed!\
                 \n    serde error: {:?}\
                 \n    payload: {}",
                e,
                String::from_utf8_lossy(&msg.payload)
            ),
        }
    });

    // TODO: Handle MQTT errors more gracefully

    let mut client = MqttClient::start(client_options, Some(mq_cbs)).expect("Coudn't start");
    client.subscribe(vec![(ERR_TOPIC, QoS::Level0)]).unwrap();

    let rate_limits = hashmap!{
        // max 5 messages per minute
        Duration::from_secs(60) => 5,

        // max 20 messages per hour
        Duration::from_secs(60 * 60) => 20,
    };

    let mut sent_error_times = HashMap::new();

    'main: loop {
        let err_msg = rx.recv()
            .expect("[rx] channel closed, this should never happen!");

        let irc_msg = IrcMessage {
            content: format!("Error in {}: {}", err_msg.origin, err_msg.message),
        };

        let times = sent_error_times.entry(err_msg.origin).or_insert_with(|| Vec::new());
        let now = Instant::now();

        for (&duration, &limit) in &rate_limits {
            if times.iter().filter(|&&t| now - t <= duration).count() >= limit {
                continue 'main;
            }
        }

        client
            .publish(
                IRC_TOPIC,
                QoS::Level0,
                serde_json::to_vec(&irc_msg).expect("serializing irc message payload failed!"),
            )
            .unwrap();

        // Throw away error times that aren't relevant for any rate limit any more
        times.retain(|&t| now - t <= *rate_limits.keys().max().unwrap());

        // Add the time of the message we just published
        times.push(now);
    }
}
