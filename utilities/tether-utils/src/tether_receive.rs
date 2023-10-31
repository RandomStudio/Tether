use clap::Args;
use log::{debug, error, info, warn};
use tether_agent::{mqtt::Message, PlugOptionsBuilder, TetherAgent, TetherOrCustomTopic};

#[derive(Args)]
pub struct ReceiveOptions {
    /// Topic to subscribe; by default we subscribe to everything
    #[arg(long = "topic")]
    pub subscribe_topic: Option<String>,

    /// Specify a ROLE (instead of wildcard +)
    #[arg(long = "plug.role")]
    pub subscribe_role: Option<String>,

    /// Specify an ID (instead of wildcard +)
    #[arg(long = "plug.id")]
    pub subscribe_id: Option<String>,

    /// Specify a plug name for the topic (instead of wildcard +)
    #[arg(long = "plug.name")]
    pub subscribe_plug: Option<String>,
}

pub fn receive(
    options: &ReceiveOptions,
    tether_agent: &TetherAgent,
    on_message: fn(plug_name: String, message: Message, decoded: Option<String>),
) {
    info!("Tether Receive Utility");

    let input_def = {
        if options.subscribe_id.is_some()
            || options.subscribe_role.is_some()
            || options.subscribe_plug.is_some()
        {
            debug!(
                "TPT Overrides apply: {:?}, {:?}, {:?}",
                &options.subscribe_id, &options.subscribe_role, &options.subscribe_plug
            );
            PlugOptionsBuilder::create_input("all")
                .role(options.subscribe_role.as_deref())
                .id(options.subscribe_id.as_deref())
                .name(options.subscribe_plug.as_deref())
        } else {
            debug!(
                "Using custom override topic \"{:?}\"",
                &options.subscribe_topic
            );
            PlugOptionsBuilder::create_input("all")
                .topic(Some(options.subscribe_topic.as_deref().unwrap_or("#")))
        }
    };

    let input = input_def
        .build(tether_agent)
        .expect("failed to create input plug");

    info!("Subscribed to topic \"{}\" ...", input.topic());

    loop {
        let mut did_work = false;
        while let Some((topic, message)) = tether_agent.check_messages() {
            did_work = true;
            debug!("Received message on topic \"{}\"", message.topic());
            let plug_name = match topic {
                TetherOrCustomTopic::Custom(_) => String::from("unknown"),
                TetherOrCustomTopic::Tether(tpt) => String::from(tpt.plug_name()),
            };

            let bytes = message.payload();
            if bytes.is_empty() {
                debug!("Empty message payload");
                on_message(plug_name, message, None);
            } else if let Ok(value) = rmp_serde::from_slice::<rmpv::Value>(bytes) {
                let json = serde_json::to_string(&value).expect("failed to stringify JSON");
                debug!("Decoded MessagePack payload: {}", json);
                on_message(plug_name, message, Some(json));
            } else {
                debug!("Failed to decode MessagePack payload");
                if let Ok(s) = String::from_utf8(bytes.to_vec()) {
                    warn!("String representation of payload: \"{}\"", s);
                } else {
                    error!("Could not decode payload bytes as string, either");
                }
                on_message(plug_name, message, None);
            }
        }
        if !did_work {
            std::thread::sleep(std::time::Duration::from_micros(100)); //0.1 ms
        }
    }
}
