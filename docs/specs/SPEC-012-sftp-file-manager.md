# SPEC-012 - SFTP file manager

Status: Done
Prioridade: P2
Fonte: README.md

## Problema

Usuarios frequentemente precisam navegar, baixar e enviar arquivos em hosts SSH. Um SFTP integrado reduz troca de ferramentas e melhora produtividade.

## Objetivos

- Navegar diretorios remotos.
- Upload, download, rename, delete e mkdir.
- Mostrar progresso e permitir cancelamento.
- Reutilizar credenciais e sessao SSH quando possivel.

## Nao-objetivos

- Editor remoto completo.
- Sync bidirecional de pastas.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `sftp_list_dir` | `{ sessionId: string; path: string }` | `RemoteDirEntry[]` | `not_found`, `permission_denied` |
| `sftp_download` | `SftpTransferRequest` | `TransferJob` | `permission_denied`, `io_error` |
| `sftp_upload` | `SftpTransferRequest` | `TransferJob` | `permission_denied`, `io_error` |
| `sftp_cancel_transfer` | `{ jobId: string }` | `TransferJob` | `not_found` |
| `sftp_mkdir` | `{ sessionId: string; path: string }` | `RemoteDirEntry` | `permission_denied`, `io_error` |
| `sftp_rename` | `{ sessionId: string; oldPath: string; newPath: string }` | `RemoteDirEntry` | `permission_denied`, `io_error` |
| `sftp_delete` | `{ sessionId: string; path: string; kind?: RemoteEntryKind }` | `RemoteDirEntry` | `permission_denied`, `io_error` |

## Seguranca e privacidade

- Caminhos locais devem ser escolhidos pelo usuario.
- Logs nao devem incluir conteudo de arquivos.
- Operacoes destrutivas exigem confirmacao.

## Plano de implementacao

1. Verificar suporte SFTP na crate SSH escolhida.
2. Criar modelos de arquivo remoto e jobs.
3. Implementar listagem.
4. Implementar transfers com progresso.
5. Criar UI em painel dedicado.

## Implementacao entregue

- A crate `russh-sftp` foi adicionada para abrir o subsistema SFTP sobre `russh`.
- `SftpRegistry` gerencia listagem, mkdir, rename, delete, upload, download e cancelamento cooperativo de jobs.
- Transfers usam jobs em memoria com `bytesTransferred`, `totalBytes` e estado `running`, `completed`, `cancelled` ou `failed`.
- A UI ganhou um painel SFTP preso ao pane/sessao ativa, com listagem, selecao, operacoes destrutivas confirmadas e upload/download por caminhos informados pelo usuario.
- Caminhos locais continuam sendo entrada explicita do usuario; nenhum conteudo de arquivo e logado ou persistido pelo app.

## Criterios de aceite

- Lista diretorio remoto.
- Download e upload mostram progresso.
- Cancelamento interrompe transferencia.
- Erros de permissao sao claros.

## Plano de testes

- Integration test com servidor SFTP local quando viavel.
- Teste manual de upload/download.
- Teste manual de cancelamento.
- Unit tests para validacao e ordenacao de entradas remotas.

## Riscos e decisoes abertas

- Decisao: SFTP usa uma conexao SFTP separada derivada da `sessionId` ativa, reutilizando temporariamente o mesmo material de conexao em memoria. A sessao shell existente continua dedicada ao terminal/Data Plane.
- Decisao: o cancelamento e cooperativo; o job e marcado como cancelado e o loop de transferencia para no proximo chunk.
- Risco: validacao manual ainda precisa de um servidor SFTP real para confirmar compatibilidade de permissao, path e throughput por plataforma.
