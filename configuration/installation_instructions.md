## Join the Preview!

The Datadog MCP Server is in Preview. If you're interested in this feature, complete [this form](https://www.datadoghq.com/product-preview/datadog-mcp-server/).

Read more about the MCP Server on the [Datadog blog](https://www.datadoghq.com/blog/datadog-remote-mcp-server/) or in the [Datadog Documentation](https://docs.datadoghq.com/bits_ai/mcp_server/).


### MCP Server Installation Instructions

Make sure you are using the correct site value: **US1, US3, US5, EU1, AP1, or AP2**.

#### Toolsets

The Datadog MCP Server supports **toolsets**, which let you enable only the product-specific tools you need and reduce the number of tool definitions sent to Zed.

Configure toolsets with the `toolsets` array in the server settings:

```json
{
  "site": "US1",
  "toolsets": ["core", "dashboards", "synthetics"]
}
```

If `toolsets` is omitted or empty, Datadog uses the default `core` toolset. You can also use `["all"]` to enable all generally available toolsets, although this increases the number of tools sent to the client.

See the full list of available toolsets in the [Datadog MCP Server documentation](https://docs.datadoghq.com/bits_ai/mcp_server/setup/?tab=vscode#available-toolsets).

Finally, **click on Configure Server** and follow the prompts to log in to your Datadog account.
