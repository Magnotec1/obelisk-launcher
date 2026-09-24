use crate::backend::auth::microsoft::{Account, AccountType};
use crate::backend::instance::groups::{GroupInfo, InstanceGroups};
use crate::backend::instance::manager::{
    Instance, InstanceComponent, ModInfo, WorldInfo,
};
use crate::backend::playtime::{InstancePlaytimeData, PlaySession, PlaytimeManager};
use crate::config::{Config, PreferredViewType, SortBy};
use chrono::{Duration, Utc};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn create_demo_config() -> Config {
    let demo_dir = std::env::temp_dir().join("obelisk_demo");
    let instances_dir = demo_dir.join("instances");
    let _ = std::fs::create_dir_all(&instances_dir);

    Config {
        instances_path: Some(instances_dir.clone()),
        shared_data_path: Some(demo_dir.join("shared")),
        minecraft_data_path: demo_dir.join("minecraft"),
        java_path: Some(PathBuf::from("/usr/bin/java")),
        max_memory: 4096,
        min_memory: 1024,
        microsoft_client_id: Some("demo-client-id".to_string()),
        accounts: create_demo_accounts(),
        active_account_uuid: Some("demo-steve-uuid".to_string()),
        default_instance_icon: None,
        total_playtime: (142 + 78 + 24 + 31 + 6) * 3600,
        preferred_view_type: PreferredViewType::Grid,
        sort_by: SortBy::LastPlayed,
        is_demo: true,
    }
}

pub fn create_demo_accounts() -> Vec<Account> {
    vec![
        Account {
            username: "Steve".to_string(),
            uuid: "demo-steve-uuid".to_string(),
            access_token: "demo-token-steve".to_string(),
            refresh_token: "demo-refresh-steve".to_string(),
            token_expiry: u64::MAX,
            account_type: AccountType::Microsoft,
        },
        Account {
            username: "Alex".to_string(),
            uuid: "demo-alex-uuid".to_string(),
            access_token: "demo-token-alex".to_string(),
            refresh_token: "demo-refresh-alex".to_string(),
            token_expiry: u64::MAX,
            account_type: AccountType::Offline,
        },
    ]
}

pub fn create_demo_groups() -> InstanceGroups {
    let mut groups = HashMap::new();

    let mut perf_set = HashSet::new();
    perf_set.insert("fabulously-optimized".to_string());
    groups.insert("Performance".to_string(), GroupInfo { hidden: false, instances: perf_set });

    let mut pack_set = HashSet::new();
    pack_set.insert("create-astral".to_string());
    pack_set.insert("cobblemon".to_string());
    groups.insert("Modpacks".to_string(), GroupInfo { hidden: false, instances: pack_set });

    let mut surv_set = HashSet::new();
    surv_set.insert("vanilla-1-21-4".to_string());
    groups.insert("Survival".to_string(), GroupInfo { hidden: false, instances: surv_set });

    let mut test_set = HashSet::new();
    test_set.insert("redstone-lab".to_string());
    groups.insert("Testing".to_string(), GroupInfo { hidden: false, instances: test_set });

    InstanceGroups {
        format_version: 1,
        groups,
    }
}

