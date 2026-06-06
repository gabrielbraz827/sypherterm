# SPEC-015 - Release hardening

Status: Draft
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

## Seguranca e privacidade

- Artefatos nao podem incluir stores locais, vaults, tokens ou chaves.
- Dependencias criptograficas devem ser revisadas com cuidado adicional.
- Permissoes Tauri precisam de justificativa documentada.

## Plano de implementacao

1. Criar workflow CI para Windows, Linux e macOS.
2. Adicionar scripts de check em `package.json`.
3. Adicionar auditoria Rust e npm.
4. Criar checklist em `docs/release/CHECKLIST.md`.
5. Definir processo de assinatura quando canal de distribuicao for escolhido.

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
