// Sanitised copy of AlurianOt-ClientOld/dll/src/config.h for build.rs tests
// (keys zeroed, addresses replaced). Keep in sync when config.h gains fields.
#ifndef TIBIA_CONFIG_H
#define TIBIA_CONFIG_H

// Base URL of the Store offer and banner images (the epic client used
// http://store.epicwarserver.com/images/store/). The icon name sent by the
// server is appended (".png" is added when it has no extension). Override it
// in the project settings for production without changing the source file.
#ifndef TIBIA_STORE_IMAGE_URL
// AlurianOT local dev: AlurianOt-Web\images\store served from the WSL2 host.
#define TIBIA_STORE_IMAGE_URL "http://192.0.2.10:8080/images/store/"
#endif

// Login screen (login_screen.cpp) links. An empty URL disables the link and
// the screen says it is not configured. Override them in the project settings
// for production without changing the source file.
#ifndef TIBIA_DISCORD_URL
#define TIBIA_DISCORD_URL ""
#endif
#ifndef TIBIA_WEBSITE_URL
#define TIBIA_WEBSITE_URL ""
#endif
#ifndef TIBIA_FORGOT_PASSWORD_URL
#define TIBIA_FORGOT_PASSWORD_URL ""
#endif
#ifndef TIBIA_CREATE_ACCOUNT_URL
#define TIBIA_CREATE_ACCOUNT_URL ""
#endif
// Server name shown in the login box until the status protocol answers
// (then the server's own serverName is shown).
#ifndef TIBIA_SERVER_DISPLAY_NAME
#define TIBIA_SERVER_DISPLAY_NAME "AlurianOT"
#endif
// Folder under %LOCALAPPDATA% that holds the DPAPI-encrypted remembered
// login (login.dat).
#ifndef TIBIA_LOGIN_STORE_DIR
#define TIBIA_LOGIN_STORE_DIR "AlurianOT"
#endif
// Discord Rich Presence (discord_presence.h). Paste the Application ID from
// the Discord Developer Portal (digits only); empty disables the feature.
// The image values are asset keys uploaded under Rich Presence > Art Assets;
// an empty small image is left out.
#ifndef TIBIA_DISCORD_APP_ID
// AlurianOT application (public client id, not a secret).
#define TIBIA_DISCORD_APP_ID "000000000000000000"
#endif
#ifndef TIBIA_DISCORD_LARGE_IMAGE
#define TIBIA_DISCORD_LARGE_IMAGE "logo"
#endif
#ifndef TIBIA_DISCORD_SMALL_IMAGE
#define TIBIA_DISCORD_SMALL_IMAGE ""
#endif
#ifndef TIBIA_DISCORD_SMALL_IMAGE_TEXT
#define TIBIA_DISCORD_SMALL_IMAGE_TEXT ""
#endif
// Up to two Rich Presence buttons. A button appears as soon as its URL is a
// full http:// or https:// link; an empty or invalid URL hides it (Discord
// refuses the whole presence with a bad button). Labels: 1..32 characters.
// By default they follow the login screen's Website / Discord links above.
#ifndef TIBIA_DISCORD_RPC_BUTTON1_LABEL
#define TIBIA_DISCORD_RPC_BUTTON1_LABEL "Website"
#endif
#ifndef TIBIA_DISCORD_RPC_BUTTON1_URL
#define TIBIA_DISCORD_RPC_BUTTON1_URL TIBIA_WEBSITE_URL
#endif
#ifndef TIBIA_DISCORD_RPC_BUTTON2_LABEL
#define TIBIA_DISCORD_RPC_BUTTON2_LABEL "Discord"
#endif
#ifndef TIBIA_DISCORD_RPC_BUTTON2_URL
#define TIBIA_DISCORD_RPC_BUTTON2_URL TIBIA_DISCORD_URL
#endif

