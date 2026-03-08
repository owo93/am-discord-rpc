mod media;
mod session;

use anyhow::Result;
use rpcdiscord::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, ActivityType, Assets, Button, Timestamps},
};
use std::time::Duration;

use crate::media::MediaInfo;
use crate::session::AppleMusicSession;

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const REPO: &str = env!("CARGO_PKG_REPOSITORY");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID must be set");

    let mut cache = None;

    loop {
        let mut discord = DiscordIpcClient::new(&client_id)?;
        loop {
            match discord.connect() {
                Ok(_) => {
                    println!("Connected to Discord!");
                    break;
                }
                Err(_) => {
                    eprintln!("Waiting for Discord...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        _ = discord.clear_activity();

        match AppleMusicSession::get().await? {
            None => {
                eprintln!("Apple Music is not running.");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Some(session) => {
                println!("Found Apple music, starting presence loop...");
                loop {
                    let (info, new_cache) = MediaInfo::from(&session, cache.as_ref()).await?;
                    if new_cache != cache {
                        cache = new_cache;
                    }

                    let assets = match &info.song.artwork {
                        Some(artwork) => Assets::new()
                            .large_image(&artwork.0)
                            .large_text(&info.song.title),
                        None => Assets::new(),
                    };

                    let state_msg = format!("{} {}", &info.song.album, &info.song.artist);
                    let am_activity = Activity::new()
                        .activity_type(ActivityType::Listening)
                        .details(&info.song.title)
                        .state(&state_msg)
                        .assets(assets)
                        .timestamps(Timestamps::new().start(info.start).end(info.end))
                        .buttons(vec![
                            Button::new(
                                "Open in Apple Music",
                                info.track_url.as_deref().unwrap_or(""),
                            ),
                            Button::new(env!("CARGO_PKG_NAME"), REPO),
                        ]);
                    match discord.set_activity(am_activity) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Lost Discord connection: {e}, reconnecting...");
                            break;
                        }
                    }

                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            }
        }
    }
}
