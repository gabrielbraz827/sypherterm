# Tauri capabilities review

Status: Baseline for SPEC-015

This document records why each default capability is enabled before public release. Any new permission must be added here before merging.

| Permission | Purpose | Sensitive surface | Release decision |
| --- | --- | --- | --- |
| `core:default` | Required baseline for Tauri window/runtime behavior. | Generic app runtime access. | Keep. Required by Tauri. |
| `opener:default` | Open external paths or links through OS integration when explicitly requested. | Can launch external handlers. | Keep, but only call from explicit user action. |
| `store:default` | Persist local preferences, profiles metadata and encrypted vault envelope. | Local store may include encrypted vault data and non-secret profile metadata. | Keep. Store files must never be committed or bundled. |
| `websocket:default` | Connect terminal UI to the local Data Plane WebSocket. | Local auth token exists only in memory and is single-session scoped. | Keep. WebSocket URL remains local. |
| `os:default` | Detect OS/platform for native integration behavior. | Platform metadata. | Keep. No secrets expected. |
| `notification:default` | Notify connection/sync outcomes. | Notification text could leak operational context. | Keep with redacted, generic messages. |
| `clipboard-manager:default` | Copy terminal selection and paste clipboard into terminal. | Clipboard may contain secrets. | Keep. Operations must stay user initiated. |

## Review checklist

- Confirm no permission grants filesystem-wide reads beyond explicit paths selected or typed by the user.
- Confirm local stores are excluded by `.gitignore` and not present in release artifacts.
- Confirm notification bodies avoid hostnames, usernames, command bodies, vault payloads, tokens and keys.
- Confirm clipboard operations are triggered only by visible user controls.
- Confirm WebSocket auth tokens are never logged and are not persisted.
- Confirm any added plugin permission has a matching row in this document.
