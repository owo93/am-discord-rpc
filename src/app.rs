use anyhow::Result;
use rpcdiscord::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, ActivityType, Assets, Button, Timestamps},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::media::MediaInfo;
use crate::session::AppleMusicSession;

const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct App {
    config: AppConfig,
}

#[derive(Debug)]
pub struct AppConfig {
    pub client_id: String,
    pub repo: &'static str,
}

impl App {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            config: AppConfig {
                client_id: client_id.into(),
                repo: env!("CARGO_PKG_REPOSITORY"),
            },
        }
    }

    pub async fn run(&self, running: Arc<AtomicBool>) -> Result<()> {
        let mut cache = None;
        let mut discord: Option<DiscordIpcClient> = None;

        while running.load(Ordering::SeqCst) {
            let mut discord_client = match DiscordIpcClient::new(&self.config.client_id) {
                Ok(client) => client,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            while running.load(Ordering::SeqCst) {
                match discord_client.connect() {
                    Ok(_) => {
                        debug!("Connected to discord");
                        break;
                    }
                    Err(_) => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            let _ = discord_client.clear_activity();
            discord = Some(discord_client);

            match AppleMusicSession::get().await? {
                None => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                Some(session) => {
                    debug!("Found Apple Music, starting presence loop...");
                    while running.load(Ordering::SeqCst) {
                        let (info, new_cache) =
                            match MediaInfo::from(&session, cache.as_ref()).await {
                                Ok(res) => res,
                                Err(e) => {
                                    warn!("Failed to get media info: {e}, waiting for Apple Music");
                                    break;
                                }
                            };
                        if new_cache != cache {
                            cache = new_cache;
                        }

                        let assets = match &info.song.artwork {
                            Some(artwork) => Assets::new().large_image(&artwork.0).small_image("https://cdn.brandfetch.io/id_yBTuraI/w/100/h/100/theme/dark/icon.jpeg?c=1dxbfHSJFAPEGdCLU4o5B"),
                            None => Assets::new(),
                        };

                        let mut buttons =
                            vec![Button::new(env!("CARGO_PKG_NAME"), self.config.repo)];
                        if let Some(url) = &info.track_url {
                            buttons.insert(0, Button::new("Open in Apple Music", url.as_str()));
                        }

                        let am_activity = Activity::new()
                            .activity_type(ActivityType::Listening)
                            .details(&info.song.title)
                            .state(&info.song.artist)
                            .assets(assets)
                            .timestamps(Timestamps::new().start(info.start).end(info.end))
                            .buttons(buttons);

                        match discord.as_mut().unwrap().set_activity(am_activity) {
                            Ok(_) => {}
                            Err(e) => {
                                warn!("Lost Discord connection: {e}, reconnecting...");
                                break;
                            }
                        }

                        tokio::time::sleep(POLL_INTERVAL).await;
                    }
                }
            }
        }

        if let Some(ref mut client) = discord {
            let _ = client.clear_activity();
            let _ = client.close();
        }
        info!("Goodbye!");
        Ok(())
    }
}
