# SPEC-003 - Vault criptografado local

Status: Done
Prioridade: P0
Fonte: README.md, ARCHITECTURE.md

## Problema

Credenciais SSH, chaves privadas, snippets sensiveis e dados de sync precisam ser protegidos localmente antes de qualquer persistencia ou envio para infraestrutura do usuario.

## Objetivos

- Implementar vault local zero-knowledge.
- Usar Argon2id para derivacao da chave a partir da master password.
- Usar AES-256-GCM para criptografia autenticada.
- Versionar o envelope criptografado para futuras migracoes.

## Nao-objetivos

- Sincronizar o vault com cloud.
- Integrar keychain nativo do sistema operacional.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `create_vault` | `CreateVaultRequest` | `VaultStatus` | `vault_exists`, `weak_password`, `crypto_error` |
| `unlock_vault` | `UnlockVaultRequest` | `VaultStatus` | `invalid_password`, `vault_missing`, `crypto_error` |
| `lock_vault` | vazio | `VaultStatus` | `vault_missing` |
| `change_master_password` | `ChangeMasterPasswordRequest` | `VaultStatus` | `invalid_password`, `weak_password`, `crypto_error` |

### Modelos persistidos

```ts
type VaultEnvelope = {
  version: 1;
  kdf: {
    algorithm: "argon2id";
    memoryKiB: number;
    iterations: number;
    parallelism: number;
    saltBase64: string;
  };
  cipher: {
    algorithm: "aes-256-gcm";
    nonceBase64: string;
    ciphertextBase64: string;
  };
  createdAt: string;
  updatedAt: string;
};

type VaultStatus = {
  state: "missing" | "locked" | "unlocked";
  version?: number;
};
```

## Seguranca e privacidade

- Master password nunca deve ser persistida.
- Payload descriptografado nunca deve ser logado.
- Usar `zeroize` para limpar buffers sensiveis quando aplicavel.
- Nonce AES-GCM deve ser unico por operacao de criptografia.

## Plano de implementacao

1. Escolher crates: `argon2`, `aes-gcm`, `rand`, `zeroize`, `base64`.
2. Implementar `CryptoEngine::encrypt_payload`.
3. Implementar `CryptoEngine::decrypt_payload`.
4. Criar armazenamento do envelope no Store.
5. Criar cache em memoria somente enquanto vault estiver desbloqueado.
6. Implementar bloqueio manual e bloqueio por encerramento do app.

## Criterios de aceite

- Vault criado pode ser desbloqueado com a senha correta.
- Senha incorreta retorna erro sem corromper o envelope.
- Dois encrypts do mesmo payload geram ciphertexts diferentes.
- Dados descriptografados nao aparecem em logs.

## Plano de testes

- Unit tests de encrypt/decrypt.
- Unit tests de senha invalida.
- Unit tests de nonce unico.
- Teste de migracao rejeitando versao desconhecida com erro claro.

## Decisoes tomadas

- O vault e persistido no mesmo store local da SPEC-002, arquivo `sypherterm.local.json`, chave `vault`.
- O payload inicial criptografado e `{}` para validar criacao/desbloqueio antes dos modelos sensiveis de credenciais e snippets.
- A master password minima tem 12 caracteres e retorna `weak_password` quando falha.
- Parametros Argon2id iniciais: `memoryKiB = 19456`, `iterations = 2`, `parallelism = 1`.
- AES-256-GCM usa nonce aleatorio de 12 bytes e salt aleatorio de 16 bytes por criptografia.
- O campo do envelope segue exatamente `memoryKiB`, com rename explicito no modelo Rust.
- O payload descriptografado fica em memoria no `AppState` somente enquanto o vault esta desbloqueado e e limpo ao bloquear.
- `get_app_status` sincroniza o estado do vault com o Store para mostrar `locked` quando existe envelope salvo.
- A UI minima permite criar, desbloquear, bloquear e trocar a master password sem persistir a senha no frontend.

## Riscos e decisoes abertas

- Parametros Argon2id devem equilibrar seguranca e performance em maquinas modestas.
- O payload `{}` deve evoluir para um schema versionado quando credenciais e snippets entrarem no vault.
