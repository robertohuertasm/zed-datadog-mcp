use std::env;

use zed_extension_api::{
    self as zed, Command, ContextServerConfiguration, ContextServerId, Project, Result,
};

const PACKAGE_NAME: &str = "mcp-remote";
const PACKAGE_PATH: &str = "node_modules/mcp-remote/dist/proxy.js";

struct DatadogMcpServer {}

impl zed::Extension for DatadogMcpServer {
    fn new() -> Self {
        Self {}
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Command> {
        let latest_version = zed::npm_package_latest_version(PACKAGE_NAME)?;
        let version = zed::npm_package_installed_version(PACKAGE_NAME)?;
        if version.as_deref() != Some(latest_version.as_ref()) {
            zed::npm_install_package(PACKAGE_NAME, &latest_version)?;
        }

        Ok(Command {
            command: zed::node_binary_path()?,
            args: vec![
                env::current_dir()
                    .unwrap()
                    .join(PACKAGE_PATH)
                    .to_string_lossy()
                    .to_string(),
                "https://mcp.datadoghq.com/api/unstable/mcp-server/mcp".to_string(),
            ],
            env: vec![],
        })
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Option<ContextServerConfiguration>> {
        let installation_instructions =
            include_str!("../configuration/installation_instructions.md").to_string();
        let default_settings = include_str!("../configuration/default_settings.jsonc").to_string();

        Ok(Some(ContextServerConfiguration {
            installation_instructions,
            default_settings,
            settings_schema: "{}".to_string(),
        }))
    }
}

zed::register_extension!(DatadogMcpServer);
