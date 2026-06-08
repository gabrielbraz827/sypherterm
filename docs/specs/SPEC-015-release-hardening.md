# SPEC-015 - Release hardening

Status: Implementada
Prioridade: P2
Fonte: README.md, ARCHITECTURE.md

## Problema

Antes de releases publicos, o SypherTerm precisa ter builds confiaveis, permissoes revisadas, auditorias de dependencias e smoke tests de seguranca.

## Objetivos

- Configurar CI multiplataforma.
- Verificar frontend, backend e empacotamento.
- Auditar dependencias Rust e npm.
- Revisar capabilities Tauri.
- Preparar checklist de release.

## Nao-objetivos

- Definir estrategia comercial.
- Implementar auto-update antes da decisao de canal.

## Contratos

### Checks obrigatorios

| Check | Comando |
| --- | --- |
| Rust check | `cargo check` |
| Rust tests | `cargo test` |
| Frontend build | `pnpm build` |
| Svelte check | `pnpm check` |
| Tauri build | `pnpm tauri build` |
| npm audit | `pnpm audit:npm` |
| Rust audit | `pnpm audit:rust` |

## Seguranca e privacidade

- Artefatos nao podem incluir stores locais, vaults, tokens ou chaves.
- Dependencias criptograficas devem ser revisadas com cuidado adicional.
- Permissoes Tauri precisam de justificativa documentada.

## Plano de implementacao

1. [x] Criar workflow CI para Windows, Linux e macOS.
2. [x] Adicionar scripts de check em `package.json`.
3. [x] Adicionar auditoria Rust e npm.
4. [x] Criar checklist em `docs/release/CHECKLIST.md`.
5. [ ] Definir processo de assinatura quando canal de distribuicao for escolhido.

## Criterios de aceite

- CI executa checks em PR.
- Build Tauri gera artefato local.
- Checklist cobre SSH, vault, sync e UI basica.
- Capabilities revisadas antes de release.

## Plano de testes

- Rodar pipeline em PR.
- Smoke test manual do binario gerado.
- Verificar conteudo do artefato final.

## Riscos e decisoes abertas

- Assinatura e auto-update dependem do canal de distribuicao escolhido.

## Implementacao

- Workflow criado em `.github/workflows/ci.yml` com matriz `ubuntu-22.04`, `macos-latest` e `windows-latest`.
- CI executa `pnpm check`, `pnpm test:unit`, `pnpm build`, `pnpm check:rust`, `pnpm test:rust` e `pnpm build:tauri`.
- Auditoria roda em job separado com `pnpm audit:npm` e `pnpm audit:rust`; `cargo-audit` e instalado no CI.
- `packageManager` fixado em `pnpm@11.2.2`, alinhado ao Corepack local e compativel com `pnpm-lock.yaml` lockfile v9.
- Checklist criado em `docs/release/CHECKLIST.md`.
- Revisao de capabilities documentada em `docs/release/CAPABILITIES.md`.
- Notas de auditoria documentadas em `docs/release/DEPENDENCY_AUDIT.md`.
- Decisao: `RUSTSEC-2023-0071` e temporariamente ignorado por vir de `rsa` via `russh`/`ssh-key` sem upgrade corrigido disponivel na stack atual; deve ser reavaliado antes de release publico.
- `.gitignore` passou a cobrir artefatos Tauri e arquivos locais sensiveis (`*.local.json`, `*.vault`, `*.key`, `*.pem`, `*.p12`).
