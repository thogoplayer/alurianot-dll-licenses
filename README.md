# alurianot-dll-licenses

Licenças da `ddraw.dll` do AlurianOT. Repo público: só dados públicos (ids, nomes, chaves públicas). A chave mestre fica nos Secrets.

## Como funciona

```
licenses.json (este repo, público)
      │  lido pelo Worker (cache 1 min; se o GitHub cair usa a última cópia boa)
      ▼
Cloudflare Worker  alurianot-dll-licenses.<conta>.workers.dev/tokens/<id>.tok
      assina NA HORA um token com a CHAVE MESTRE (secret MASTER_KEY do Worker),
      válido por leaseHours (48 h). Licença revogada/vencida → 404.
      │
Servidor TFS do dono da licença
      baixa o token a cada 1 h (dllLicenseTokenUrl), guarda em <key>.token
      no login manda pra DLL: token + assinatura do nonce com a chave do servidor
      │
DLL: tem só a chave PÚBLICA mestre e o id que aceita (TIBIA_LICENSE_ID)
     token assinado pela mestre? não venceu? id certo? nonce assinado pelo
     servidor que o token cita?  Se não: mensagem e fecha o cliente.
```

Revogar: o Worker para de entregar token na hora (≈1 min); o último token que
o servidor já baixou vence em até `leaseHours`, mesmo se ele ficar offline.

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
3. Commit + push. Em ~1 min o Worker já entrega `/tokens/3.tok`.
4. No `config.lua` do servidor dele:
   ```lua
   dllLicenseKeyFile = "/home/usuario/.alurianot/dll_license.key"
   dllLicenseId = 3
   dllLicenseTokenUrl = "https://alurianot-dll-licenses.<conta>.workers.dev/tokens/3.tok"
   ```
5. A DLL dele é compilada com `TIBIA_LICENSE_ID 3u`.

## Revogar

`"revoked": true` (ou apague a entrada) → commit + push. O Worker para de
entregar o token e o servidor dele para em até `leaseHours`.

## Deploy do Worker (uma vez)

```
cd worker
npx wrangler@3.114.0 login
npx wrangler@3.114.0 deploy
npx wrangler@3.114.0 secret put MASTER_KEY < C:/tmp/licenses/master.key
```
Plano grátis do Cloudflare: 100 mil pedidos/dia (cada servidor faz 24/dia).

## Chave mestre

`tools/gen_master_key.py` gera uma vez. Fica no secret do Worker e num backup offline,
**nunca** no git. Perder a chave mestre = recompilar e redistribuir todas as
DLLs com uma chave nova. Vazar a chave mestre = qualquer um emite licenças.

## Teste local

```
python tools/sign_tokens.py --master-key C:\tmp\licenses\master.key --out C:\tmp\licenses\tokens
```
