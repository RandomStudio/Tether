use log::{debug, error, info, warn};

use crate::{
    definitions::{InputPlugDefinition, OutputPlugDefinition, PlugDefinitionCommon},
    three_part_topic::ThreePartTopic,
    PlugDefinition, TetherAgent, TetherOrCustomTopic,
};

pub struct InputPlugOptions {
    plug_name: String,
    qos: Option<i32>,
    override_subscribe_role: Option<String>,
    override_subscribe_id: Option<String>,
    override_topic: Option<String>,
}

pub struct OutputPlugOptions {
    plug_name: String,
    qos: Option<i32>,
    override_publish_role: Option<String>,
    override_publish_id: Option<String>,
    override_topic: Option<String>,
    retain: Option<bool>,
}

/// This is the definition of an Input or Output Plug.
///
/// You should never use an instance of this directly; call `.build()` at the
/// end of the chain to get a usable PlugDefinition
pub enum PlugOptionsBuilder {
    InputPlugOptions(InputPlugOptions),
    OutputPlugOptions(OutputPlugOptions),
}

impl PlugOptionsBuilder {
    pub fn create_input(name: &str) -> PlugOptionsBuilder {
        PlugOptionsBuilder::InputPlugOptions(InputPlugOptions {
            plug_name: String::from(name),
            override_subscribe_id: None,
            override_subscribe_role: None,
            override_topic: None,
            qos: None,
        })
    }

    pub fn create_output(name: &str) -> PlugOptionsBuilder {
        PlugOptionsBuilder::OutputPlugOptions(OutputPlugOptions {
            plug_name: String::from(name),
            override_publish_id: None,
            override_publish_role: None,
            override_topic: None,
            qos: None,
            retain: None,
        })
    }

    pub fn qos(mut self, qos: i32) -> Self {
        match &mut self {
            PlugOptionsBuilder::InputPlugOptions(s) => s.qos = Some(qos),
            PlugOptionsBuilder::OutputPlugOptions(s) => s.qos = Some(qos),
        };
        self
    }

    /// Override the "role" part of the topic that gets generated for this Plug.
    /// If you override the entire topic using `.topic` this will be ignored.
    pub fn role(mut self, role: Option<&str>) -> Self {
        match &mut self {
            PlugOptionsBuilder::InputPlugOptions(s) => {
                if s.override_topic.is_some() {
                    error!("Override topic was also provided; this will take precedence");
                } else {
                    s.override_subscribe_role = role.and_then(|s| Some(String::from(s)));
                }
            }
            PlugOptionsBuilder::OutputPlugOptions(s) => {
                if s.override_topic.is_some() {
                    error!("Override topic was also provided; this will take precedence");
                } else {
                    s.override_publish_role = role.and_then(|s| Some(String::from(s)));
                }
            }
        };
        self
    }

    /// Override the "id" part of the topic that gets generated for this Plug.
    /// If you override the entire topic using `.topic` this will be ignored.
    pub fn id(mut self, id: Option<&str>) -> Self {
        match &mut self {
            PlugOptionsBuilder::InputPlugOptions(s) => {
                if s.override_topic.is_some() {
                    error!("Override topic was also provided; this will take precedence");
                } else {
                    s.override_subscribe_id = id.and_then(|s| Some(String::from(s)));
                }
            }
            PlugOptionsBuilder::OutputPlugOptions(s) => {
                if s.override_topic.is_some() {
                    error!("Override topic was also provided; this will take precedence");
                } else {
                    s.override_publish_id = id.and_then(|s| Some(String::from(s)));
                }
            }
        };
        self
    }

    /// Override the final topic to use for publishing or subscribing. The provided topic will be checked
    /// against the Tether Three Part Topic convention, but will not reject topic strings - just produce
    /// a warning. It's therefore valid to use a wildcard such as "#".
    ///
    /// Anything customised using `override_role` or `override_id` will be ignored if this function is called.
    pub fn topic(mut self, override_topic: &str) -> Self {
        if TryInto::<ThreePartTopic>::try_into(override_topic).is_ok() {
            info!("Custom topic passes Three Part Topic validation");
        } else {
            warn!(
                "Could not convert \"{}\" into Tether 3 Part Topic; presumably you know what you're doing!",
                override_topic
            );
        }
        match &mut self {
            PlugOptionsBuilder::InputPlugOptions(s) => {
                s.override_topic = Some(override_topic.into())
            }
            PlugOptionsBuilder::OutputPlugOptions(s) => {
                s.override_topic = Some(override_topic.into())
            }
        };
        self
    }

    pub fn retain(mut self, should_retain: bool) -> Self {
        match &mut self {
            Self::InputPlugOptions(_) => {
                error!("Cannot set retain flag on Input Plug / subscription");
            }
            Self::OutputPlugOptions(s) => {
                s.retain = Some(should_retain);
            }
        }
        self
    }

    /// Finalise the options (substituting suitable defaults if no custom values have been
    /// provided) and return a valid PlugDefinition that you can actually use.
    pub fn build(self, tether_agent: &TetherAgent) -> anyhow::Result<PlugDefinition> {
        match self {
            Self::InputPlugOptions(plug_options) => {
                let tpt: TetherOrCustomTopic = match plug_options.override_topic {
                    Some(custom) => TetherOrCustomTopic::Custom(custom),
                    None => TetherOrCustomTopic::Tether(ThreePartTopic::new_for_subscribe(
                        plug_options.override_subscribe_role,
                        plug_options.override_subscribe_id,
                        &plug_options.plug_name,
                    )),
                };
                let plug_definition =
                    InputPlugDefinition::new(&plug_options.plug_name, tpt, plug_options.qos);
                match tether_agent
                    .client()
                    .subscribe(plug_definition.topic().as_str(), plug_definition.qos())
                {
                    Ok(res) => {
                        debug!("This topic was fine: \"{}\"", plug_definition.topic());
                        debug!("Server respond OK for subscribe: {res:?}");
                        Ok(PlugDefinition::InputPlug(plug_definition))
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Self::OutputPlugOptions(plug_options) => {
                let tpt: TetherOrCustomTopic = match plug_options.override_topic {
                    Some(custom) => TetherOrCustomTopic::Custom(custom),
                    None => TetherOrCustomTopic::Tether(ThreePartTopic::new_for_publish(
                        plug_options.override_publish_role,
                        plug_options.override_publish_id,
                        &plug_options.plug_name,
                        tether_agent,
                    )),
                };

                let plug_definition = OutputPlugDefinition::new(
                    &plug_options.plug_name,
                    tpt,
                    plug_options.qos,
                    plug_options.retain,
                );
                Ok(PlugDefinition::OutputPlug(plug_definition))
            }
        }
    }
}
