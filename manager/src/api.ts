// Typed wrappers for the Rust commands (src-tauri/src/commands.rs).
import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  repoDir: string;
  keysDir: string;
  workerBase: string;
  clientDir: string;
  msbuildPath: string;
}

export interface Compat {
  mountSystem: boolean;
  healthU32: boolean;
  manaU32: boolean;
  effectsU16: boolean;
  shootsU16: boolean;
  outfitCountU16: boolean;
  outfitLimit: number;
  extended: boolean;
  frameGroups: boolean;
  datFrameGroups: boolean;
  improvedAnimations: boolean;
  datImprovedAnimations: boolean;
  customPing: boolean;
  customClientOs: boolean;
  multiClient: boolean;
  hiresTimer: boolean;
  alpha: boolean;
  manabar: boolean;
}

export interface Profile {
  serverHost: string;
  serverName: string;
  loginStoreDir: string;
  websiteUrl: string;
  discordUrl: string;
  forgotPasswordUrl: string;
  createAccountUrl: string;
  storeImageUrl: string;
  updaterUrl: string;
  discordAppId: string;
  discordLargeImage: string;
  discordSmallImage: string;
  discordSmallImageText: string;
  discordButton1Label: string;
  discordButton1Url: string;
  discordButton2Label: string;
  discordButton2Url: string;
  compat: Compat;
}

export interface BuildReport {
  folder: string;
  logTail: string;
}

export interface LicenseView {
  id: number;
  name: string;
  status: "ativa" | "revogada" | "vencida";
  expires: string | null;
  note: string;
  hasKey: boolean;
  features: string[];
}

export interface OpResult {
  pushed: boolean;
  message: string;
}

export type TokenStatus =
  | { kind: "served"; expiresAt: number }
  | { kind: "notServed" }
  | { kind: "invalid" }
  | { kind: "unreachable"; error: string };

// Sellable DLL features (append-only, same order as licenses.rs FEATURES).
export const FEATURES: { id: string; label: string }[] = [
  { id: "store", label: "Store" },
  { id: "market", label: "Market" },
  { id: "forge", label: "Forge + tiers" },
  { id: "supply_stash", label: "Supply Stash" },
  { id: "task_hunt", label: "Task Hunt" },
  { id: "daily_reward", label: "Daily Reward" },
  { id: "hunting", label: "Hunting" },
  { id: "creature_icons", label: "Ícones de criatura" },
  { id: "guild_emblems", label: "Emblemas de guild" },
  { id: "skills", label: "Skills (ícones, linhas, XP Boost)" },
  { id: "side_columns", label: "Colunas laterais" },
  { id: "store_inbox_purse", label: "Bolsa da Store" },
  { id: "login_screen", label: "Tela de login" },
  { id: "discord_rpc", label: "Discord RPC" },
  { id: "mount", label: "Montaria (brinde)" },
  { id: "auto_updater", label: "Auto Updater" },
];

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listLicenses: () => invoke<LicenseView[]>("list_licenses"),
  pullLatest: () => invoke<void>("pull_latest"),
  createLicense: (name: string, expires: string | null, note: string, features: string[]) =>
    invoke<OpResult>("create_license", { name, expires, note, features }),
  setFeatures: (id: number, features: string[]) => invoke<OpResult>("set_features", { id, features }),
  setExpires: (id: number, expires: string | null) => invoke<OpResult>("set_expires", { id, expires }),
  setRevoked: (id: number, revoked: boolean) => invoke<OpResult>("set_revoked", { id, revoked }),
  deleteLicense: (id: number) => invoke<OpResult>("delete_license", { id }),
  retryPush: () => invoke<OpResult>("retry_push"),
  pendingPush: () => invoke<boolean>("pending_push"),
  configText: (id: number) => invoke<string>("config_text", { id }),
  prepareHandoff: (id: number) => invoke<string>("prepare_handoff", { id }),
  openKeyFolder: (id: number) => invoke<void>("open_key_folder", { id }),
  tokenStatus: (id: number) => invoke<TokenStatus>("token_status", { id }),
  getProfile: (id: number) => invoke<Profile>("get_profile", { id }),
  saveProfile: (id: number, profile: Profile) => invoke<void>("save_profile", { id, profile }),
  buildDll: (id: number) => invoke<BuildReport>("build_dll", { id }),
  prepareUpdaterHandoff: (id: number) => invoke<string>("prepare_updater_handoff", { id }),
};
