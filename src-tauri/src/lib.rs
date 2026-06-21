#![allow(non_snake_case)]

use tauri::{Manager, Emitter};
use std::time::Duration;
use std::fs;
use std::sync::OnceLock;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Structs for LCU communication
#[derive(Serialize, Clone)]
struct LcuStatus {
    connected: bool,
    port: String,
    password: String,
    pid: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct LcuSummoner {
    displayName: Option<String>,
    gameName: Option<String>,
    summonerLevel: u32,
    profileIconId: u32,
    puuid: String,
    accountId: u64,
}

#[derive(Serialize, Clone)]
struct SummonerEvent {
    name: String,
    level: u32,
    profileIconId: u32,
    puuid: String,
}

#[derive(Serialize, Clone)]
struct MatchInfo {
    gameId: u64,
    win: bool,
    kills: u32,
    deaths: u32,
    assists: u32,
    championName: String,
    championImage: Option<String>,
}

#[derive(Deserialize)]
struct LcuMatchlist {
    games: Option<LcuGamesList>,
}

#[derive(Deserialize)]
struct LcuGamesList {
    games: Option<Vec<LcuGame>>,
}

#[derive(Deserialize)]
struct LcuGame {
    gameId: u64,
    participantIdentities: Option<Vec<LcuParticipantIdentity>>,
    participants: Option<Vec<LcuParticipant>>,
}

#[derive(Deserialize)]
struct LcuParticipantIdentity {
    participantId: u32,
    player: Option<LcuPlayerInfo>,
}

#[derive(Deserialize)]
struct LcuPlayerInfo {
    puuid: String,
}

#[derive(Deserialize)]
struct LcuParticipant {
    participantId: u32,
    championId: u32,
    stats: Option<LcuStats>,
}

#[derive(Deserialize)]
struct LcuStats {
    win: bool,
    kills: u32,
    deaths: u32,
    assists: u32,
}

#[derive(Deserialize)]
struct LcuChampSelectSession {
    myTeam: Option<Vec<LcuChampSelectPlayer>>,
}

#[derive(Deserialize)]
struct LcuChampSelectPlayer {
    cellId: i64,
    championId: u32,
}

#[derive(Serialize, Clone)]
struct ChampSelectInfo {
    active: bool,
    myTeam: Vec<ChampSelectPlayerEvent>,
}

#[derive(Serialize, Clone)]
struct ChampSelectPlayerEvent {
    cellId: i64,
    championName: String,
    championImage: Option<String>,
}

// Global static map for champion name translations
static CHAMPION_MAP: OnceLock<HashMap<u32, (String, String)>> = OnceLock::new();
const LOCKFILE_PATH: &str = "C:\\Riot Games\\League of Legends\\lockfile";

async fn load_champion_map() {
    let client = reqwest::Client::new();
    if let Ok(res) = client.get("https://ddragon.leagueoflegends.com/cdn/14.3.1/data/es_ES/champion.json").send().await {
        if let Ok(json) = res.json::<serde_json::Value>().await {
            let mut map = HashMap::new();
            if let Some(data) = json.get("data") {
                if let Some(obj) = data.as_object() {
                    for (_key, val) in obj {
                        let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let key_str = val.get("key").and_then(|v| v.as_str()).unwrap_or("0");
                        let id_str = val.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if let Ok(id_num) = key_str.parse::<u32>() {
                            map.insert(id_num, (name, id_str));
                        }
                    }
                }
            }
            let _ = CHAMPION_MAP.set(map);
            println!("Champion map cached.");
        }
    }
}

fn read_lockfile() -> Option<(String, String, String)> {
    if let Ok(content) = fs::read_to_string(LOCKFILE_PATH) {
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() >= 5 {
            let pid = parts[1].to_string();
            let port = parts[2].to_string();
            let password = parts[3].to_string();
            return Some((pid, port, password));
        }
    }
    None
}

async fn fetch_summoner(client: &reqwest::Client, port: &str, password: &str) -> Option<LcuSummoner> {
    let url = format!("https://127.0.0.1:{}/lol-summoner/v1/current-summoner", port);
    client.get(&url)
        .basic_auth("riot", Some(password))
        .send()
        .await
        .ok()?
        .json::<LcuSummoner>()
        .await
        .ok()
}

async fn fetch_match_history(client: &reqwest::Client, port: &str, password: &str, puuid: &str) -> Option<Vec<MatchInfo>> {
    let url = format!("https://127.0.0.1:{}/lol-match-history/v1/products/lol/{}/matches", port, puuid);
    let res = client.get(&url)
        .basic_auth("riot", Some(password))
        .send()
        .await
        .ok()?;
    
    let matchlist = res.json::<LcuMatchlist>().await.ok()?;
    let raw_games = matchlist.games?.games?;
    
    let mut mapped_games = Vec::new();
    let champ_map = CHAMPION_MAP.get()?;

    for game in raw_games.into_iter().take(5) {
        let p_identities = game.participantIdentities?;
        let identity = p_identities.iter().find(|id| id.player.as_ref().map_or(false, |p| p.puuid == puuid));
        if let Some(id) = identity {
            let p_id = id.participantId;
            let participants = game.participants?;
            if let Some(participant) = participants.iter().find(|p| p.participantId == p_id) {
                let stats = participant.stats.as_ref()?;
                let champion_id = participant.championId;
                
                let (champ_name, champ_id_name) = champ_map.get(&champion_id)
                    .map(|(name, id_name)| (name.clone(), Some(format!("https://ddragon.leagueoflegends.com/cdn/14.3.1/img/champion/{}.png", id_name))))
                    .unwrap_or_else(|| ("Desconocido".to_string(), None));

                mapped_games.push(MatchInfo {
                    gameId: game.gameId,
                    win: stats.win,
                    kills: stats.kills,
                    deaths: stats.deaths,
                    assists: stats.assists,
                    championName: champ_name,
                    championImage: champ_id_name,
                });
            }
        }
    }
    Some(mapped_games)
}

async fn fetch_champ_select(client: &reqwest::Client, port: &str, password: &str) -> Option<ChampSelectInfo> {
    let url = format!("https://127.0.0.1:{}/lol-champ-select/v1/session", port);
    let res = client.get(&url)
        .basic_auth("riot", Some(password))
        .send()
        .await
        .ok()?;
    
    if res.status().is_success() {
        let session = res.json::<LcuChampSelectSession>().await.ok()?;
        let raw_team = session.myTeam?;
        let champ_map = CHAMPION_MAP.get()?;
        
        let my_team = raw_team.into_iter().map(|player| {
            let (champ_name, champ_id_name) = champ_map.get(&player.championId)
                .map(|(name, id_name)| (name.clone(), Some(format!("https://ddragon.leagueoflegends.com/cdn/14.3.1/img/champion/{}.png", id_name))))
                .unwrap_or_else(|| (
                    if player.championId > 0 { "Seleccionando...".to_string() } else { "Buscando...".to_string() },
                    None
                ));
            
            ChampSelectPlayerEvent {
                cellId: player.cellId,
                championName: champ_name,
                championImage: champ_id_name,
            }
        }).collect();

        Some(ChampSelectInfo {
            active: true,
            myTeam: my_team,
        })
    } else {
        None
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            // Load champion mapping once on startup
            tauri::async_runtime::spawn(async move {
                load_champion_map().await;
            });

            // Monitor League of Legends Client and LCU API
            let window_clone = window.clone();
            tauri::async_runtime::spawn(async move {
                let mut is_connected = false;
                let mut current_pid = String::new();
                let client = reqwest::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .unwrap();

                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    if let Some((pid, port, password)) = read_lockfile() {
                        let _ = window_clone.emit("lcu-status", LcuStatus {
                            connected: true,
                            port: port.clone(),
                            password: password.clone(),
                            pid: pid.clone(),
                        });

                        if !is_connected || current_pid != pid {
                            is_connected = true;
                            current_pid = pid.clone();

                            // Fetch summoner and match history
                            if let Some(summoner) = fetch_summoner(&client, &port, &password).await {
                                let _ = window_clone.emit("summoner-info", SummonerEvent {
                                    name: summoner.displayName.unwrap_or(summoner.gameName.unwrap_or_default()),
                                    level: summoner.summonerLevel,
                                    profileIconId: summoner.profileIconId,
                                    puuid: summoner.puuid.clone(),
                                });

                                // Fetch match history
                                if let Some(matches) = fetch_match_history(&client, &port, &password, &summoner.puuid).await {
                                    let _ = window_clone.emit("match-history", matches);
                                }
                            }
                            
                            // Spawn champ select polling loop
                            let window_inner = window_clone.clone();
                            let client_inner = client.clone();
                            let port_inner = port.clone();
                            let password_inner = password.clone();
                            let current_pid_inner = current_pid.clone();
                            let app_handle = window_inner.app_handle().clone();

                            tauri::async_runtime::spawn(async move {
                                loop {
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    
                                    // Check if we are still connected to the same PID
                                    if let Some((check_pid, _, _)) = read_lockfile() {
                                        if check_pid != current_pid_inner {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }

                                    if let Some(session) = fetch_champ_select(&client_inner, &port_inner, &password_inner).await {
                                        let _ = window_inner.emit("champ-select-update", session.clone());

                                        // Dynamically create overlay window if it does not exist
                                        if app_handle.get_webview_window("overlay").is_none() {
                                            let overlay = tauri::WebviewWindowBuilder::new(
                                                &app_handle,
                                                "overlay",
                                                tauri::WebviewUrl::App("overlay.html".into())
                                            )
                                            .title("chupachotas.companion - Overlay")
                                            .transparent(true)
                                            .decorations(false)
                                            .always_on_top(true)
                                            .maximized(true)
                                            .build();

                                            if let Ok(w) = overlay {
                                                let w_clone = w.clone();
                                                tauri::async_runtime::spawn(async move {
                                                    let mut currently_ignored = false;
                                                    loop {
                                                        tokio::time::sleep(Duration::from_millis(40)).await;
                                                        
                                                        #[cfg(target_os = "windows")]
                                                        {
                                                            use winapi::shared::windef::POINT;
                                                            use winapi::um::winuser::GetCursorPos;
                                                            
                                                            let mut point = POINT { x: 0, y: 0 };
                                                            if unsafe { GetCursorPos(&mut point) } != 0 {
                                                                if let Ok(win_pos) = w_clone.outer_position() {
                                                                    if let Ok(scale_factor) = w_clone.scale_factor() {
                                                                        // Bounding box of the HUD element: left: 20px, top: 20px, width: 320px, height: 250px
                                                                        let box_left = win_pos.x + (20.0 * scale_factor) as i32;
                                                                        let box_top = win_pos.y + (20.0 * scale_factor) as i32;
                                                                        let box_right = box_left + (320.0 * scale_factor) as i32;
                                                                        let box_bottom = box_top + (250.0 * scale_factor) as i32;
                                                                        
                                                                        let inside_box = point.x >= box_left && point.x <= box_right &&
                                                                                         point.y >= box_top && point.y <= box_bottom;
                                                                        
                                                                        if inside_box && currently_ignored {
                                                                            currently_ignored = false;
                                                                            if w_clone.set_ignore_cursor_events(false).is_err() {
                                                                                break;
                                                                            }
                                                                        } else if !inside_box && !currently_ignored {
                                                                            currently_ignored = true;
                                                                            if w_clone.set_ignore_cursor_events(true).is_err() {
                                                                                break;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    } else {
                                        let _ = window_inner.emit("champ-select-update", ChampSelectInfo {
                                            active: false,
                                            myTeam: vec![]
                                        });

                                        // Close overlay window if it exists
                                        if let Some(overlay_win) = app_handle.get_webview_window("overlay") {
                                            let _ = overlay_win.close();
                                        }
                                    }
                                }
                            });
                        }
                    } else {
                        if is_connected {
                            is_connected = false;
                            current_pid.clear();
                            let _ = window_clone.emit("lcu-status", LcuStatus {
                                connected: false,
                                port: String::new(),
                                password: String::new(),
                                pid: String::new(),
                            });
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
