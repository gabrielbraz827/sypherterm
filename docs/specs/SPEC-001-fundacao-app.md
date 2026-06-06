# SPEC-001 - Fundacao do app e organizacao de modulos

Status: Done
Prioridade: P0
Fonte: ARCHITECTURE.md, README.md

## Problema

O repositorio tem o esqueleto inicial Tauri/Svelte, mas ainda nao possui a organizacao modular descrita na arquitetura. O backend registra um comando exemplo `greet`, a UI ainda e placeholder e nao existe base compartilhada para sessoes, WebSocket, storage ou erros.

## Objetivos

- Criar a estrutura real dos modulos Rust: `commands`, `crypto`, `ssh`, `sync`, `ws` e estado da aplicacao.
- Criar a estrutura frontend: `components`, `lib/api.ts`, `lib/store.ts` e `lib/websocket.ts`.
- Configurar Tailwind CSS como camada padrao de estilos do frontend Svelte.
- Remover APIs de exemplo e registrar apenas comandos planejados.
- Deixar o projeto compilavel com a fundacao vazia, mas coerente.

## Nao-objetivos

- Implementar conexao SSH real.
- Implementar criptografia completa.
- Implementar tela final de produto.

## Experiencia do usuario

Ao abrir o app, o usuario ve uma tela inicial do SypherTerm com estrutura visual minima para perfis e terminal, ainda sem conexao real. Nenhum fluxo fake deve prometer conexao funcional.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `get_app_status` | vazio | `AppStatus` | `internal_error` |

### Modelos

```ts
type AppStatus = {
  appVersion: string;
  vault: "missing" | "locked" | "unlocked";
  activeSessions: number;
  dataPlane: "stopped" | "starting" | "running";
};
```

## Seguranca e privacidade

- Nenhum segredo deve ser aceito ou logado nesta spec.
- Logs iniciais devem ser estruturais: inicializacao, plugins, status e erros.

## Plano de implementacao

1. Remover `greet` de `src-tauri/src/lib.rs`.
2. Criar `src-tauri/src/commands.rs`, `state.rs`, `crypto/mod.rs`, `ssh/mod.rs`, `sync/mod.rs`, `ws/mod.rs`.
3. Criar `AppState` com contadores e placeholders thread-safe.
4. Registrar `get_app_status`.
5. Criar `src/lib/api.ts`, `src/lib/store.ts`, `src/lib/websocket.ts`.
6. Atualizar `App.svelte` para consumir status real.
7. Adicionar script `check` para `svelte-check` em `package.json`.
8. Instalar e configurar Tailwind CSS com PostCSS, arquivo global de estilos e content paths para Svelte/TypeScript.
9. Definir tokens iniciais de UI em `tailwind.config.*`: cores, spacing, radii, fonte mono e superficie do terminal.

## Criterios de aceite

- `cargo check` passa em `src-tauri`.
- `pnpm build` passa.
- `pnpm check` existe e executa `svelte-check`.
- `greet` nao existe mais como comando registrado.
- A UI mostra status real retornado pelo backend.
- Tailwind compila no build Vite e estilos globais sao carregados no app.

## Plano de testes

- Unit test Rust para criar `AppState`.
- Teste manual abrindo o app e confirmando status.
- Verificacao de build frontend e backend.

## Decisoes tomadas

- `AppState` usa `AtomicUsize` para contagem de sessoes e `Mutex` para estados pequenos de vault e Data Plane.
- Tailwind CSS foi configurado na linha 3.x com PostCSS e Autoprefixer, mantendo compatibilidade simples com Vite/Svelte.
- Os modulos `crypto`, `ssh`, `sync` e `ws` foram criados como placeholders intencionais para as proximas specs.

## Riscos e decisoes abertas

- Tailwind deve ser usado como sistema utilitario principal, sem impedir CSS local em componentes quando `xterm.js` ou layout de terminal exigirem regras especificas.
