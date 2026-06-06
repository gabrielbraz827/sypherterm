# SPEC-012 - SFTP file manager

Status: Draft
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

## Criterios de aceite

- Lista diretorio remoto.
- Download e upload mostram progresso.
- Cancelamento interrompe transferencia.
- Erros de permissao sao claros.

## Plano de testes

- Integration test com servidor SFTP local quando viavel.
- Teste manual de upload/download.
- Teste manual de cancelamento.

## Riscos e decisoes abertas

- A crate SSH escolhida pode exigir implementacao adicional para SFTP.