// === BEGIN DLL LICENCE ===
// Server licence lock (license_guard.cpp), see the alurianot-dll-licenses repo.
// After login the DLL sends a nonce; the server answers with
//   * a lease token signed by the AlurianOT MASTER key (from the licences
//     repo, re-signed twice a day, valid leaseHours = 48 h), naming the game
//     server's own public key, the licence id and an expiry time, and
//   * a signature over domainTag || nonce || licenceId made with that game
//     server's private key.
// The DLL only holds the master public key. A revoked licence stops getting
// tokens and stops working when its last token expires.
//
// TIBIA_LICENSE_ID is the only licence this build accepts (1 = AlurianOT,
// 2 = tharcio, ...); per-build scripts override it. The domain/lease tags must
// byte-for-byte match the server (dll_license.h) and lease_token.py.
#ifndef TIBIA_LICENSE_DOMAIN_TAG
#define TIBIA_LICENSE_DOMAIN_TAG "AlurianOT-DLL-License-v1"
#endif
#ifndef TIBIA_LICENSE_LEASE_TAG
#define TIBIA_LICENSE_LEASE_TAG "AlurianOT-DLL-Lease-v1"
#endif
#ifndef TIBIA_LICENSE_ID
#define TIBIA_LICENSE_ID 1u
#endif
#ifndef TIBIA_LICENSE_MASTER_PUBLIC_KEY
#define TIBIA_LICENSE_MASTER_PUBLIC_KEY \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00
#endif
// Sellable features (features.h). 0 = official build: every compiled feature
// on, nothing fetched. The License Manager sets 1 in buyer builds: features
// then come from the master-signed v2 token of TIBIA_LICENSE_ID.
#ifndef TIBIA_FEATURE_GATING
#define TIBIA_FEATURE_GATING 0
#endif
#ifndef TIBIA_LICENSE_WORKER_URL
#define TIBIA_LICENSE_WORKER_URL "https://alurianot-dll-licenses.thiago-freitasferraz.workers.dev"
#endif

// === END DLL LICENCE ===

// Auto updater (auto_updater.h, tools/updater/README.md). The DLL downloads
// <URL>manifest.json + manifest.sig at start; only a manifest signed with the
// updater key (C:\tmp\licenses\updater.key, matching the public key below)
// is used. Must end with '/'. Site offline = the client starts as it is.
#ifndef TIBIA_UPDATER_URL
#define TIBIA_UPDATER_URL "http://192.0.2.10:8080/updates/"
#endif
#ifndef TIBIA_UPDATER_PRODUCT
#define TIBIA_UPDATER_PRODUCT "AlurianOT"
#endif
#ifndef TIBIA_UPDATER_PUBLIC_KEY
#define TIBIA_UPDATER_PUBLIC_KEY \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, \
    0x00, 0x00, 0x00, 0x00
#endif

#ifdef __CONFIG__
// Bootstrap host used for the initial login connection. Public builds encrypt
// this literal in the binary. Override TIBIA_BOOTSTRAP_DOMAIN in the project
// settings for production without changing the source file.
#ifndef TIBIA_BOOTSTRAP_DOMAIN
// AlurianOT local dev: WSL2 server address (changes when WSL restarts).
#define TIBIA_BOOTSTRAP_DOMAIN "192.0.2.10"
#endif

