use std::collections::HashMap;
use std::sync::LazyLock;

static CATEGORY_MAP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // Network
    for k in ["DefaultPort", "UDPPort", "PingLimit", "SteamPort1", "SteamPort2"] {
        m.insert(k, "Network");
    }
    // Access
    for k in ["Password", "Open", "MaxPlayers", "MaxAccountsPerUser", "AllowCoop",
              "AutoCreateUserInWhiteList", "DropOffWhiteListAfterDeath",
              "DenyLoginOnOverloadedServer"] {
        m.insert(k, "Access");
    }
    // Server Info
    for k in ["Public", "PublicName", "PublicDescription", "ServerWelcomeMessage",
              "ServerImageLoginScreen", "ServerImageLoadingScreen", "ServerImageIcon"] {
        m.insert(k, "Server Info");
    }
    // PVP
    for k in ["PVP", "PVPLogToolChat", "PVPLogToolFile", "SafetySystem", "ShowSafety",
              "SafetyToggleTimer", "SafetyCooldownTimer", "SafetyDisconnectDelay"] {
        m.insert(k, "PVP");
    }
    // Safehouse
    for k in ["PlayerSafehouse", "AdminSafehouse", "SafehouseAllowTrepass",
              "SafehouseAllowFire", "SafehouseAllowLoot", "SafehouseAllowRespawn",
              "SafehouseDaySurvivedToClaim", "SafeHouseRemovalTime",
              "SafehouseAllowNonResidential", "SafehouseDisableDisguises",
              "SafehousePreventsLootRespawn", "MaxSafezoneSize"] {
        m.insert(k, "Safehouse");
    }
    // Communication
    for k in ["GlobalChat", "ChatStreams", "DiscordEnable", "DiscordToken",
              "DiscordChannel", "DiscordChannelID", "WebhookAddress",
              "DisplayUserName", "ShowFirstAndLastName", "AnnounceDeath",
              "AnnounceAnimalDeath"] {
        m.insert(k, "Communication");
    }
    // Gameplay
    for k in ["PauseEmpty", "NoFire", "SpawnPoint", "SpawnItems",
              "AllowDestructionBySledgehammer", "SledgehammerOnlyInSafehouse",
              "UsernameDisguises", "HideDisguisedUserName",
              "SwitchZombiesOwnershipEachUpdate", "SaveWorldEveryMinutes"] {
        m.insert(k, "Gameplay");
    }
    // Mods
    for k in ["Mods", "WorkshopItems", "Map", "DoLuaChecksum"] {
        m.insert(k, "Mods");
    }
    // Admin
    for k in ["RCONPort", "RCONPassword", "ServerPlayerID", "ResetID"] {
        m.insert(k, "Admin");
    }
    // War
    for k in ["WarStartDelay", "WarDuration", "WarSafehouseHitPoints"] {
        m.insert(k, "War");
    }
    m
});

pub fn get_category(key: &str) -> &'static str {
    CATEGORY_MAP.get(key).copied().unwrap_or("Other")
}
