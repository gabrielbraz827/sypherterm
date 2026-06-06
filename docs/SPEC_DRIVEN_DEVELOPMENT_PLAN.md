# Plano de Implementacoes e Acoes Spec-Driven Development - SypherTerm

Este documento transforma a visao do `README.md` e a arquitetura do `ARCHITECTURE.md` em um plano de desenvolvimento guiado por especificacoes. A ideia e que cada entrega relevante comece por uma spec versionada, com contratos, criterios de aceite, plano de testes e decisoes de seguranca antes da implementacao.

## Objetivo

Construir o SypherTerm como um cliente SSH moderno, leve, local-first e zero-knowledge, mantendo rastreabilidade entre:

- Visao do produto: soberania de dados, BYOi, UX moderna, baixo consumo e codigo aberto.
- Arquitetura: Tauri + Rust, Svelte + Vite, Control Plane via IPC, Data Plane via WebSocket local.
- Implementacao: modulos Rust, componentes Svelte, contratos de dados, testes e releases.

## Fontes de verdade

- `README.md`: define a proposta de valor, os principios de produto e as capacidades esperadas: SSH, sincronizacao em infraestrutura propria, criptografia local, SFTP, tunnels, snippets, tabs, split panes e UX premium.
- `ARCHITECTURE.md`: define o stack, separacao Control Plane/Data Plane, estrutura de diretorios, plugins Tauri e esqueletos dos modulos Rust.
- Estado atual do repositorio: app Tauri/Svelte inicial, plugins Tauri ja instalados, `App.svelte` ainda placeholder e backend com comando exemplo `greet`.

## Principios do processo spec-driven

1. Toda funcionalidade de produto ou arquitetura deve nascer como uma spec em `docs/specs/`.
2. Nenhuma spec deve ser considerada pronta sem objetivos, nao-objetivos, contratos, criterios de aceite e plano de testes.
3. Contratos entre frontend e backend devem ser escritos antes da implementacao.
4. Mudancas que afetem seguranca, armazenamento, criptografia, rede ou sincronizacao exigem secao explicita de risco.
5. Implementacoes devem ser pequenas o bastante para caber em PRs/revisoes orientadas por uma spec.
6. A spec deve ser atualizada quando a implementacao revelar uma decisao nova.

## Fluxo padrao para cada spec

1. Escrever a spec em `docs/specs/SPEC-xxx-nome.md`.
2. Definir contratos: comandos Tauri, payloads JSON, eventos, erros, estados e modelos persistidos.
3. Definir ameacas e limites de seguranca quando houver credenciais, chaves, rede ou sincronizacao.
4. Criar testes ou fixtures que expressem os criterios de aceite.
5. Implementar backend, frontend e integracao em fatias verticais.
6. Rodar verificacoes: `pnpm build`, `pnpm tauri build` quando aplicavel, `cargo test`, `cargo check`, `svelte-check` e testes especificos.
7. Atualizar a spec com decisoes finais, pendencias e observacoes de release.

## Template recomendado de spec

```md
# SPEC-XXX - Nome da funcionalidade

Status: Draft | Accepted | Implementing | Done
Fonte: README.md | ARCHITECTURE.md | Issue | Decisao tecnica

## Problema

## Objetivos

## Nao-objetivos

## Experiencia do usuario

## Contratos

### Comandos Tauri

### Eventos e WebSocket

### Modelos persistidos

### Erros esperados

## Seguranca e privacidade

## Plano de implementacao

## Criterios de aceite

## Plano de testes

## Riscos e decisoes abertas
```

## Backlog de specs

