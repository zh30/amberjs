//! Zed extension that starts Beejs `bee lsp`.

mod command;

pub use command::{
    assemble_lsp_command, build_language_server_command, candidate_binary_names,
    resolve_bee_binary, LspLaunch, WhichLookup,
};

use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct BeejsLspExtension;

impl zed::Extension for BeejsLspExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let settings = zed::settings::LspSettings::for_worktree("bee-lsp", worktree).ok();
        let binary = settings.as_ref().and_then(|s| s.binary.as_ref());
        let configured = binary.and_then(|b| b.path.as_deref());
        let extra_owned = binary.and_then(|b| b.arguments.clone()).unwrap_or_default();
        let (os, _arch) = zed::current_platform();
        let windows = matches!(os, zed::Os::Windows);
        let which = |name: &str| worktree.which(name);
        let launch = build_language_server_command(WhichLookup {
            configured_path: configured,
            windows,
            which: &which,
            extra_args: &extra_owned,
        })?;
        let mut env = worktree.shell_env();
        if let Some(extra_env) = binary.and_then(|b| b.env.clone()) {
            env.extend(extra_env);
        }
        Ok(zed::Command {
            command: launch.command,
            args: launch.args,
            env,
        })
    }
}

zed::register_extension!(BeejsLspExtension);
