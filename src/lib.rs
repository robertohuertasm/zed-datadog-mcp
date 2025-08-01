use std::env;

use schemars::JsonSchema;
use serde::Deserialize;
use zed_extension_api::{
    self as zed, Command, ContextServerConfiguration, ContextServerId, Project, Result, serde_json,
    settings::ContextServerSettings,
};

const PACKAGE_NAME: &str = "mcp-remote";
const PACKAGE_PATH: &str = "node_modules/mcp-remote/dist/proxy.js";

#[derive(Debug, Deserialize, JsonSchema)]
struct DatadogMcpServerConfiguration {
    #[serde(default)]
    site: Option<String>,
}

fn get_mcp_url(project: &Project) -> String {
    let site = ContextServerSettings::for_project("datadog-mcp", project)
        .ok()
        .and_then(|s| s.settings)
        .and_then(|s| serde_json::from_value::<DatadogMcpServerConfiguration>(s).ok())
        .and_then(|s| s.site)
        .unwrap_or_else(|| "US1".to_string())
        .to_uppercase();

    match site.as_str() {
        "US1" => "https://mcp.datadoghq.com/api/unstable/mcp-server/mcp".to_string(),
        "US3" => "https://mcp.us3.datadoghq.com/api/unstable/mcp-server/mcp".to_string(),
        "US5" => "https://mcp.us5.datadoghq.com/api/unstable/mcp-server/mcp".to_string(),
        "EU1" => "https://mcp.datadoghq.eu/api/unstable/mcp-server/mcp".to_string(),
        "AP1" => "https://mcp.ap1.datadoghq.com/api/unstable/mcp-server/mcp".to_string(),
        "AP2" => "https://mcp.ap2.datadoghq.com/api/unstable/mcp-server/mcp".to_string(),
        _ => format!("https://mcp.{site}.datadoghq.com/api/unstable/mcp-server/mcp"),
    }
}

struct DatadogMcpServer {}

impl zed::Extension for DatadogMcpServer {
    fn new() -> Self {
        Self {}
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        let latest_version = zed::npm_package_latest_version(PACKAGE_NAME)?;
        let version = zed::npm_package_installed_version(PACKAGE_NAME)?;
        if version.as_deref() != Some(latest_version.as_ref()) {
            zed::npm_install_package(PACKAGE_NAME, &latest_version)?;
        }

        let mcp_url = get_mcp_url(project);

        Ok(Command {
            command: zed::node_binary_path()?,
            args: vec![
                env::current_dir()
                    .unwrap()
                    .join(PACKAGE_PATH)
                    .to_string_lossy()
                    .to_string(),
                mcp_url,
            ],
            env: vec![],
        })
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Option<ContextServerConfiguration>> {
        let installation_instructions =
            include_str!("../configuration/installation_instructions.md").to_string();

        let settings = ContextServerSettings::for_project("datadog-mcp", project);

        let mut default_settings =
            include_str!("../configuration/default_settings.jsonc").to_string();

        if let Some(site) = settings
            .ok()
            .and_then(|s| s.settings)
            .and_then(|s| serde_json::from_value::<DatadogMcpServerConfiguration>(s).ok())
            .and_then(|s| s.site)
        {
            default_settings = default_settings.replace("\"US1\"", &format!("\"{site}\""));
        }

        let settings_schema =
            serde_json::to_string(&schemars::schema_for!(DatadogMcpServerConfiguration))
                .map_err(|e| e.to_string())?;

        Ok(Some(ContextServerConfiguration {
            installation_instructions,
            default_settings,
            settings_schema,
        }))
    }
}

zed::register_extension!(DatadogMcpServer);
