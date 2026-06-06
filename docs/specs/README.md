# SypherTerm Specs

Este diretorio contem as especificacoes que guiam o desenvolvimento do SypherTerm. Cada spec descreve o problema, os contratos, o plano de implementacao, os criterios de aceite e os testes esperados antes de uma entrega ser considerada pronta.

## Fluxo

1. Criar ou atualizar uma spec antes de implementar.
2. Marcar o status como `Draft`, `Accepted`, `Implementing` ou `Done`.
3. Manter contratos de IPC, WebSocket, modelos persistidos e erros documentados.
4. Implementar em fatias pequenas rastreaveis pela spec.
5. Atualizar a spec se a implementacao mudar alguma decisao.

## Estados

- `Draft`: proposta inicial, ainda pode mudar bastante.
- `Accepted`: escopo aprovado para implementacao.
- `Implementing`: trabalho em andamento no codigo.
- `Done`: criterios de aceite cumpridos e verificados.

## Indice

| ID | Arquivo | Prioridade |
| --- | --- | --- |
| SPEC-001 | [Fundacao do app e organizacao de modulos](SPEC-001-fundacao-app.md) | P0 |
| SPEC-002 | [Modelo de dominio e armazenamento local](SPEC-002-modelo-dominio-storage.md) | P0 |
| SPEC-003 | [Vault criptografado local](SPEC-003-vault-criptografado-local.md) | P0 |
| SPEC-004 | [Control Plane IPC](SPEC-004-control-plane-ipc.md) | P0 |
| SPEC-005 | [Data Plane WebSocket local](SPEC-005-data-plane-websocket-local.md) | P0 |
| SPEC-006 | [Engine SSH assincrona](SPEC-006-engine-ssh-assincrona.md) | P0 |
| SPEC-007 | [Terminal UI](SPEC-007-terminal-ui.md) | P0 |
| SPEC-008 | [Gerenciamento de perfis](SPEC-008-gerenciamento-perfis.md) | P1 |
| SPEC-009 | [Tabs e split panes](SPEC-009-tabs-split-panes.md) | P1 |
| SPEC-010 | [Sync BYOi MVP](SPEC-010-sync-byoi-mvp.md) | P1 |
| SPEC-011 | [Integracoes nativas](SPEC-011-integracoes-nativas.md) | P1 |
| SPEC-012 | [SFTP file manager](SPEC-012-sftp-file-manager.md) | P2 |
| SPEC-013 | [Port forwarding](SPEC-013-port-forwarding.md) | P1 |
| SPEC-014 | [Snippet organizer](SPEC-014-snippet-organizer.md) | P2 |
| SPEC-015 | [Release hardening](SPEC-015-release-hardening.md) | P2 |

## Definition of Done

- Spec atualizada com decisoes finais.
- Contratos implementados e versionados.
- Testes automaticos adicionados ou justificativa documentada.
- `cargo check`, `cargo test`, `pnpm build` e `svelte-check` passam quando aplicavel.
- Logs e erros nao vazam segredos.
- Permissoes Tauri revisadas quando a feature toca OS, rede, arquivos ou clipboard.
