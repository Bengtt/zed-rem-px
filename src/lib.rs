use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree};

struct CssRemExtension;

impl zed::Extension for CssRemExtension {
    fn new() -> Self {
        CssRemExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let path = worktree
            .which("cssrem-lsp")
            .ok_or_else(|| {
                "cssrem-lsp not found on PATH.\n\
                 Build it with: cargo install --path lsp\n\
                 (run from the cssrem repo root)"
                    .to_string()
            })?;

        Ok(Command {
            command: path,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(CssRemExtension);
