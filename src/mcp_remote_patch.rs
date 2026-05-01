use std::{fs, path::Path};

use zed_extension_api::Result;

/// Entrypoint used by the extension to launch `mcp-remote`.
pub const PACKAGE_PATH: &str = "node_modules/mcp-remote/dist/proxy.js";

/// Internal bundled module used by `mcp-remote@0.1.30`.
///
/// Most of `mcp-remote`'s proxy implementation is bundled in this generated
/// chunk and imported by `proxy.js`. The message forwarding function we need to
/// patch lives here.
const PACKAGE_CHUNK_PATH: &str = "node_modules/mcp-remote/dist/chunk-RGTAVJIZ.js";

/// Marker used to make the message-delay patch idempotent.
const DELAY_PATCH_MARKER: &str = "__zedDatadogMcpReadyDelayV4";

/// Marker used to make the `params: null` normalization patch idempotent.
const PARAMS_NULL_PATCH_MARKER: &str = "__zedDatadogMcpParamsNullPatchV2";

/// Applies the compatibility patches required for `mcp-remote@0.1.30` to work
/// reliably as a Datadog MCP bridge from Zed.
///
/// Why patch at all?
///
/// The Zed extension API currently only supports context servers that are
/// launched as local commands. Zed's user settings support remote MCP servers
/// via a `url` field, but extensions cannot return a remote URL directly.
/// Therefore this extension must launch a local stdio bridge (`mcp-remote`) and
/// make it compatible with Zed + Datadog.
///
/// The two compatibility issues are:
///
/// 1. Zed may send JSON-RPC notifications with `params: null`. The MCP SDK
///    bundled in `mcp-remote@0.1.30` rejects those messages during stdio
///    parsing, even though treating null params as absent params is safe for
///    these notifications.
///
/// 2. Zed sends `notifications/initialized` and `tools/list` immediately after
///    `initialize`. Datadog's Streamable HTTP MCP endpoint can need a few
///    seconds before `tools/list` reliably returns tools through this proxy.
///
/// Long term, this logic should live in an upstream/forked `mcp-remote` rather
/// than being patched in-place. For now, patching the installed package keeps
/// the extension self-contained.
pub fn apply(extension_dir: &Path) -> Result<()> {
    let package_chunk_path = extension_dir.join(PACKAGE_CHUNK_PATH);
    let package_proxy_path = extension_dir.join(PACKAGE_PATH);

    patch_message_forwarding(&package_chunk_path)?;
    patch_params_null_normalization(&package_proxy_path)?;

    Ok(())
}

fn replace_existing_patch(contents: &mut String, patch_starts: &[&str], replacement: &str) -> bool {
    for patch_start in patch_starts {
        if let Some(start) = contents.find(patch_start) {
            if let Some(relative_end) = contents[start..].find("\n  };") {
                let end = start + relative_end + "\n  };".len();
                contents.replace_range(start..end, replacement);
                return true;
            }
        }
    }

    false
}

fn patch_message_forwarding(package_chunk_path: &Path) -> Result<()> {
    let mut contents = fs::read_to_string(package_chunk_path).map_err(|e| e.to_string())?;

    // Original forwarding hook inside `mcpProxy`. Without the patch, every Zed
    // message is forwarded to Datadog immediately.
    let send_original = "    transportToServer.send(message).catch(onServerError);\n  };";

    // Replacement forwarding hook. It records when `initialize` was sent and
    // delays early follow-up messages until Datadog is ready.
    let send_patch = r#"    const __zedDatadogMcpReadyDelayV4 = true;
    const __zedDatadogMcpReadyDelay = 2000;
    const __zedDatadogMcpToolsDelay = 2000;
    const __zedDatadogMcpSend = (m) => transportToServer.send(m).catch(onServerError);
    if (message.method === "initialize") {
      globalThis.__zedDatadogMcpInitializeAt = Date.now();
      __zedDatadogMcpSend(message);
    } else {
      const __zedDatadogMcpInitializeAt = globalThis.__zedDatadogMcpInitializeAt || 0;
      const __zedDatadogMcpDelay = message.method === "tools/list" ? __zedDatadogMcpToolsDelay : __zedDatadogMcpReadyDelay;
      const __zedDatadogMcpWaitMs = __zedDatadogMcpInitializeAt ? Math.max(0, __zedDatadogMcpInitializeAt + __zedDatadogMcpDelay - Date.now()) : 0;
      if (__zedDatadogMcpWaitMs > 0) {
        setTimeout(() => __zedDatadogMcpSend(message), __zedDatadogMcpWaitMs);
      } else {
        __zedDatadogMcpSend(message);
      }
    }
  };"#;

    if !contents.contains(DELAY_PATCH_MARKER)
        && !contents.contains(send_original)
        && !replace_existing_patch(
            &mut contents,
            &[
                "    const __zedDatadogMcpReadyDelayV3 = true;",
                "    const __zedDatadogMcpReadyDelayV2 = true;",
                "    const __zedDatadogMcpInitializeAt = globalThis.__zedDatadogMcpInitializeAt || 0;",
            ],
            send_patch,
        )
    {
        return Err("Could not patch mcp-remote: expected send hook was not found".to_string());
    }

    if contents.contains(send_original) {
        contents = contents.replace(send_original, send_patch);
    }

    fs::write(package_chunk_path, contents).map_err(|e| e.to_string())?;

    Ok(())
}

fn patch_params_null_normalization(package_proxy_path: &Path) -> Result<()> {
    let mut contents = fs::read_to_string(package_proxy_path).map_err(|e| e.to_string())?;

    // Original stdio parser in `proxy.js`. It validates the JSON-RPC message
    // before we have a chance to normalize `params: null`.
    let parse_original = "function deserializeMessage(line) {\n  return JSONRPCMessageSchema.parse(JSON.parse(line));\n}";

    // Previous diagnostic patch kept here only so local dev installations can
    // be upgraded cleanly if they still contain it.
    let parse_patch_v1 = r#"function deserializeMessage(line) {
  const __zedDatadogMcpParamsNullPatchV1 = true;
  const message = JSON.parse(line);
  if (message && typeof message === "object" && message.params === null) {
    console.error(`[zed-datadog-mcp ${new Date().toISOString()}] normalizing inbound params:null for ${message.method || message.id || "unknown"}`);
    delete message.params;
  }
  return JSONRPCMessageSchema.parse(message);
}"#;

    let parse_patch = r#"function deserializeMessage(line) {
  const __zedDatadogMcpParamsNullPatchV2 = true;
  const message = JSON.parse(line);
  if (message && typeof message === "object" && message.params === null) {
    delete message.params;
  }
  return JSONRPCMessageSchema.parse(message);
}"#;

    if contents.contains(parse_original) {
        contents = contents.replace(parse_original, parse_patch);
    } else if contents.contains(parse_patch_v1) {
        contents = contents.replace(parse_patch_v1, parse_patch);
    } else if !contents.contains(PARAMS_NULL_PATCH_MARKER) {
        return Err(
            "Could not patch mcp-remote: expected stdio parse hook was not found".to_string(),
        );
    }

    fs::write(package_proxy_path, contents).map_err(|e| e.to_string())?;

    Ok(())
}