pub fn create_demo_instances() -> Vec<Instance> {
    let base_dir = std::env::temp_dir().join("obelisk_demo").join("instances");

    let defs = vec![
        (
            "Fabulously Optimized",
            "fabulously-optimized",
            "1.21.4",
            Some("Fabric"),
            vec![
                ("net.minecraft", "1.21.4"),
                ("net.fabricmc.fabric-loader", "0.16.9"),
            ],
            vec![
                ("sodium", "Sodium", "0.5.11", "sodium-fabric-0.5.11.jar"),
                ("iris", "Iris Shaders", "1.7.5", "iris-fabric-1.7.5.jar"),
                ("lithium", "Lithium", "0.12.7", "lithium-fabric-0.12.7.jar"),
                ("entityculling", "Entity Culling", "1.6.5", "entityculling-1.6.5.jar"),
                ("ferritecore", "FerriteCore", "6.0.3", "ferritecore-6.0.3.jar"),
            ],
            142 * 3600,
            Some(Utc::now().timestamp() as u64 - 3600 * 2),
        ),
        (
            "Create: Astral",
            "create-astral",
            "1.20.1",
            Some("Forge"),
            vec![
                ("net.minecraft", "1.20.1"),
                ("net.minecraftforge", "47.3.0"),
            ],
            vec![
                ("create", "Create", "0.5.1.f", "create-1.20.1-0.5.1.f.jar"),
                ("jei", "Just Enough Items", "15.3.0", "jei-1.20.1-15.3.0.jar"),
                ("curios", "Curios API", "5.8.1", "curios-forge-5.8.1.jar"),
                ("applied-energistics-2", "AE2", "15.0.18", "ae2-forge-15.0.18.jar"),
            ],
            78 * 3600,
            Some(Utc::now().timestamp() as u64 - 3600 * 24),
        ),
        (
            "Vanilla 1.21.4",
            "vanilla-1-21-4",
            "1.21.4",
            None,
            vec![("net.minecraft", "1.21.4")],
            vec![],
            24 * 3600,
            Some(Utc::now().timestamp() as u64 - 3600 * 72),
        ),
        (
            "Cobblemon Adventure",
            "cobblemon",
            "1.20.1",
            Some("Fabric"),
            vec![
                ("net.minecraft", "1.20.1"),
                ("net.fabricmc.fabric-loader", "0.15.11"),
            ],
            vec![
                ("cobblemon", "Cobblemon", "1.5.2", "Cobblemon-fabric-1.5.2.jar"),
                ("architectury-api", "Architectury API", "9.2.14", "architectury-9.2.14.jar"),
            ],
            31 * 3600,
            Some(Utc::now().timestamp() as u64 - 3600 * 120),
        ),
        (
            "Redstone Lab",
            "redstone-lab",
            "1.21.1",
            Some("NeoForge"),
            vec![
                ("net.minecraft", "1.21.1"),
                ("net.neoforged", "21.1.65"),
            ],
            vec![
                ("carpet", "Carpet Mod", "1.4.147", "carpet-1.21.1.jar"),
                ("litematica", "Litematica", "0.18.0", "litematica-1.21.1.jar"),
            ],
            6 * 3600,
            Some(Utc::now().timestamp() as u64 - 3600 * 168),
        ),
    ];

    let mut instances = Vec::new();

    for (name, folder, mc_ver, loader_str, comps, mod_defs, playtime, last_launch) in defs {
        let inst_path = base_dir.join(folder);
        let mc_dir = inst_path.join(".minecraft");
        let _ = std::fs::create_dir_all(&mc_dir);
        // Ensure no leftover dummy icon file exists from a previous run
        let _ = std::fs::remove_file(inst_path.join("icon.png"));
        let _ = std::fs::remove_file(mc_dir.join("icon.png"));

        let components = comps
            .into_iter()
            .map(|(uid, ver)| InstanceComponent {
                uid: uid.to_string(),
                name: uid.to_string(),
                version: ver.to_string(),
            })
            .collect();

        let mods = mod_defs
            .into_iter()
            .map(|(id, mname, ver, fname)| ModInfo {
                id: id.to_string(),
                name: mname.to_string(),
                version: ver.to_string(),
                filename: fname.to_string(),
                description: Some(format!("{} mod for Minecraft {}", mname, mc_ver)),
                homepage: Some(format!("https://modrinth.com/mod/{}", id)),
                icon_path: None,
                enabled: true,
            })
            .collect();

        instances.push(Instance {
            name: name.to_string(),
            path: inst_path,
            icon_key: None,
            total_time_played: playtime,
            last_launched: last_launch,
            minecraft_version: Some(mc_ver.to_string()),
            mod_loader: loader_str.map(String::from),
            components,
            mods,
            resource_packs: Vec::new(),
            shader_packs: Vec::new(),
            worlds: vec![WorldInfo {
                name: "Main World".to_string(),
                folder_name: "world".to_string(),
                file_size: 45 * 1024 * 1024,
                seed: Some(123456789),
                mc_version: Some(mc_ver.to_string()),
                last_played: last_launch.map(|l| l as i64),
            }],
            screenshot_count: 4,
            java_path: Some(PathBuf::from("/usr/bin/java")),
            minecraft_dir: mc_dir,
            has_mismatch: false,
            feral_gamemode: false,
            discrete_gpu: false,
            zink_vulkan: false,
            use_wayland: true,
            id: folder.to_string(),
        });
    }

    instances
}