// Fixed production profile. Keeping the packet layout inside the binary avoids
// an external file changing protocol widths after the DLL was built.
inline constexpr bool should_use_hirestimer = true;
inline constexpr bool should_use_extended = true;
inline constexpr bool should_use_alpha = false;
inline constexpr bool should_draw_manabar = true;
inline constexpr bool should_use_frame_groups = true;
inline constexpr bool should_use_improved_animations = true;
inline constexpr bool dat_has_frame_groups = true;
inline constexpr bool dat_has_improved_animations = true;
inline constexpr bool should_use_mount_system = true;
inline constexpr bool should_use_magic_effects_u16 = true;
inline constexpr bool should_use_distance_shoots_u16 = true;
inline constexpr bool should_use_player_health_u32 = true;
inline constexpr bool should_use_player_mana_u32 = true;
inline constexpr bool should_use_outfit_count_u16 = true;
inline constexpr bool should_use_custom_ping = true;
inline constexpr bool should_consume_impact_tracker = true;
inline constexpr bool should_use_custom_client_os = true;
// Lets several clients run at once (skips the TibiaPlayerMutex check).
inline constexpr bool should_allow_multi_client = true;
inline constexpr bool should_debug_mount = false;
inline constexpr bool should_debug_frame_groups = false;
inline constexpr bool should_debug_store = false;
inline constexpr bool should_use_store = true;
// Exaltation Forge window (opcode 0x00) and item tier bytes (opcode 0x00,
// negotiated with the server; see item_tier.h).
inline constexpr bool should_use_forge = true;
inline constexpr bool should_use_item_tiers = true;
// Market window (server opcode 0x00, see market_window.h).
inline constexpr bool should_use_market = true;
// Supply Stash window (opcodes 0x00/0x00, see stash_window.h).
inline constexpr bool should_use_supply_stash = true;
// Task Hunt window: Bounty/Weekly Tasks, Hunting Task Shop (server opcode
// 0x00, client opcode 0x00, see task_window.h).
inline constexpr bool should_use_task_hunt = true;
// Influenced/fiendish creature icons beside the health bar and the fiendish
// orange name halo (server opcode 0x00, see creature_icons.h).
inline constexpr bool should_use_creature_icons = true;
// Daily Reward / Reward Wall window (server opcodes 0x00/0x00/0x00/0x00,
// client opcodes 0x00/0x00/0x00, see daily_reward_window.h).
inline constexpr bool should_use_daily_reward = true;
// Guild membership emblems 4 (own guild, gold) and 5 (other guild, silver)
// beside the native 1..3 (see guild_emblems.h).
inline constexpr bool should_use_guild_emblems = true;
// Epic-style 9x9 skill icons beside the Skills window bars (see
// skill_icons.h; art = skin 378, already in the epic Tibia.pic atlas).
inline constexpr bool should_use_skill_icons = true;
// AstraClient Skills rows (XP Gain Rate, XP Boost, Speed, Regeneration,
// Offline Training) from the DLL-only server opcode 0x00 (see skill_rows.h).
inline constexpr bool should_use_skill_rows = true;
// AstraClient green "XP Boost" buy button on the Skills window (needs the
// skill rows and the Store; art = skin 379, see xp_boost_button.h).
inline constexpr bool should_use_xp_boost_button = true;
// OTClient-style extra side columns: arrows at the top of the game view add
// or remove columns of mini windows on both sides (see side_columns.h).
inline constexpr bool should_use_side_columns = true;
// "Purse" button above the inventory backpack slot (opens the Store inbox).
inline constexpr bool should_use_store_inbox_purse = true;
inline constexpr bool should_use_smart_walking = true;
inline constexpr bool should_use_dynamic_endpoint = true;
inline constexpr bool should_use_login_screen = true;
// Discord Rich Presence over the local IPC pipe; also needs
// TIBIA_DISCORD_APP_ID above (see discord_presence.h).
inline constexpr bool should_use_discord_rpc = true;
// Server licence lock (license_guard.cpp): the DLL demands a signed proof from
// the server after login and refuses to play without it (client opcode 0x00,
// server opcode 0x00). Keep this true in every distributed build.
inline constexpr bool should_use_license_guard = true;
// Signed auto updater at start-up (see TIBIA_UPDATER_URL above). A file named
// noupdate.txt next to Tibia.exe turns it off on that machine (developers).
inline constexpr bool should_use_auto_updater = true;
inline constexpr unsigned int should_outfit_limit = 255;
#endif

#endif