| ID | Spec | Entrega principal | Prioridade |
| --- | --- | --- | --- |
| SPEC-001 | Fundacao do app e organizacao de modulos | Remover placeholder, criar estrutura Rust/Svelte real e AppState | P0 |
| SPEC-002 | Modelo de dominio e armazenamento local | Perfis SSH, snippets, preferencias e estado de sessao no Store | P0 |
| SPEC-003 | Vault criptografado local | AES-256-GCM, Argon2id, envelope de dados e protecao de segredo | P0 |
| SPEC-004 | Control Plane IPC | Comandos Tauri para perfis, conexoes, sessao, sync e config | P0 |
| SPEC-005 | Data Plane WebSocket local | Servidor `tokio-tungstenite` em `127.0.0.1:0`, token por sessao e streaming binario | P0 |
| SPEC-006 | Engine SSH assincrona | Ciclo de vida de conexao, autenticacao, canais, resize e encerramento | P0 |
| SPEC-007 | Terminal UI | `xterm.js`, input/output binario, temas, copy/paste e resize | P0 |
| SPEC-008 | Gerenciamento de perfis | CRUD de hosts, grupos, tags, identidade e validacao | P1 |
| SPEC-009 | Tabs e split panes | Multiplexacao visual, foco, layouts e restauracao de sessao | P1 |
| SPEC-010 | Sync BYOi MVP | Provedor inicial, envelope criptografado, conflito e restore | P1 |
| SPEC-011 | Integracoes nativas | Clipboard, notificacoes, OS info e opener com permissoes minimas | P1 |
| SPEC-012 | SFTP file manager | Navegacao, upload/download, progresso e erros recuperaveis | P2 |
| SPEC-013 | Port forwarding | Tunnels local/remote/dynamic, status e encerramento seguro | P1 |
| SPEC-014 | Snippet organizer | Snippets criptografados, variaveis, busca e insercao no terminal | P2 |
| SPEC-015 | Release hardening | Assinatura, empacotamento, telemetria opcional zero-sensitive e auditorias | P2 |

## Fase 0 - Infraestrutura de specs

Resultado esperado: o repositorio passa a ter um processo claro para transformar ideias em entregas verificaveis.

Acoes:

- Criar `docs/specs/README.md` explicando o processo.
- Criar `docs/specs/TEMPLATE.md` com o template acima.
- Criar `docs/adr/` para decisoes arquiteturais duradouras.
- Registrar `ADR-001-control-plane-data-plane.md`.
- Registrar `ADR-002-local-first-zero-knowledge.md`.
- Definir labels ou convencoes de PR: `spec:SPEC-xxx`, `area:frontend`, `area:rust`, `area:security`.

Criterios de aceite:

- Toda nova issue de feature aponta para uma spec.
- Toda spec tem status claro.
- Mudancas sensiveis de arquitetura possuem ADR.

## Fase 1 - Fundacao tecnica do aplicativo

Resultado esperado: substituir o esqueleto inicial por uma base coerente com a arquitetura.

Acoes:

- Remover o comando exemplo `greet` de `src-tauri/src/lib.rs`.
- Criar os modulos Rust planejados: `commands`, `crypto`, `ssh`, `sync`, `ws`.
- Criar `AppState` compartilhado para sessoes, servidor WebSocket e configuracao.
- Registrar comandos Tauri reais, mesmo que alguns com implementacao inicial controlada.
- Adicionar dependencias Rust base: `tokio`, `tokio-tungstenite`, `futures-util`, `uuid`, `thiserror`, `tracing`, `serde`, `serde_json`.
- Criar estrutura frontend: `src/components`, `src/lib`, `src/lib/store.ts`, `src/lib/websocket.ts`, `src/lib/api.ts`.
- Configurar Tailwind CSS como sistema utilitario principal do frontend Svelte.
- Definir tipos TypeScript espelhando os contratos Rust serializados.
- Adicionar `svelte-check` como script em `package.json`.

Criterios de aceite:

- `cargo check` passa no backend.
- `pnpm build` passa no frontend.
- O app abre com uma tela inicial real do SypherTerm.
- Nenhum comando placeholder fica registrado como API publica.

## Fase 2 - Modelo local, vault e seguranca

Resultado esperado: perfis e preferencias podem ser armazenados localmente sem expor segredos em texto puro.

Acoes:

- Especificar `ConnectionProfile`, `CredentialRef`, `Snippet`, `ThemePreference` e `SessionLayout`.
- Definir formato do vault criptografado com versao, salt, nonce, KDF, ciphertext e metadata minima.
- Implementar `CryptoEngine` com Argon2id para derivacao e AES-256-GCM para criptografia.
- Usar `zeroize` para limpar materiais sensiveis da memoria quando possivel.
- Garantir que logs nunca imprimam senha, chave privada, passphrase ou payload descriptografado.
- Definir fluxo de master password: criar vault, desbloquear vault, bloquear vault, trocar senha.
- Implementar comandos IPC para salvar/carregar perfis sem expor segredo ao frontend alem do necessario.

Criterios de aceite:

- Um perfil pode ser criado, salvo, recarregado e removido.
- Segredos persistidos aparecem apenas como blob criptografado.
- Falha de senha mestre retorna erro controlado e nao corrompe dados.
- Testes unitarios cobrem encrypt/decrypt, senha invalida, nonce unico e migracao por versao.

## Fase 3 - SSH MVP com Data Plane

Resultado esperado: abrir uma sessao SSH interativa usando IPC para controle e WebSocket local para bytes do terminal.

Acoes:

- Resolver a decisao da crate SSH em spec: `russh` ou alternativa compativel com async Rust.
- Implementar `ws::StreamServer` com bind em `127.0.0.1:0` e porta efemera.
- Gerar token por sessao para impedir conexoes WebSocket locais nao autorizadas.
- Retornar ao frontend `session_id`, `ws_url` e `auth_token` via comando IPC `connect_ssh`.
- Implementar piping bidirecional: SSH stdout/stderr para WebSocket e WebSocket input para SSH stdin.
- Implementar eventos de terminal: `data`, `resize`, `close`, `error`, `heartbeat`.
- Implementar encerramento limpo de sessao e liberacao de recursos.
- Adicionar timeout e limite de reconexao para evitar processos presos.

Criterios de aceite:

- Usuario conecta em um host SSH valido e recebe prompt interativo.
- Digitar no terminal envia bytes ao SSH.
- Saida intensa nao trava a UI.
- Resize do terminal atualiza o PTY/canal remoto.
- Fechar uma aba encerra a sessao no backend.

Observacao arquitetural:

- O `tauri-plugin-websocket` e cliente WebSocket. Ele nao deve hospedar o Data Plane. O servidor local deve ser implementado manualmente com `tokio-tungstenite`, como descrito no fluxo principal da arquitetura.

## Fase 4 - UX de terminal e produtividade

Resultado esperado: entregar a experiencia central do produto, com terminal bonito, rapido e ergonomico.

Acoes:

- Adicionar `xterm.js` e addons necessarios: fit, web-links, search quando aplicavel.
- Criar `TerminalInstance.svelte` com ciclo de montagem/desmontagem confiavel.
- Criar sidebar com grupos, perfis recentes e status de conexao.
- Implementar temas, fonte, tamanho, cursor e comportamento de copy/paste.
- Criar barra de tabs com estado de foco e titulo dinamico.
- Implementar split panes por layout declarativo no estado.
- Persistir layout recente sem salvar dados sensiveis.
- Usar Tailwind CSS para layout, superficies, paineis e controles, mantendo CSS especifico apenas onde `xterm.js` exigir.

Criterios de aceite:

- Terminal renderiza saida continua sem flicker perceptivel.
- O usuario consegue alternar entre sessoes sem perder estado.
- Copy/paste funciona via plugin nativo quando necessario.
- Layout de tabs e panes sobrevive a reload controlado.
- Interface nao depende de texto explicativo para acoes principais.

## Fase 5 - Sync BYOi criptografado

Resultado esperado: sincronizar apenas blobs criptografados para infraestrutura do usuario.

Acoes:

- Especificar interface `SyncProvider`: `pull`, `push`, `list_versions`, `test_connection`.
- Comecar por um provedor com menor superficie de risco, preferencialmente arquivo local ou S3 compativel.
- Definir estrategia de conflito: timestamp, device id, hash do payload e merge assistido quando possivel.
- Garantir que provider nunca receba dados descriptografados.
- Adicionar comando `trigger_cloud_sync(provider)` com progresso e erros estruturados.
- Emitir notificacoes nativas para sucesso/falha de sync quando habilitado.

Criterios de aceite:

- Sync envia somente blob criptografado.
- Restore em instalacao limpa recupera perfis apos senha mestre correta.
- Conflito nao sobrescreve dados silenciosamente.
- Logs mostram provider e status, mas nunca segredos ou conteudo do vault.

## Fase 6 - Recursos avancados

Resultado esperado: expandir o produto alem do SSH basico sem comprometer a arquitetura.

Acoes:

- SFTP: especificar navegacao, operacoes, progresso, cancelamento e permissao de arquivos.
- Port forwarding: implementar local forwarding primeiro, depois remote e dynamic conforme suporte da crate SSH.
- Snippets: especificar armazenamento criptografado, busca, variaveis e insercao segura.
- Colaboracao ou WebSocket externo: usar `tauri-plugin-websocket` apenas como cliente auxiliar se a feature exigir.
- Importacao/exportacao: suportar formatos comuns sem vazar segredo em exports nao criptografados por padrao.

Criterios de aceite:

- Cada recurso avancado pode ser desabilitado sem quebrar SSH basico.
- Falhas de rede sao recuperaveis e visiveis.
- Permissoes Tauri continuam minimas.

## Fase 7 - Release hardening

Resultado esperado: preparar builds confiaveis, auditaveis e seguros.

Acoes:

- Configurar CI para Windows, macOS e Linux.
- Rodar `cargo test`, `cargo check`, `pnpm build`, `svelte-check` e lint.
- Adicionar `cargo audit` ou ferramenta equivalente.
- Revisar `src-tauri/capabilities/default.json` para permissoes minimas.
- Configurar assinatura e updater quando o canal de distribuicao estiver definido.
- Criar checklist de release com smoke test de conexao SSH, vault e sync.

Criterios de aceite:

- Build reproduzivel nos sistemas alvo.
- Permissoes documentadas e justificadas.
- Artefatos de release nao contem chaves, tokens ou configuracoes locais.

## Contratos iniciais recomendados

### Comandos Tauri

| Comando | Entrada | Saida | Spec |
| --- | --- | --- | --- |
| `create_vault` | `CreateVaultRequest` | `VaultStatus` | SPEC-003 |
| `unlock_vault` | `UnlockVaultRequest` | `VaultStatus` | SPEC-003 |
| `save_profile` | `ConnectionProfileDraft` | `ConnectionProfile` | SPEC-002 |
| `list_profiles` | vazio | `ConnectionProfileSummary[]` | SPEC-002 |
| `connect_ssh` | `ConnectSshRequest` | `ConnectSshResponse` | SPEC-004/SPEC-006 |
| `disconnect_session` | `SessionId` | `SessionStatus` | SPEC-006 |
| `resize_session` | `SessionResizeRequest` | `SessionStatus` | SPEC-006 |
| `trigger_cloud_sync` | `SyncRequest` | `SyncJobStatus` | SPEC-010 |

### Eventos do Data Plane

| Evento | Direcao | Payload | Observacao |
| --- | --- | --- | --- |
| `auth` | Frontend -> Backend | token da sessao | Primeiro frame apos conectar |
| `input` | Frontend -> Backend | bytes | Teclas e paste |
| `resize` | Frontend -> Backend | cols, rows | Ajuste de terminal |
| `output` | Backend -> Frontend | bytes | Stream SSH |
| `status` | Backend -> Frontend | estado | Connecting, connected, closed |
| `error` | Backend -> Frontend | codigo, mensagem | Sem segredos |
| `heartbeat` | ambos | timestamp | Deteccao de conexao morta |

## Definition of Done global

- Spec aceita e atualizada.
- Contratos documentados.
- Testes criados ou justificativa registrada quando teste automatico nao for viavel.
- Build local passa.
- Erros retornam tipos previsiveis.
- Logs nao contem segredos.
- UX principal validada manualmente no app Tauri.
- Permissoes Tauri revisadas quando a feature toca sistema operacional, rede, arquivos ou clipboard.

## Riscos principais

- Streaming via IPC causar travamento: mitigar mantendo bytes do terminal fora do IPC e usando WebSocket local binario.
- Porta local acessivel por outro processo: mitigar com bind em `127.0.0.1`, porta efemera, token por sessao e expiracao curta.
- Vazamento de credenciais em logs ou store: mitigar com tipos dedicados para segredo, redaction e testes.
- Criptografia caseira insegura: mitigar usando crates auditadas, parametros KDF documentados e testes com vetores.
- Sync sobrescrever dados: mitigar com versionamento, hash, device id e politica explicita de conflito.
- UI bonita mas pesada: mitigar com Svelte, xterm.js, renderizacao incremental e medicoes de throughput.

## Primeiras 10 acoes recomendadas

1. Criar `docs/specs/TEMPLATE.md`.
2. Criar `SPEC-001-fundacao-app.md`.
3. Criar `ADR-001-control-plane-data-plane.md`.
4. Refatorar `src-tauri/src/lib.rs` para registrar `commands.rs` em vez de `greet`.
5. Criar modulos Rust vazios com tipos e erros compartilhados.
6. Adicionar `AppState` e testes de inicializacao basicos.
7. Criar `src/lib/api.ts`, `src/lib/store.ts` e `src/lib/websocket.ts`.
8. Especificar `ConnectionProfile` e `VaultEnvelope`.
9. Implementar o primeiro comando real: `list_profiles`.
10. Adicionar `xterm.js` somente depois do contrato inicial do Data Plane estar aceito.

## Marco MVP

O MVP deve ser considerado pronto quando o usuario conseguir:

- Criar ou desbloquear um vault local.
- Cadastrar um perfil SSH.
- Conectar em um host via SSH.
- Usar terminal interativo com entrada, saida e resize.
- Fechar a sessao sem processo preso.
- Persistir o perfil criptografado.
- Exportar ou sincronizar um blob criptografado em pelo menos um provider inicial.

Esse marco preserva o centro da proposta do SypherTerm: terminal SSH moderno, leve, local-first, seguro e independente de infraestrutura proprietaria.
