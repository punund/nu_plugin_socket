// Declare the modules that the compiler should look for.
// It will expect to find `src/connect.rs`, `src/listen.rs`, etc.
mod connect;
mod listen;

// Import the command structs from our modules.
use crate::connect::Connect;
use crate::listen::Listen;

use nu_plugin::{
    EngineInterface, EvaluatedCall, Plugin, PluginCommand,
};
use nu_protocol::{Category, LabeledError, PipelineData, Signature};

// The main struct that represents our plugin to Nushell.
// It must be public so that child modules can see it.
pub struct SocketPlugin;

impl Plugin for SocketPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    // This method is the central registry for all commands in the plugin.
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![
            // The parent command
            Box::new(Socket),
            // The subcommands
            Box::new(Connect),
            Box::new(Listen),
        ]
    }
}

// The parent "socket" command. It acts as a namespace for the subcommands.
struct Socket;

impl PluginCommand for Socket {
    type Plugin = SocketPlugin;

    fn name(&self) -> &str {
        "socket"
    }

    fn description(&self) -> &str {
        "A plugin for low-level socket communication."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Network)
    }

    fn extra_description(&self) -> &str {
        "Run `help socket connect` or `help socket listen` for more information."
    }

    // This runs if the user just types `socket` without a subcommand.
    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Err(LabeledError::new("Subcommand required")
            .with_help(
                "You must run a subcommand like 'connect' or 'listen'",
            )
            .with_label("subcommand missing here", call.head))
    }
}

// The main entry point of the executable.
// This starts the plugin and makes it available to Nushell.
fn main() {
    nu_plugin::serve_plugin(
        &mut SocketPlugin {},
        nu_plugin::MsgPackSerializer {},
    );
}
