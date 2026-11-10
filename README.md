# alurianot-dll-licenses

Licenças da `ddraw.dll` do AlurianOT. Repo público: só dados públicos (ids, nomes, chaves públicas). A chave mestre fica nos Secrets.

## Como funciona

```
licenses.json ──► GitHub Action (2x/dia + a cada push)
                    assina um token por licença ativa com a CHAVE MESTRE
                    (secret DLL_LICENSE_MASTER_KEY), válido por leaseHours (48 h)
                    └─► gist secreto: <id>.tok   (sobrescreve, não cresce o repo)

Servidor TFS do dono da licença
  baixa <id>.tok a cada 1 h (dllLicenseTokenUrl) e guarda em <key>.token
  no login, manda pra DLL: token + assinatura do nonce com a chave do servidor

DLL
  tem só a chave PÚBLICA mestre e o id que aceita (TIBIA_LICENSE_ID)
  confere: token assinado pela mestre? não venceu? id certo?
           nonce assinado pela chave do servidor que o token cita?
  se não: mostra mensagem e fecha o cliente
```

Revogar = o token para de ser renovado e o último vence em até 48 h, mesmo
que o servidor fique offline.

## Criar uma licença

1. O dono do servidor gera a chave do servidor dele (no repo do cliente):
   `python tools/license/gen_license.py --name fulano --license-id 3 --out C:\tmp\licenses`
   - `fulano-3.key` → vai para o servidor dele (`dllLicenseKeyFile`). Secreto.
   - `fulano-3.json` → tem o `publicKey` (pode ser público).
2. Adicione em `licenses.json`:
   ```json
   { "id": 3, "name": "fulano", "serverPublicKey": "<publicKey>", "revoked": false, "expires": null }
   ```
   `expires` opcional (`"2027-01-31"`, UTC, inclusive) = data final automática.
3. Commit + push. A Action publica `3.tok` no gist em ~1 min.
4. No `config.lua` do servidor dele:
   ```lua
   dllLicenseKeyFile = "/home/usuario/.alurianot/dll_license.key"
   dllLicenseId = 3
   dllLicenseTokenUrl = "https://gist.githubusercontent.com/<usuario>/<GIST_ID>/raw/3.tok"
   ```
5. A DLL dele é compilada com `TIBIA_LICENSE_ID 3u`.

## Revogar

`"revoked": true` (ou apague a entrada) → commit + push. O arquivo some do gist
e o servidor dele para em até `leaseHours`.

## Configuração do GitHub (uma vez)

- **Secret** `DLL_LICENSE_MASTER_KEY`: conteúdo de `master.key` (PEM).
- **Secret** `GIST_TOKEN`: fine-grained token com **só** a permissão de conta
  *Gists: Read and write*.
- **Variable** `TOKEN_GIST_ID`: id do gist secreto dos tokens.

## Chave mestre

`tools/gen_master_key.py` gera uma vez. Fica no secret e num backup offline,
**nunca** no git. Perder a chave mestre = recompilar e redistribuir todas as
DLLs com uma chave nova. Vazar a chave mestre = qualquer um emite licenças.

## Teste local

```
python tools/sign_tokens.py --master-key C:\tmp\licenses\master.key --out C:\tmp\licenses\tokens
```
