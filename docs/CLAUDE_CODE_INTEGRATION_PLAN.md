# Claude Code Integration Plan: Prompt Quiver Channel

This document outlines the strategy for integrating **Prompt Quiver** with **Claude Code** using the **Model Context Protocol (MCP)** "channels" feature.

## 1. Overview
The integration allows Prompt Quiver to act as an external "event source" for a running Claude Code session. Users can select a prompt in Prompt Quiver and "push" it directly into Claude's active terminal context.

### Key Concepts
- **Channel**: A push-based communication link from an external app to Claude Code.
- **Bridge**: A lightweight shim process spawned by Claude Code via stdio.
- **Registry**: A local file-based discovery mechanism to link TUIs to active Claude sessions.

## 2. Architectural Design

### Component Diagram
```text
[ Prompt Quiver TUI ] <--- (Local IPC: Named Pipe/Socket) ---+
                                                             |
[ Registry (~/.promptquiver/sessions/) ] <-------------------+
                                                             |
[ Claude Instance A ] --(stdio)--> [ Bridge A ] <------------+
[ Claude Instance B ] --(stdio)--> [ Bridge B ] <------------+
```

### Verified Transport Mechanism
1.  **Claude-to-Bridge**: Claude Code spawns the Bridge as a subprocess. Communication is via **stdio**. This ensures 1:1 isolation; a message sent to Bridge A *only* reaches Claude Instance A.
2.  **TUI-to-Bridge**: The Prompt Quiver TUI communicates with the Bridge via a **local IPC channel** (Named Pipes on Windows, Unix Sockets on Linux/macOS).

## 3. Technical Specification

### The Bridge Process
The Bridge is a minimal Rust executable (e.g., `pq-bridge`) that:
1.  Implements the MCP server protocol over `stdin`/`stdout`.
2.  Declares the `experimental["claude/channel"]` capability.
3.  Creates a **Session Entry** in the Registry upon startup (e.g., `session_<PID>.json`).
4.  Listens on a unique IPC pipe (e.g., `\\.\pipe\pq_bridge_<PID>`).
5.  Relays any prompt received via the IPC pipe to Claude via `notifications/claude/channel`.

### Registry Schema
Each session entry in the registry contains:
```json
{
  "pid": 1234,
  "cwd": "/work/my-project",
  "ipc_path": "\\\\.\\pipe\\pq_bridge_1234",
  "start_time": "2026-05-03T15:00:00Z"
}
```

## 4. Multi-Instance Support (Targeting)

### Scenario: Multiple Claude Windows
If multiple Claude instances are running, the Prompt Quiver TUI will:
1.  Scan the Registry directory.
2.  Filter for active sessions (checking if PIDs are still alive).
3.  Display a selection menu to the user if more than one session is found.
4.  Target only the selected session's IPC path.

### Scenario: Multiple Prompt Quivers
Each TUI instance is independent. They all read the same Registry and can send to any active Claude session.

## 5. Implementation Steps

### Phase 1: The Bridge (`pq-bridge`)
1.  Create a new crate/bin `pq-bridge`.
2.  Implement MCP handshake + `claude/channel` capability.
3.  Implement Registry registration/cleanup.
4.  Implement the IPC listener.

### Phase 2: The TUI Integration
1.  Add a `bridge_client` module to `infra` or `app`.
2.  Add a "Target Selector" UI component.
3.  Map the `C` key to the "Push to Claude" workflow.

### Phase 3: Claude Configuration
Users add the bridge to their `.mcp.json` or start Claude with:
```powershell
claude --dangerously-load-development-channels server:pq-bridge
```

## 6. References
- [Claude Code Channels Documentation](https://code.claude.com/docs/en/channels)
- [Model Context Protocol (MCP) Specification](https://modelcontextprotocol.io)
- [Claude Code CLI Reference](https://code.claude.com/docs/en/cli-reference)
- [Official MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
