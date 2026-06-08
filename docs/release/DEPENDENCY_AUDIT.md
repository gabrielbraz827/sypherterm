# Dependency audit notes

Status: Baseline for SPEC-015

## Known accepted RustSec advisory

| Advisory | Source | Current decision | Release requirement |
| --- | --- | --- | --- |
| `RUSTSEC-2023-0071` | `rsa 0.10.0-rc.18` via `russh` and `ssh-key` | Temporarily ignored in `pnpm audit:rust` because no fixed upgrade is available through the current SSH stack. | Re-check `russh` upgrades before public release and document whether RSA authentication/host-key paths remain exposed. |

`cargo tree --manifest-path src-tauri/Cargo.toml -i rsa` shows:

```text
rsa v0.10.0-rc.18
|-- russh v0.61.2
|   `-- sypherterm
`-- ssh-key v0.7.0-rc.10
    `-- russh
```

## Current warning classes

`cargo audit` also reports unmaintained or unsound transitive crates from platform UI stacks and parser utilities. They are not denied by the release audit script, but they must be reviewed before publishing:

- GTK3 binding advisories inherited through desktop/webview dependencies.
- `glib` unsound iterator advisory inherited through GTK3 bindings.
- `proc-macro-error` unmaintained.
- `unic-*` unmaintained parser utility crates.

## Review process

- Keep `pnpm audit:npm` at `moderate` or stricter.
- Keep `pnpm audit:rust` in CI and document every ignored advisory here.
- Do not add a new ignored RustSec advisory without a specific source path, impact note and release requirement.
- Prefer dependency upgrades over ignores whenever a fixed upgrade exists.
