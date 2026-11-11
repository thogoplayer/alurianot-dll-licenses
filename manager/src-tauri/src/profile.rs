//! Buyer profile: the data compiled into a buyer's ddraw.dll. Stored locally
//! as <keys>\profiles\<name>-<id>.json (never in the public repo).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Technical compatibility flags: must match the buyer's server and dat/spr.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Compat {
    pub mount_system: bool,
    pub health_u32: bool,
    pub mana_u32: bool,
    pub effects_u16: bool,
    pub shoots_u16: bool,
    pub outfit_count_u16: bool,
    pub outfit_limit: u32,
    pub extended: bool,
    pub frame_groups: bool,
    pub dat_frame_groups: bool,
    pub improved_animations: bool,
    pub dat_improved_animations: bool,
    pub custom_ping: bool,
    pub custom_client_os: bool,
    pub multi_client: bool,
    pub hires_timer: bool,
    pub alpha: bool,
    pub manabar: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub server_host: String,
    pub server_name: String,
    pub login_store_dir: String,
    pub website_url: String,
    pub discord_url: String,
    pub forgot_password_url: String,
    pub create_account_url: String,
    pub store_image_url: String,
    pub updater_url: String,
    pub discord_app_id: String,
    pub discord_large_image: String,
    pub discord_small_image: String,
    pub discord_small_image_text: String,
    pub discord_button1_label: String,
    pub discord_button1_url: String,
    pub discord_button2_label: String,
    pub discord_button2_url: String,
    pub compat: Compat,
}

impl Default for Compat {
    fn default() -> Self {
        Compat {
            mount_system: true,
            health_u32: true,
            mana_u32: true,
            effects_u16: true,
            shoots_u16: true,
            outfit_count_u16: true,
            outfit_limit: 255,
            extended: true,
            frame_groups: true,
            dat_frame_groups: true,
            improved_animations: true,
            dat_improved_animations: true,
            custom_ping: true,
            custom_client_os: true,
            multi_client: true,
            hires_timer: true,
            alpha: false,
            manabar: true,
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            server_host: String::new(),
            server_name: String::new(),
            login_store_dir: String::new(),
            website_url: String::new(),
            discord_url: String::new(),
            forgot_password_url: String::new(),
            create_account_url: String::new(),
            store_image_url: String::new(),
            updater_url: String::new(),
            discord_app_id: String::new(),
            discord_large_image: "logo".into(),
            discord_small_image: String::new(),
            discord_small_image_text: String::new(),
            discord_button1_label: "Website".into(),
            discord_button1_url: String::new(),
            discord_button2_label: "Discord".into(),
            discord_button2_url: String::new(),
            compat: Compat::default(),
        }
    }
}

/// The C++ flag behind each compat field (inline constexpr in config.h).
pub const COMPAT_FLAGS: [(&str, &str); 17] = [
    ("mountSystem", "should_use_mount_system"),
    ("healthU32", "should_use_player_health_u32"),
    ("manaU32", "should_use_player_mana_u32"),
    ("effectsU16", "should_use_magic_effects_u16"),
    ("shootsU16", "should_use_distance_shoots_u16"),
    ("outfitCountU16", "should_use_outfit_count_u16"),
    ("extended", "should_use_extended"),
    ("frameGroups", "should_use_frame_groups"),
    ("datFrameGroups", "dat_has_frame_groups"),
    ("improvedAnimations", "should_use_improved_animations"),
    ("datImprovedAnimations", "dat_has_improved_animations"),
    ("customPing", "should_use_custom_ping"),
    ("customClientOs", "should_use_custom_client_os"),
    ("multiClient", "should_allow_multi_client"),
    ("hiresTimer", "should_use_hirestimer"),
    ("alpha", "should_use_alpha"),
    ("manabar", "should_draw_manabar"),
];

impl Compat {
    pub fn get(&self, field: &str) -> bool {
        match field {
            "mountSystem" => self.mount_system,
            "healthU32" => self.health_u32,
            "manaU32" => self.mana_u32,
            "effectsU16" => self.effects_u16,
            "shootsU16" => self.shoots_u16,
            "outfitCountU16" => self.outfit_count_u16,
            "extended" => self.extended,
            "frameGroups" => self.frame_groups,
            "datFrameGroups" => self.dat_frame_groups,
            "improvedAnimations" => self.improved_animations,
            "datImprovedAnimations" => self.dat_improved_animations,
            "customPing" => self.custom_ping,
            "customClientOs" => self.custom_client_os,
            "multiClient" => self.multi_client,
            "hiresTimer" => self.hires_timer,
            "alpha" => self.alpha,
            "manabar" => self.manabar,
            _ => false,
        }
    }

    fn set(&mut self, field: &str, v: bool) {
        match field {
            "mountSystem" => self.mount_system = v,
            "healthU32" => self.health_u32 = v,
            "manaU32" => self.mana_u32 = v,
            "effectsU16" => self.effects_u16 = v,
            "shootsU16" => self.shoots_u16 = v,
            "outfitCountU16" => self.outfit_count_u16 = v,
            "extended" => self.extended = v,
            "frameGroups" => self.frame_groups = v,
            "datFrameGroups" => self.dat_frame_groups = v,
            "improvedAnimations" => self.improved_animations = v,
            "datImprovedAnimations" => self.dat_improved_animations = v,
            "customPing" => self.custom_ping = v,
            "customClientOs" => self.custom_client_os = v,
            "multiClient" => self.multi_client = v,
            "hiresTimer" => self.hires_timer = v,
            "alpha" => self.alpha = v,
            "manabar" => self.manabar = v,
            _ => {}
        }
    }
}

