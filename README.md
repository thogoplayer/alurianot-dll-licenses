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

## App de licenças (manager/)

App de desktop (Tauri) que faz tudo isso com cliques: lista as licenças,
cria (gera a chave do servidor, escolhe o próximo id, atualiza
`licenses.json`, faz commit e push), revoga/reativa, apaga, mostra as linhas
do `config.lua`, prepara a pasta de envio (`dll_license.key` + `LEIA-ME.txt`)
e pergunta ao Worker até quando o token vale.

- Rodar: `manager\src-tauri\target\release\alurianot-license-manager.exe`
  (ou instalar com o `.exe` em `target\release\bundle\nsis\`).
- Padrões (botão Configurações): repo `Documentos\GitHub\alurianot-dll-licenses`,
  chaves `C:\tmp\licenses`, Worker `alurianot-dll-licenses...workers.dev`.
- Usa o git já logado no PC; commita **só** `licenses.json`. Se o push
  falhar, a mudança fica salva no PC e aparece "Tentar enviar de novo".
- Ids nunca são reutilizados (`retiredIds`): a DLL de um comprador é
  compilada para um id fixo.
- Apagar não destrói a chave: ela vai para `C:\tmp\licenses\deleted\`.
- Nunca lê nem usa a chave mestra.

Desenvolvimento: `cd manager && npm install && npm run tauri dev`;
testes: `cargo test --manifest-path manager/src-tauri/Cargo.toml`.

### Recursos por licença (venda à la carte)

Cada licença tem uma lista `features` (Store, Market, Forge, Supply Stash,
Task Hunt, Daily Reward, Hunting, ícones de criatura, emblemas de guild,
Skills, colunas laterais, bolsa da Store, tela de login, Discord RPC,
Montaria, Auto Updater). No app: **Editar** → marcar/desmarcar → Salvar.
O Worker entrega um token v2 (`/v2/tokens/<id>.tok`) com os recursos
assinados pela chave mestra; a DLL do comprador lê esse token ao abrir.
Muda na **próxima vez que o cliente da pessoa abrir**. O caminho v1
(`/tokens/<id>.tok`, usado pelo servidor) não mudou.

### Gerar DLL do comprador

**Gerar DLL** na linha da licença: preencha o perfil (IP, nome, links,
Discord RPC, compatibilidade com o servidor/dat dele) → **Salvar e gerar
DLL**. Sai em `C:\tmp\licenses\builds\<nome>-<id>\` (ddraw.dll, Tibia.exe,
Tibia.pic, Tibia.otfi, LEIA-ME). Precisa do Visual Studio (MSBuild) e da
pasta `AlurianOt-ClientOld` (Configurações). Cada comprador ganha uma chave
de updater própria (`<nome>-<id>.updater.key`); **Enviar updater** monta a
pasta com a chave e a ferramenta de publicar, para quem comprou o Auto
Updater. O comprador precisa de servidor com a licença (TFS 1.x com
`dll_license.cpp`).
