import { api, Compat, FEATURES, LicenseView, OpResult, Profile, Settings, TokenStatus } from "./api";

const app = document.querySelector<HTMLDivElement>("#app")!;

app.innerHTML = `
  <header>
    <h1>AlurianOT <span>— Licenças</span></h1>
    <div class="actions">
      <button id="btn-new" class="primary">+ Nova licença</button>
      <button id="btn-refresh">Atualizar</button>
      <button id="btn-settings">Configurações</button>
    </div>
  </header>
  <div id="banner" class="banner hidden">
    <span>Alterações não enviadas ao GitHub.</span>
    <button id="btn-retry">Tentar enviar de novo</button>
  </div>
  <main>
    <table>
      <thead>
        <tr><th>Id</th><th>Nome</th><th>Status</th><th>Validade</th><th>Recursos</th><th>Nota</th><th>Token</th><th>Ações</th></tr>
      </thead>
      <tbody id="rows"><tr><td colspan="8" class="muted">Carregando...</td></tr></tbody>
    </table>
  </main>
  <div id="modal" class="modal hidden"><div class="dialog" id="dialog"></div></div>
  <div id="toast" class="toast hidden"></div>
`;

const $ = <T extends HTMLElement>(sel: string) => document.querySelector<T>(sel)!;
const rows = $("#rows");
const banner = $("#banner");
const modal = $("#modal");
const dialog = $("#dialog");
const toast = $("#toast");
let busy = false;

function esc(text: string): string {
  return text.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}

function brDate(iso: string | null): string {
  if (!iso) return "sem validade";
  const [y, m, d] = iso.split("-");
  return `${d}/${m}/${y}`;
}

let toastTimer = 0;
function showToast(text: string, kind: "ok" | "error" = "ok") {
  toast.textContent = text;
  toast.className = `toast ${kind}`;
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toast.classList.add("hidden"), kind === "error" ? 9000 : 4000);
}

function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}

let lastPushFailed = false;
async function refreshBanner() {
  try {
    banner.classList.toggle("hidden", !(lastPushFailed || (await api.pendingPush())));
  } catch {
    banner.classList.toggle("hidden", !lastPushFailed);
  }
}

function report(result: OpResult) {
  if (result.pushed) {
    showToast(result.message);
  } else {
    showToast(`Salvo no PC, mas não enviado ao GitHub: ${result.message}`, "error");
    banner.classList.remove("hidden");
  }
}

/** Runs one change at a time, then reloads the list. */
async function run(action: () => Promise<OpResult>) {
  if (busy) return;
  busy = true;
  document.body.classList.add("busy");
  try {
    const result = await action();
    lastPushFailed = !result.pushed;
    report(result);
  } catch (e) {
    showToast(errorText(e), "error");
  } finally {
    busy = false;
    document.body.classList.remove("busy");
    await load();
  }
}

