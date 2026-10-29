use crate::{Shared, deliver};
use pumpkin_plugin_api::command::{
    Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::wit::pumpkin::plugin::{
    permission::{Permission, PermissionDefault, PermissionLevel},
    text::TextComponent,
};
use pumpkin_plugin_api::{Context, Server};
use vivecraft_core::config::{self, Settings};

pub(crate) fn native_permission(legacy: &str) -> String {
    format!("Vivecraft Pumpkin:{legacy}")
}

struct Handler {
    shared: Shared,
    arguments: bool,
}
pub(crate) fn register(context: &Context, shared: Shared) -> Result<(), String> {
    for (index, node) in config::PERMISSIONS.iter().enumerate() {
        context.register_permission(&Permission {
            node: native_permission(node),
            description: "Vivecraft compatibility permission".into(),
            default: if index == 0 {
                PermissionDefault::Op(PermissionLevel::Two)
            } else {
                PermissionDefault::Deny
            },
            children: vec![],
        })?;
    }
    let command = Command::new(
        &config::COMMANDS
            .iter()
            .map(|s| (*s).into())
            .collect::<Vec<_>>(),
        "Vivecraft configuration (sessions preview)",
    )
    .execute(Handler {
        shared: shared.clone(),
        arguments: false,
    })
    .then(
        CommandNode::argument("arguments", &ArgumentType::String(StringType::Greedy)).execute(
            Handler {
                shared,
                arguments: true,
            },
        ),
    );
    context.register_command(command, "vivecraft.config");
    Ok(())
}
impl CommandHandler for Handler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !sender.has_permission(&server, "vivecraft.config") {
            return Err(CommandError::PermissionDenied);
        }
        let text = if self.arguments {
            match args.get_value("arguments") {
                Arg::Simple(s) | Arg::Msg(s) => s,
                _ => return Err(CommandError::InvalidConsumption(None)),
            }
        } else {
            "help".into()
        };
        let result = self.execute(&text, &sender, &server);
        match result {
            Ok(message) => {
                sender.send_message(TextComponent::text(&message));
                Ok(1)
            }
            Err(error) => Err(CommandError::CommandFailed(TextComponent::text(&error))),
        }
    }
}
impl Handler {
    fn execute(
        &self,
        text: &str,
        sender: &CommandSender,
        server: &Server,
    ) -> Result<String, String> {
        if text.len() > 32767 {
            return Err("command too long".into());
        }
        let text = text.trim();
        let (first, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        let rest = rest.trim_start();
        let (second, argument) = rest
            .split_once(char::is_whitespace)
            .map_or((rest, None), |(action, value)| (action, Some(value.trim())));
        if first == "help" {
            return Ok("vivecraft-server-config (aliases vive, vse): help | list [withVersion] | reload | import-yaml <staged.yml> | <setting> get/set/reset/add/remove [value]. Inactive settings are retained, NOT applied.".into());
        }
        if first == "list" {
            let snapshots = self
                .shared
                .lock()
                .map_err(|e| e.to_string())?
                .sessions
                .snapshots();
            return Ok(format!(
                "{} online; {} negotiated Vivecraft sessions\n{}",
                server.get_player_count(),
                snapshots.len(),
                snapshots
                    .iter()
                    .map(|s| format!(
                        "{:02x?}: {}{}",
                        s.id,
                        if s.vr { "VR" } else { "NONVR" },
                        if second == "withVersion" {
                            format!(" {} / protocol {}", s.client_version, s.network_version)
                        } else {
                            String::new()
                        }
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let (folder, settings) = {
            let state = self.shared.lock().map_err(|e| e.to_string())?;
            if !state.active {
                return Err("plugin is unloading".into());
            }
            (state.folder.clone(), state.sessions.settings().clone())
        };
        if first == "import-yaml" {
            if !sender.is_console()
                && !sender.has_permission_level(
                    pumpkin_plugin_api::wit::pumpkin::plugin::command::PermissionLevel::Two,
                )
            {
                return Err("YAML import requires an operator or console".into());
            }
            let warnings = config::import_file(&folder, second)?;
            return Ok(format!(
                "Created config.imported.toml; NOT activated. Original YAML and config.toml are unchanged. Review then deliberately activate it.\n{}",
                warnings.join("\n")
            ));
        }
        if second == "get" {
            let value = settings.get(first).ok_or("unknown setting")?;
            return Ok(format!(
                "{first} = {value}{}",
                if settings.active(first) {
                    ""
                } else {
                    " (inactive; retained only)"
                }
            ));
        }
        // Refuse concurrent mutations. No state lock is held over filesystem or host calls.
        let settings = {
            let mut state = self.shared.lock().map_err(|e| e.to_string())?;
            if state.config_busy {
                return Err("another configuration change is in progress".into());
            }
            state.config_busy = true;
            state.sessions.settings().clone()
        };
        let result = (|| {
            let next = if first == "reload" {
                Settings::parse(&config::read_bounded(&folder.join("config.toml"))?)?
            } else {
                settings.edit(first, second, argument)?
            };
            if first != "reload" {
                config::save_atomic(&folder, &next)?;
            }
            let effects = self
                .shared
                .lock()
                .map_err(|e| e.to_string())?
                .sessions
                .configure(next);
            deliver(server, &self.shared, effects);
            Ok(if first == "reload" {
                "Configuration reloaded. Inactive values remain inactive; no unsupported gameplay was enabled.".into()
            } else if settings.active(first) {
                format!("Updated {first}.")
            } else {
                format!(
                    "Stored {first} in inactive configuration only; feature is NOT enabled in this preview."
                )
            })
        })();
        if let Ok(mut state) = self.shared.lock() {
            state.config_busy = false;
        }
        result
    }
}