pub fn create_demo_playtime() -> PlaytimeManager {
    let now = Utc::now();
    let mut instances = HashMap::new();
    let mut sessions = Vec::new();

    let defs = vec![
        ("fabulously-optimized", "Fabulously Optimized", 142 * 3600, 48),
        ("create-astral", "Create: Astral", 78 * 3600, 26),
        ("vanilla-1-21-4", "Vanilla 1.21.4", 24 * 3600, 12),
        ("cobblemon", "Cobblemon Adventure", 31 * 3600, 15),
        ("redstone-lab", "Redstone Lab", 6 * 3600, 4),
    ];

    for (id, name, total_secs, count) in &defs {
        instances.insert(
            id.to_string(),
            InstancePlaytimeData {
                name: name.to_string(),
                playtime: *total_secs,
                session_count: *count,
                last_played: Some(now - Duration::hours(2)),
                first_played: Some(now - Duration::days(45)),
            },
        );
    }

    // Generate daily sessions over the past 7 days
    let daily_durations = [
        ("fabulously-optimized", 0, 7200),  // Today: 2 hrs
        ("create-astral", 0, 3600),        // Today: 1 hr
        ("fabulously-optimized", 1, 10800), // Yesterday: 3 hrs
        ("cobblemon", 1, 5400),            // Yesterday: 1.5 hrs
        ("create-astral", 2, 7200),        // 2 days ago: 2 hrs
        ("vanilla-1-21-4", 2, 3600),       // 2 days ago: 1 hr
        ("fabulously-optimized", 3, 14400), // 3 days ago: 4 hrs
        ("redstone-lab", 4, 7200),         // 4 days ago: 2 hrs
        ("create-astral", 5, 9000),        // 5 days ago: 2.5 hrs
        ("cobblemon", 6, 10800),           // 6 days ago: 3 hrs
    ];

    for (id, days_ago, dur) in daily_durations {
        let start = now - Duration::days(days_ago) - Duration::seconds(dur as i64);
        let end = now - Duration::days(days_ago);
        sessions.push(PlaySession {
            instance_id: id.to_string(),
            start_time: start,
            end_time: end,
            duration_seconds: dur,
        });
    }

    PlaytimeManager {
        version: "2.0".to_string(),
        instances,
        sessions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_demo_config() {
        let config = create_demo_config();
        assert!(config.is_demo);
        assert_eq!(config.accounts.len(), 2);
        assert_eq!(config.active_account_uuid, Some("demo-steve-uuid".to_string()));
    }

    #[test]
    fn test_create_demo_groups() {
        let groups = create_demo_groups();
        assert_eq!(groups.groups.len(), 4);
        assert!(groups.groups.contains_key("Modpacks"));
    }

    #[test]
    fn test_create_demo_instances() {
        let instances = create_demo_instances();
        assert_eq!(instances.len(), 5);
        assert!(instances.iter().any(|i| i.name == "Fabulously Optimized"));
        assert!(instances.iter().any(|i| i.name == "Vanilla 1.21.4"));
        for inst in &instances {
            assert!(!inst.path.join("icon.png").exists());
            assert!(!inst.minecraft_dir.join("icon.png").exists());
        }
    }

    #[test]
    fn test_create_demo_playtime() {
        let playtime = create_demo_playtime();
        assert_eq!(playtime.instances.len(), 5);
        assert!(!playtime.sessions.is_empty());
    }
}

