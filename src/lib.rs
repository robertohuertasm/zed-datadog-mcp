use std::env;

use schemars::JsonSchema;
use serde::Deserialize;
use zed_extension_api::{
    self as zed, Command, ContextServerConfiguration, ContextServerId, Project, Result, serde_json,
    settings::ContextServerSettings,
};

mod mcp_remote_patch;

const PACKAGE_NAME: &str = "mcp-remote";

#[derive(Debug, Deserialize, JsonSchema)]
struct DatadogMcpServerConfiguration {
    #[serde(default)]
    site: Option<String>,
    #[serde(default)]
    toolsets: Vec<String>,
}

fn get_mcp_url(project: &Project) -> String {
    let settings = ContextServerSettings::for_project("datadog-mcp", project)
        .ok()
        .and_then(|s| s.settings)
        .and_then(|s| serde_json::from_value::<DatadogMcpServerConfiguration>(s).ok());

    let site = settings
        .as_ref()
        .and_then(|s| s.site.clone())
        .unwrap_or_else(|| "US1".to_string())
        .to_uppercase();

    let mut query_params = vec!["referrer_ide=zed".to_string()];

    if let Some(toolsets) = settings
        .as_ref()
        .map(|s| {
            s.toolsets
                .iter()
                .map(|toolset| toolset.trim())
                .filter(|toolset| !toolset.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|toolsets| !toolsets.is_empty())
    {
        query_params.push(format!("toolsets={}", toolsets.join(",")));
    }

    let mcp_path = format!("api/unstable/mcp-server/mcp?{}", query_params.join("&"));

    match site.as_str() {
        "US1" => format!("https://mcp.datadoghq.com/{mcp_path}"),
        "US3" => format!("https://mcp.us3.datadoghq.com/{mcp_path}"),
        "US5" => format!("https://mcp.us5.datadoghq.com/{mcp_path}"),
        "EU1" => format!("https://mcp.datadoghq.eu/{mcp_path}"),
        "AP1" => format!("https://mcp.ap1.datadoghq.com/{mcp_path}"),
        "AP2" => format!("https://mcp.ap2.datadoghq.com/{mcp_path}"),
        _ => format!("https://mcp.{site}.datadoghq.com/{mcp_path}"),
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
        // let latest_version = zed::npm_package_latest_version(PACKAGE_NAME)?;
        // 0.1.31 has issues, so we need to pin to 0.1.30 for now
        let version = zed::npm_package_installed_version(PACKAGE_NAME)?;
        if version.as_deref() != Some("0.1.30") {
            zed::npm_install_package(PACKAGE_NAME, "0.1.30")?;
        }
        // if version.as_deref() != Some(latest_version.as_ref()) {
        //     zed::npm_install_package(PACKAGE_NAME, &latest_version)?;
        // }

        let current_dir = env::current_dir().unwrap();
        mcp_remote_patch::apply(&current_dir)?;

        let mcp_url = get_mcp_url(project);

        Ok(Command {
            command: zed::node_binary_path()?,
            args: vec![
                current_dir
                    .join(mcp_remote_patch::PACKAGE_PATH)
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

        if let Some(config) = settings
            .ok()
            .and_then(|s| s.settings)
            .and_then(|s| serde_json::from_value::<DatadogMcpServerConfiguration>(s).ok())
        {
            if let Some(site) = config.site {
                default_settings = default_settings.replace("\"US1\"", &format!("\"{site}\""));
            }

            if !config.toolsets.is_empty() {
                let toolsets = config
                    .toolsets
                    .iter()
                    .map(|toolset| format!("\"{toolset}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                default_settings = default_settings
                    .replace("\"toolsets\": []", &format!("\"toolsets\": [{toolsets}]"));
            }
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