function tokenText(s: TokenStatus): string {
  switch (s.kind) {
    case "served": {
      const d = new Date(s.expiresAt * 1000);
      const pad = (n: number) => String(n).padStart(2, "0");
      return `até ${pad(d.getDate())}/${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
    }
    case "notServed":
      return "não servido";
    case "invalid":
      return "resposta inválida";
    case "unreachable":
      return "sem resposta";
  }
}

function render(list: LicenseView[]) {
  if (list.length === 0) {
    rows.innerHTML = `<tr><td colspan="8" class="muted">Nenhuma licença.</td></tr>`;
    return;
  }
  rows.innerHTML = list
    .map(
      (l) => `
    <tr data-id="${l.id}">
      <td class="id">#${l.id}</td>
      <td class="name">${esc(l.name)}</td>
      <td><span class="badge ${l.status}">${l.status}</span></td>
      <td>${brDate(l.expires)}</td>
      <td class="features" title="${esc(featureLabels(l.features))}">${l.features.length}/${FEATURES.length}</td>
      <td class="note">${esc(l.note)}</td>
      <td class="token"><button class="small" data-act="token">Verificar</button></td>
      <td class="row-actions">
        <button class="small" data-act="${l.status === "revogada" ? "reactivate" : "revoke"}">${l.status === "revogada" ? "Reativar" : "Revogar"}</button>
        <button class="small" data-act="edit">Editar</button>
        <button class="small" data-act="build">Gerar DLL</button>
        <button class="small" data-act="config">Config</button>
        <button class="small" data-act="send" ${l.hasKey ? "" : 'disabled title="A chave desta licença não está neste PC."'}>Enviar</button>
        <button class="small danger" data-act="delete">Apagar</button>
      </td>
    </tr>`,
    )
    .join("");
  current = list;
}

let current: LicenseView[] = [];

async function load() {
  try {
    render(await api.listLicenses());
  } catch (e) {
    rows.innerHTML = `<tr><td colspan="8" class="error-cell">${esc(errorText(e))}</td></tr>`;
  }
  await refreshBanner();
}

function openModal(html: string) {
  dialog.innerHTML = html;
  modal.classList.remove("hidden");
  dialog.querySelector<HTMLElement>("input, button")?.focus();
}

function closeModal() {
  modal.classList.add("hidden");
  dialog.innerHTML = "";
}

modal.addEventListener("mousedown", (e) => {
  if (e.target === modal) closeModal();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !modal.classList.contains("hidden")) closeModal();
});

function featureLabels(ids: string[]): string {
  return FEATURES.filter((f) => ids.includes(f.id)).map((f) => f.label).join(", ") || "nenhum recurso";
}

function featureChecks(selected: string[]): string {
  return FEATURES.map(
    (f) => `<label class="check"><input type="checkbox" name="feature" value="${f.id}" ${selected.includes(f.id) ? "checked" : ""} /> ${esc(f.label)}</label>`,
  ).join("");
}

function checkedFeatures(form: HTMLFormElement): string[] {
  return [...form.querySelectorAll<HTMLInputElement>('input[name="feature"]:checked')].map((i) => i.value);
}

function editDialog(l: LicenseView) {
  openModal(`
    <h2>Editar — ${esc(l.name)} (#${l.id})</h2>
    <form id="form-edit">
      <fieldset class="features"><legend>Recursos</legend>${featureChecks(l.features)}</fieldset>
      <div class="feature-tools">
        <button type="button" data-all>Marcar todos</button>
        <button type="button" data-none>Desmarcar todos</button>
      </div>
      <label>Validade <small>(vence depois deste dia; vazio = sem validade)</small>
        <input name="expires" type="date" value="${l.expires ?? ""}" /></label>
      <p class="muted">Recursos ligam/desligam na próxima vez que o cliente da pessoa abrir.</p>
      <div class="dialog-actions">
        <button type="button" data-close>Cancelar</button>
        <button type="submit" class="primary">Salvar</button>
      </div>
    </form>`);
  const form = dialog.querySelector<HTMLFormElement>("#form-edit")!;
  form.querySelector("[data-all]")!.addEventListener("click", () =>
    form.querySelectorAll<HTMLInputElement>('input[name="feature"]').forEach((i) => (i.checked = true)));
  form.querySelector("[data-none]")!.addEventListener("click", () =>
    form.querySelectorAll<HTMLInputElement>('input[name="feature"]').forEach((i) => (i.checked = false)));
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    const features = checkedFeatures(form);
    const expires = String(new FormData(form).get("expires") ?? "") || null;
    const featuresChanged = features.join(",") !== l.features.join(",");
    const expiresChanged = expires !== l.expires;
    closeModal();
    if (!featuresChanged && !expiresChanged) {
      showToast("Nada mudou.");
      return;
    }
    run(async () => {
      let result: OpResult = { pushed: true, message: "" };
      if (featuresChanged) result = await api.setFeatures(l.id, features);
      if (expiresChanged && result.pushed) result = await api.setExpires(l.id, expires);
      return result;
    });
  });
}

const PROFILE_FIELDS: [keyof Profile, string, string][] = [
  ["serverHost", "IP ou domínio do servidor", "ex.: play.fulano.com"],
  ["serverName", "Nome do servidor (login e Discord)", "ex.: Fulano OT"],
  ["loginStoreDir", "Pasta onde o login salva a conta", "ex.: FulanoOT"],
  ["websiteUrl", "Site", "https://..."],
  ["discordUrl", "Discord (convite)", "https://discord.gg/..."],
  ["forgotPasswordUrl", "Esqueci a senha", "https://..."],
  ["createAccountUrl", "Criar conta", "https://..."],
  ["storeImageUrl", "Imagens da Store", "https://.../images/store/"],
  ["updaterUrl", "Auto updater (termina com /)", "https://.../updates/"],
  ["discordAppId", "Discord RPC — App ID", "só números"],
  ["discordLargeImage", "Discord RPC — imagem grande", "logo"],
  ["discordSmallImage", "Discord RPC — imagem pequena", ""],
  ["discordSmallImageText", "Discord RPC — texto da imagem pequena", ""],
  ["discordButton1Label", "Botão 1 — texto", "Website"],
  ["discordButton1Url", "Botão 1 — link", "https://..."],
  ["discordButton2Label", "Botão 2 — texto", "Discord"],
  ["discordButton2Url", "Botão 2 — link", "https://..."],
];

const COMPAT_FIELDS: [keyof Compat, string][] = [
  ["mountSystem", "Sistema de montaria (protocolo)"],
  ["healthU32", "Vida u32"],
  ["manaU32", "Mana u32"],
  ["effectsU16", "Efeitos mágicos u16"],
  ["shootsU16", "Projéteis u16"],
  ["outfitCountU16", "Quantidade de outfits u16"],
  ["extended", "Dat/spr estendido"],
  ["frameGroups", "Frame groups"],
  ["datFrameGroups", "Dat tem frame groups"],
  ["improvedAnimations", "Animações melhoradas"],
  ["datImprovedAnimations", "Dat tem animações melhoradas"],
  ["customPing", "Ping customizado"],
  ["customClientOs", "Client OS customizado"],
  ["multiClient", "Vários clientes abertos"],
  ["hiresTimer", "Timer de alta resolução"],
  ["alpha", "Transparência (spr RGBA)"],
  ["manabar", "Barra de mana"],
];

function readProfile(form: HTMLFormElement, base: Profile): Profile {
  const data = new FormData(form);
  const p: Profile = { ...base, compat: { ...base.compat } };
  for (const [key] of PROFILE_FIELDS) (p as any)[key] = String(data.get(key) ?? "").trim();
  for (const [key] of COMPAT_FIELDS) (p.compat as any)[key] = form.querySelector<HTMLInputElement>(`input[name="c_${key}"]`)!.checked;
  p.compat.outfitLimit = Math.max(1, Math.min(65535, Number(data.get("outfitLimit")) || 255));
  return p;
}

async function buildDialog(l: LicenseView) {
  let profile: Profile;
  try {
    profile = await api.getProfile(l.id);
  } catch (e) {
    showToast(errorText(e), "error");
    return;
  }
  openModal(`
    <h2>Gerar DLL — ${esc(l.name)} (#${l.id})</h2>
    <form id="form-build" class="wide">
      <div class="grid2">
        ${PROFILE_FIELDS.map(([k, label, ph]) => `<label>${esc(label)}<input name="${k}" value="${esc(String(profile[k] ?? ""))}" placeholder="${esc(ph)}" /></label>`).join("")}
      </div>
      <fieldset class="features"><legend>Compatibilidade (tem que bater com o servidor e o dat/spr do comprador)</legend>
        ${COMPAT_FIELDS.map(([k, label]) => `<label class="check"><input type="checkbox" name="c_${k}" ${profile.compat[k] ? "checked" : ""} /> ${esc(label)}</label>`).join("")}
        <label class="check">Limite de outfits <input name="outfitLimit" type="number" min="1" max="65535" value="${profile.compat.outfitLimit}" style="width:80px" /></label>
      </fieldset>
      <p class="muted">Recursos da licença: ${esc(featureLabels(l.features))}. Eles ligam pela licença, não pela DLL.</p>
      <pre id="build-log" class="hidden"></pre>
      <div class="dialog-actions">
        <button type="button" id="btn-updater" ${l.features.includes("auto_updater") ? "" : 'disabled title="Licença sem Auto Updater"'}>Enviar updater</button>
        <button type="button" data-close>Fechar</button>
        <button type="button" id="btn-save-profile">Salvar perfil</button>
        <button type="submit" class="primary" id="btn-build">Salvar e gerar DLL</button>
      </div>
    </form>`);
  const form = dialog.querySelector<HTMLFormElement>("#form-build")!;
  const log = dialog.querySelector<HTMLPreElement>("#build-log")!;
  const save = async () => {
    profile = readProfile(form, profile);
    await api.saveProfile(l.id, profile);
  };
  $("#btn-save-profile").addEventListener("click", async () => {
    try {
      await save();
      showToast("Perfil salvo.");
    } catch (e) {
      showToast(errorText(e), "error");
    }
  });
  $("#btn-updater").addEventListener("click", async () => {
    try {
      const folder = await api.prepareUpdaterHandoff(l.id);
      showToast(`Pasta do updater pronta: ${folder}. Mande em particular.`);
    } catch (e) {
      showToast(errorText(e), "error");
    }
  });
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const button = $<HTMLButtonElement>("#btn-build");
    button.disabled = true;
    button.textContent = "Compilando... (1-3 min)";
    log.classList.add("hidden");
    try {
      await save();
      const report = await api.buildDll(l.id);
      showToast(`DLL pronta: ${report.folder}`);
      closeModal();
    } catch (err) {
      log.textContent = errorText(err);
      log.classList.remove("hidden");
      showToast("A geração da DLL falhou — veja o log na janela.", "error");
      button.disabled = false;
      button.textContent = "Salvar e gerar DLL";
    }
  });
}

function newLicenseDialog() {
  openModal(`
    <h2>Nova licença</h2>
    <form id="form-new">
      <label>Nome <small>(letras sem acento, números, - e _)</small><input name="name" required maxlength="32" placeholder="ex.: fulano" /></label>
      <label>Validade <small>(opcional — vence depois deste dia)</small><input name="expires" type="date" /></label>
      <label>Nota <small>(opcional)</small><input name="note" maxlength="120" placeholder="ex.: servidor de teste do fulano" /></label>
      <fieldset class="features"><legend>Recursos</legend>${featureChecks(["mount"])}</fieldset>
      <div class="dialog-actions">
        <button type="button" data-close>Cancelar</button>
        <button type="submit" class="primary">Criar licença</button>
      </div>
    </form>`);
  dialog.querySelector<HTMLFormElement>("#form-new")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const data = new FormData(e.target as HTMLFormElement);
    const name = String(data.get("name") ?? "");
    const expires = String(data.get("expires") ?? "") || null;
    const note = String(data.get("note") ?? "");
    const features = checkedFeatures(e.target as HTMLFormElement);
    closeModal();
    run(() => api.createLicense(name, expires, note, features));
  });
}

async function settingsDialog() {
  let s: Settings;
  try {
    s = await api.getSettings();
  } catch (e) {
    showToast(errorText(e), "error");
    return;
  }
  openModal(`
    <h2>Configurações</h2>
    <form id="form-settings">
      <label>Pasta do repositório de licenças<input name="repoDir" value="${esc(s.repoDir)}" required /></label>
      <label>Pasta das chaves dos servidores<input name="keysDir" value="${esc(s.keysDir)}" required /></label>
      <label>Endereço do Worker<input name="workerBase" value="${esc(s.workerBase)}" required /></label>
      <label>Pasta do cliente (AlurianOt-ClientOld)<input name="clientDir" value="${esc(s.clientDir)}" required /></label>
      <label>MSBuild.exe<input name="msbuildPath" value="${esc(s.msbuildPath)}" required /></label>
      <p class="muted">A chave mestra nunca é usada por este app.</p>
      <div class="dialog-actions">
        <button type="button" data-close>Cancelar</button>
        <button type="submit" class="primary">Salvar</button>
      </div>
    </form>`);
  dialog.querySelector<HTMLFormElement>("#form-settings")!.addEventListener("submit", async (e) => {
    e.preventDefault();
    const data = new FormData(e.target as HTMLFormElement);
    const next: Settings = {
      repoDir: String(data.get("repoDir")).trim(),
      keysDir: String(data.get("keysDir")).trim(),
      workerBase: String(data.get("workerBase")).trim(),
      clientDir: String(data.get("clientDir")).trim(),
      msbuildPath: String(data.get("msbuildPath")).trim(),
    };
    try {
      await api.saveSettings(next);
      closeModal();
      showToast("Configurações salvas.");
      await load();
    } catch (err) {
      showToast(errorText(err), "error");
    }
  });
}

async function configDialog(l: LicenseView) {
  let text = "";
  try {
    text = await api.configText(l.id);
  } catch (e) {
    showToast(errorText(e), "error");
    return;
  }
  openModal(`
    <h2>Config do servidor — ${esc(l.name)} (#${l.id})</h2>
    <p class="muted">Cole no config.lua do servidor dele (troque SEU_USUARIO).</p>
    <pre id="cfg">${esc(text)}</pre>
    <div class="dialog-actions">
      <button type="button" id="btn-key" ${l.hasKey ? "" : "disabled"}>Abrir pasta da chave</button>
      <button type="button" id="btn-copy" class="primary">Copiar</button>
      <button type="button" data-close>Fechar</button>
    </div>
    ${l.hasKey ? "" : `<p class="muted">A chave desta licença não está neste PC.</p>`}`);
  $("#btn-copy").addEventListener("click", async () => {
    await navigator.clipboard.writeText(text);
    showToast("Config copiada.");
  });
  $("#btn-key").addEventListener("click", () => api.openKeyFolder(l.id).catch((e) => showToast(errorText(e), "error")));
}

function deleteDialog(l: LicenseView) {
  openModal(`
    <h2>Apagar licença</h2>
    <p>Apagar a licença <b>${esc(l.name)}</b> (#${l.id})? O id nunca será reutilizado.</p>
    <p class="muted">A chave vai para a pasta <code>deleted</code> (não é destruída). O servidor dele para em até 48 h.</p>
    <div class="dialog-actions">
      <button type="button" data-close>Cancelar</button>
      <button type="button" id="btn-confirm-delete" class="danger">Apagar</button>
    </div>`);
  $("#btn-confirm-delete").addEventListener("click", () => {
    closeModal();
    run(() => api.deleteLicense(l.id));
  });
}

dialog.addEventListener("click", (e) => {
  if ((e.target as HTMLElement).closest("[data-close]")) closeModal();
});

rows.addEventListener("click", async (e) => {
  const button = (e.target as HTMLElement).closest<HTMLButtonElement>("button[data-act]");
  if (!button) return;
  const id = Number(button.closest<HTMLElement>("tr")!.dataset.id);
  const l = current.find((x) => x.id === id);
  if (!l) return;
  switch (button.dataset.act) {
    case "revoke":
      run(() => api.setRevoked(id, true));
      break;
    case "reactivate":
      run(() => api.setRevoked(id, false));
      break;
    case "edit":
      editDialog(l);
      break;
    case "build":
      buildDialog(l);
      break;
    case "config":
      configDialog(l);
      break;
    case "delete":
      deleteDialog(l);
      break;
    case "send":
      try {
        const folder = await api.prepareHandoff(id);
        showToast(`Pasta pronta: ${folder}. Mande em particular.`);
      } catch (err) {
        showToast(errorText(err), "error");
      }
      break;
    case "token": {
      const cell = button.closest("td")!;
      button.disabled = true;
      button.textContent = "...";
      try {
        const s = await api.tokenStatus(id);
        cell.innerHTML = `<span class="token-${s.kind}" title="${s.kind === "unreachable" ? esc(s.error) : ""}">${tokenText(s)}</span>`;
      } catch (err) {
        cell.innerHTML = `<span class="token-unreachable">${esc(errorText(err))}</span>`;
      }
      break;
    }
  }
});

$("#btn-new").addEventListener("click", newLicenseDialog);
$("#btn-settings").addEventListener("click", settingsDialog);
$("#btn-refresh").addEventListener("click", async () => {
  try {
    await api.pullLatest();
    showToast("Lista atualizada do GitHub.");
  } catch (e) {
    showToast(errorText(e), "error");
  }
  await load();
});
$("#btn-retry").addEventListener("click", () => run(() => api.retryPush()));

load();
