# AI Assistant

Chat with Claude about your project using **your own Anthropic API key**. The key is saved in the encrypted vault (`ai/anthropic/api-key`) and only used for requests you send. Remove it any time in the Assistant screen.

## Use VSN Devify from Claude Desktop and other AI tools (MCP)
`vsn-agent mcp` is a Model Context Protocol server over stdio. Add it to your AI tool's MCP config, for example in Claude Desktop's `claude_desktop_config.json`:

```json
{ "mcpServers": { "vsn-devify": { "command": "C:\\Program Files\\VSN Dev Platform\\bin\\vsn-agent.exe", "args": ["mcp"] } } }
```

Tools: `list_spaces`, `space_status`, `start_service`, `stop_service`, `diagnose`, `request_stats`. Every call goes through the running VSN Agent, so space locks, permissions and the audit log apply. Keep VSN Devify open while you use it.
