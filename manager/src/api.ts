// Typed wrappers for the Rust commands (src-tauri/src/commands.rs).
import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  repoDir: string;
  keysDir: string;
  workerBase: string;
}

export interface LicenseView {
  id: number;
  name: string;
  status: "ativa" | "revogada" | "vencida";
  expires: string | null;
  note: string;
  hasKey: boolean;
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

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listLicenses: () => invoke<LicenseView[]>("list_licenses"),
  pullLatest: () => invoke<void>("pull_latest"),
  createLicense: (name: string, expires: string | null, note: string) =>
    invoke<OpResult>("create_license", { name, expires, note }),
  setRevoked: (id: number, revoked: boolean) => invoke<OpResult>("set_revoked", { id, revoked }),
  deleteLicense: (id: number) => invoke<OpResult>("delete_license", { id }),
  retryPush: () => invoke<OpResult>("retry_push"),
  pendingPush: () => invoke<boolean>("pending_push"),
  configText: (id: number) => invoke<string>("config_text", { id }),
  prepareHandoff: (id: number) => invoke<string>("prepare_handoff", { id }),
  openKeyFolder: (id: number) => invoke<void>("open_key_folder", { id }),
  tokenStatus: (id: number) => invoke<TokenStatus>("token_status", { id }),
};