/// First `#define NAME "value"` in config.h (C escapes \" and \\ undone).
fn define_string(config: &str, name: &str) -> Option<String> {
    let prefix = format!("#define {name} \"");
    let line = config.lines().find(|l| l.trim_start().starts_with(&prefix))?;
    let start = line.find('"')? + 1;
    let body = &line[start..line.rfind('"')?];
    Some(body.replace("\\\"", "\"").replace("\\\\", "\\"))
}

/// Value of `inline constexpr bool NAME = true|false;`.
fn flag(config: &str, name: &str) -> Option<bool> {
    let prefix = format!("inline constexpr bool {name} = ");
    let line = config.lines().find(|l| l.trim_start().starts_with(&prefix))?;
    Some(line.contains("= true"))
}

/// Starting values for a new buyer: the owner's current config.h.
pub fn from_config(config: &str) -> Profile {
    let mut p = Profile::default();
    let s = |name: &str, fallback: &str| define_string(config, name).unwrap_or_else(|| fallback.to_string());
    p.server_host = s("TIBIA_BOOTSTRAP_DOMAIN", "");
    p.server_name = s("TIBIA_SERVER_DISPLAY_NAME", "");
    p.login_store_dir = s("TIBIA_LOGIN_STORE_DIR", "");
    p.website_url = s("TIBIA_WEBSITE_URL", "");
    p.discord_url = s("TIBIA_DISCORD_URL", "");
    p.forgot_password_url = s("TIBIA_FORGOT_PASSWORD_URL", "");
    p.create_account_url = s("TIBIA_CREATE_ACCOUNT_URL", "");
    p.store_image_url = s("TIBIA_STORE_IMAGE_URL", "");
    p.updater_url = s("TIBIA_UPDATER_URL", "");
    p.discord_app_id = s("TIBIA_DISCORD_APP_ID", "");
    p.discord_large_image = s("TIBIA_DISCORD_LARGE_IMAGE", "logo");
    p.discord_small_image = s("TIBIA_DISCORD_SMALL_IMAGE", "");
    p.discord_small_image_text = s("TIBIA_DISCORD_SMALL_IMAGE_TEXT", "");
    p.discord_button1_label = s("TIBIA_DISCORD_RPC_BUTTON1_LABEL", "Website");
    p.discord_button1_url = define_string(config, "TIBIA_DISCORD_RPC_BUTTON1_URL").unwrap_or_else(|| p.website_url.clone());
    p.discord_button2_label = s("TIBIA_DISCORD_RPC_BUTTON2_LABEL", "Discord");
    p.discord_button2_url = define_string(config, "TIBIA_DISCORD_RPC_BUTTON2_URL").unwrap_or_else(|| p.discord_url.clone());
    for (field, cpp) in COMPAT_FLAGS {
        if let Some(v) = flag(config, cpp) {
            p.compat.set(field, v);
        }
    }
    let limit_prefix = "inline constexpr unsigned int should_outfit_limit = ";
    if let Some(line) = config.lines().find(|l| l.trim_start().starts_with(limit_prefix)) {
        if let Ok(v) = line.trim_start()[limit_prefix.len()..].trim_end_matches(';').trim().parse() {
            p.compat.outfit_limit = v;
        }
    }
    p
}

pub fn path(keys_dir: &Path, name: &str, id: u32) -> PathBuf {
    keys_dir.join("profiles").join(format!("{name}-{id}.json"))
}

/// Saved profile, or None when this licence has none yet.
pub fn load(keys_dir: &Path, name: &str, id: u32) -> Option<Profile> {
    let text = std::fs::read_to_string(path(keys_dir, name, id)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(keys_dir: &Path, name: &str, id: u32, profile: &Profile) -> Result<(), String> {
    let p = path(keys_dir, name, id);
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    std::fs::write(&p, text + "\n").map_err(|e| format!("erro ao gravar {}: {e}", p.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/config.h");

    #[test]
    fn from_config_reads_current_values() {
        let p = from_config(FIXTURE);
        assert_eq!(p.server_host, "192.0.2.10");
        assert_eq!(p.server_name, "AlurianOT");
        assert_eq!(p.login_store_dir, "AlurianOT");
        assert_eq!(p.store_image_url, "http://192.0.2.10:8080/images/store/");
        assert_eq!(p.updater_url, "http://192.0.2.10:8080/updates/");
        assert_eq!(p.discord_app_id, "000000000000000000");
        assert_eq!(p.discord_large_image, "logo");
        assert_eq!(p.discord_button1_label, "Website");
        assert!(p.compat.mount_system && p.compat.health_u32 && p.compat.extended);
        assert!(!p.compat.alpha);
        assert_eq!(p.compat.outfit_limit, 255);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path(), "x", 3).is_none());
        let mut p = from_config(FIXTURE);
        p.server_host = "play.fulano.com".into();
        p.compat.alpha = true;
        save(dir.path(), "x", 3, &p).unwrap();
        assert_eq!(load(dir.path(), "x", 3).unwrap(), p);
    }
}
