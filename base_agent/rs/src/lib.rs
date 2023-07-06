use log::{debug, error, info, warn};
use mqtt::{server_response, Client, Message, MessageBuilder, Receiver};
pub use paho_mqtt as mqtt;
use rmp_serde::to_vec_named;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone)]
struct PlugOptionsCommon {
    pub name: String,
    pub topic: Option<String>,
    pub qos: Option<i32>,
}

impl PlugOptionsCommon {
    fn new(name: &str) -> Self {
        PlugOptionsCommon {
            name: name.into(),
            topic: None,
            qos: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlugDefinitionCommon {
    pub name: String,
    pub topic: String,
    pub qos: i32,
}

pub struct InputPlugOptions {
    common: PlugOptionsCommon,
}

pub struct OutputPlugOptions {
    common: PlugOptionsCommon,
    retain: Option<bool>,
}

/// This is the definition of an Input or Output Plug
/// You should never use this directly; call build()
/// to get a usable Plug
pub enum PlugOptionsBuilder {
    InputPlugOptions(InputPlugOptions),
    OutputPlugOptions(OutputPlugOptions),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InputPlugDefinition {
    common: PlugDefinitionCommon,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OutputPlugDefinition {
    common: PlugDefinitionCommon,
    retain: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PlugDefinition {
    InputPlugDefinition(InputPlugDefinition),
    OutputPlugDefinition(OutputPlugDefinition),
}

impl PlugDefinition {
    pub fn common(&self) -> &PlugDefinitionCommon {
        match self {
            PlugDefinition::InputPlugDefinition(plug) => &plug.common,
            PlugDefinition::OutputPlugDefinition(plug) => &plug.common,
        }
    }

    pub fn common_mut(&mut self) -> &mut PlugDefinitionCommon {
        match self {
            PlugDefinition::InputPlugDefinition(plug) => &mut plug.common,
            PlugDefinition::OutputPlugDefinition(plug) => &mut plug.common,
        }
    }

    pub fn name(&self) -> &str {
        &self.common().name
    }

    pub fn topic(&self) -> &str {
        &self.common().topic
    }
}

impl PlugOptionsBuilder {
    fn common(&mut self) -> &mut PlugOptionsCommon {
        match self {
            PlugOptionsBuilder::InputPlugOptions(plug) => &mut plug.common,
            PlugOptionsBuilder::OutputPlugOptions(plug) => &mut plug.common,
        }
    }

    pub fn create_input(name: &str) -> PlugOptionsBuilder {
        PlugOptionsBuilder::InputPlugOptions(InputPlugOptions {
            common: PlugOptionsCommon::new(name),
        })
    }

    pub fn create_output(name: &str) -> PlugOptionsBuilder {
        PlugOptionsBuilder::OutputPlugOptions(OutputPlugOptions {
            common: PlugOptionsCommon::new(name),
            retain: Some(false),
        })
    }

    pub fn qos(mut self, qos: i32) -> Self {
        self.common().qos = Some(qos);
        self
    }

    pub fn topic(mut self, override_topic: &str) -> Self {
        self.common().topic = Some(override_topic.into());
        self
    }

    pub fn retain(self, should_retain: bool) -> Self {
        match self {
            Self::InputPlugOptions(_) => {
                panic!("Cannot set retain flag on Input Plug / subscription")
            }
            Self::OutputPlugOptions(plug) => {
                PlugOptionsBuilder::OutputPlugOptions(OutputPlugOptions {
                    common: plug.common,
                    retain: Some(should_retain),
                })
            }
        }
    }

    pub fn build(self, tether_agent: &TetherAgent) -> anyhow::Result<PlugDefinition> {
        match self {
            Self::InputPlugOptions(plug) => {
                let final_topic = plug
                    .common
                    .topic
                    .unwrap_or(default_subscribe_topic(&plug.common.name));
                let final_qos = plug.common.qos.unwrap_or(1);
                debug!(
                    "Attempt to subscribe for plug named {} ...",
                    plug.common.name
                );
                match tether_agent.client.subscribe(&final_topic, final_qos) {
                    Ok(res) => {
                        debug!("This topic was fine: --{final_topic}--");
                        debug!("Server respond OK for subscribe: {res:?}");
                        Ok(PlugDefinition::InputPlugDefinition(InputPlugDefinition {
                            common: PlugDefinitionCommon {
                                name: plug.common.name,
                                topic: final_topic,
                                qos: final_qos,
                            },
                        }))
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Self::OutputPlugOptions(plug) => {
                let final_topic = plug.common.topic.unwrap_or(build_topic(
                    &tether_agent.role,
                    &tether_agent.id,
                    &plug.common.name,
                ));
                let final_qos = plug.common.qos.unwrap_or(1);
                // TODO: check valid topic before assuming OK?
                Ok(PlugDefinition::OutputPlugDefinition(OutputPlugDefinition {
                    common: PlugDefinitionCommon {
                        name: plug.common.name,
                        topic: final_topic,
                        qos: final_qos,
                    },
                    retain: plug.retain.unwrap_or(false),
                }))
            }
        }
    }
}

pub struct TetherAgent {
    role: String,
    id: String,
    client: Client,
    broker_uri: String,
    receiver: Receiver<Option<Message>>,
}

#[derive(Clone)]
pub struct TetherAgentOptionsBuilder {
    role: String,
    id: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    auto_connect: bool,
}

impl TetherAgentOptionsBuilder {
    /// Initialise Tether Options struct with default options; call other methods to customise.
    /// Call `build()` to get the actual TetherAgent instance (and connect automatically, by default)
    pub fn new(role: &str) -> Self {
        TetherAgentOptionsBuilder {
            role: String::from(role),
            id: None,
            host: None,
            port: None,
            username: None,
            password: None,
            auto_connect: true,
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn host(mut self, host: &str) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn username(mut self, username: &str) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn auto_connect(mut self, should_auto_connect: bool) -> Self {
        self.auto_connect = should_auto_connect;
        self
    }

    pub fn build(self) -> anyhow::Result<TetherAgent> {
        let broker_host = self.host.clone().unwrap_or("localhost".into());
        let broker_port = self.port.unwrap_or(1883);

        let broker_uri = format!("tcp://{broker_host}:{broker_port}");

        info!("Create connection for broker at {}", &broker_uri);

        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(broker_uri.clone())
            .client_id("")
            .finalize();

        // Create the client connection
        let client = mqtt::Client::new(create_opts).unwrap();

        // Initialize the consumer before connecting
        let receiver = client.start_consuming();

        let agent = TetherAgent {
            role: self.role.clone(),
            id: self.id.clone().unwrap_or("any".into()),
            client,
            broker_uri,
            receiver,
        };

        if self.auto_connect {
            match agent.connect(&self) {
                Ok(()) => Ok(agent),
                Err(e) => Err(e.into()),
            }
        } else {
            warn!("Auto-connect disabled; you must call .connect explicitly");
            Ok(agent)
        }
    }
}

impl TetherAgent {
    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Returns the Agent Role and ID (group)
    pub fn description(&self) -> (&str, &str) {
        (&self.role, &self.id)
    }

    /// Return the URI (protocol, IP address, port, path) that
    /// was used to connect to the MQTT broker
    pub fn broker_uri(&self) -> &str {
        &self.broker_uri
    }

    pub fn set_role(&mut self, role: &str) {
        self.role = role.into();
    }

    pub fn set_id(&mut self, id: &str) {
        self.id = id.into();
    }

    pub fn connect(&self, options: &TetherAgentOptionsBuilder) -> Result<(), mqtt::Error> {
        let username = options.clone().username.unwrap_or("tether".into());
        let password = options.clone().password.unwrap_or("sp_ceB0ss!".into());
        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .user_name(username)
            .password(password)
            .connect_timeout(Duration::from_secs(TIMEOUT_SECONDS))
            .keep_alive_interval(Duration::from_secs(TIMEOUT_SECONDS))
            // .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
            .clean_session(true)
            .finalize();

        // Make the connection to the broker
        info!("Connecting to the MQTT server...");

        match self.client.connect(conn_opts) {
            Ok(res) => {
                info!("Connected OK: {res:?}");
                Ok(())
            }
            Err(e) => {
                error!("Error connecting to the broker: {e:?}");
                // self.client.stop_consuming();
                // self.client.disconnect(None).expect("failed to disconnect");
                Err(e)
            }
        }
    }

    /// If a message is waiting return Plug Name, Message (String, Message)
    pub fn check_messages(&self) -> Option<(String, Message)> {
        if let Some(message) = self.receiver.try_iter().find_map(|m| m) {
            let topic = message.topic();
            if let Some(plug_name) = parse_plug_name(topic) {
                Some((String::from(plug_name), message))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Given a plug definition and a raw (u8 buffer) payload, generate a message
    /// on an appropriate topic and with the QOS specified in the Plug Definition
    pub fn publish(
        &self,
        plug_definition: &PlugDefinition,
        payload: Option<&[u8]>,
    ) -> anyhow::Result<()> {
        match plug_definition {
            PlugDefinition::InputPlugDefinition(_) => {
                panic!("You cannot publish using an Input Plug")
            }
            PlugDefinition::OutputPlugDefinition(definition) => {
                let PlugDefinitionCommon { topic, qos, .. } = &definition.common;
                let message = MessageBuilder::new()
                    .topic(topic)
                    .payload(payload.unwrap_or(&[]))
                    .retained(definition.retain)
                    .qos(*qos)
                    .finalize();
                if let Err(e) = self.client.publish(message) {
                    error!("Error publishing: {:?}", e);
                    Err(e.into())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Similar to `publish` but serializes the data automatically before sending
    pub fn encode_and_publish<T: Serialize>(
        &self,
        plug_definition: &PlugDefinition,
        data: T,
    ) -> anyhow::Result<()> {
        match to_vec_named(&data) {
            Ok(payload) => self.publish(plug_definition, Some(&payload)),
            Err(e) => {
                error!("Failed to encode: {e:?}");
                Err(e.into())
            }
        }
    }

    pub fn publish_raw(
        &self,
        topic: &str,
        payload: &[u8],
        qos: Option<i32>,
        retained: Option<bool>,
    ) -> anyhow::Result<()> {
        let message = MessageBuilder::new()
            .topic(topic)
            .payload(payload)
            .retained(retained.unwrap_or(false))
            .qos(qos.unwrap_or(1))
            .finalize();
        if let Err(e) = self.client.publish(message) {
            error!("Error publishing: {:?}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

pub fn parse_plug_name(topic: &str) -> Option<&str> {
    let parts: Vec<&str> = topic.split('/').collect();
    match parts.get(2) {
        Some(s) => Some(*s),
        None => None,
    }
}

pub fn parse_agent_id(topic: &str) -> Option<&str> {
    let parts: Vec<&str> = topic.split('/').collect();
    match parts.get(1) {
        Some(s) => Some(*s),
        None => None,
    }
}

pub fn parse_agent_role(topic: &str) -> Option<&str> {
    let parts: Vec<&str> = topic.split('/').collect();
    match parts.first() {
        Some(s) => Some(*s),
        None => None,
    }
}

pub fn build_topic(role: &str, id: &str, plug_name: &str) -> String {
    format!("{role}/{id}/{plug_name}")
}

pub fn default_subscribe_topic(plug_name: &str) -> String {
    format!("+/+/{plug_name}")
}
