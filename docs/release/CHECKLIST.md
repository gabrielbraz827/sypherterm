# Release checklist

Status: Baseline for SPEC-015

Use this checklist before publishing any SypherTerm build.

## Required automated checks

- [ ] `pnpm install --frozen-lockfile`
- [ ] `pnpm check`
- [ ] `pnpm test:unit`
- [ ] `pnpm build`
- [ ] `pnpm check:rust`
- [ ] `pnpm test:rust`
- [ ] `pnpm audit:npm`
- [ ] `pnpm audit:rust`
- [ ] `pnpm build:tauri`

## Dependency review

- [ ] Review npm audit results and document accepted advisories.
- [ ] Review Rust audit results and [DEPENDENCY_AUDIT.md](DEPENDENCY_AUDIT.md).
- [ ] Re-check whether `RUSTSEC-2023-0071` still needs to be ignored.
- [ ] Re-check crypto-adjacent crates with extra care: `argon2`, `aes-gcm`, `rand`, `zeroize`, `russh`, `russh-sftp`.
- [ ] Confirm lockfiles are committed and match the released source.

## Capabilities and artifact hygiene

- [ ] Review [CAPABILITIES.md](CAPABILITIES.md).
- [ ] Confirm `.env`, local stores, vaults, tokens, keys and private certificates are absent from the artifact.
- [ ] Confirm `src-tauri/target/release/bundle` contains only generated app bundles/installers.
- [ ] Confirm `sypherterm.local.json`, `*.local.json`, `*.vault`, `*.key`, `*.pem` and `*.p12` are not tracked.

## Smoke tests

- [ ] Launch the packaged app.
- [ ] Create a new local vault with a strong password.
- [ ] Lock and unlock the vault.
- [ ] Save, edit, search and delete a profile.
- [ ] Open an SSH session with temporary credentials.
- [ ] Resize the terminal pane and confirm PTY resize still works.
- [ ] Split panes and tabs, then restart the app and confirm no live session ids persist.
- [ ] Copy and paste terminal text through explicit user actions.
- [ ] Start and stop a local tunnel from an active SSH session.
- [ ] List a remote SFTP directory and perform one safe download/upload against a test host.
- [ ] Save a snippet with `{{variable}}`, insert it into the active terminal and confirm Enter is not sent automatically.
- [ ] Run local sync push and pull against a temporary directory.

## Security smoke tests

- [ ] Confirm wrong vault password returns a recoverable error without data disclosure.
- [ ] Confirm vault-locked state blocks snippet listing and vault-backed operations.
- [ ] Confirm Data Plane rejects invalid or reused WebSocket auth tokens.
- [ ] Confirm notifications do not include hostnames, usernames, command bodies, vault payloads or file contents.
- [ ] Confirm logs and terminal status messages do not print passwords, private keys, passphrases or vault plaintext.

## Release decision

- [ ] Decide signing identity and distribution channel before public release.
- [ ] Decide whether auto-update is enabled for this channel.
- [ ] Record release version, commit SHA and generated artifact hashes.
- [ ] Archive CI run links and manual smoke-test notes.
