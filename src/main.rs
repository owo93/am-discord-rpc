mod media;
mod session;

use anyhow::Result;
use rpcdiscord::{
    DiscordIpc, DiscordIpcClient,
    activity::{self, Assets},
};
use std::time::Duration;

use crate::media::MediaInfo;
use crate::session::AppleMusicSession;

const POLL_INTERVAL: Duration = Duration::from_millis(250);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client_id = std::env::var("CLIENT_ID").expect("CLIENT_ID must be set");

    let mut discord = DiscordIpcClient::new(&client_id)?;
    discord.connect().expect("Failed to connect to Discord");

    _ = discord.clear_activity();

    let mut cache = None;
    match AppleMusicSession::get().await? {
        None => println!("Apple music is not running"),
        Some(session) => {
            println!("Found Apple music, starting presence loop...");
            loop {
                let (info, new_cache) = MediaInfo::from(&session, cache.as_ref()).await?;
                if new_cache != cache {
                    cache = new_cache;
                }

                let assets = match &info.song.artwork {
                    Some(artwork) => Assets::new()
                        .small_image(&artwork.0)
                        .small_text(&info.song.title),
                    None => Assets::new(),
                };

                let state_msg = format!("{} {}", &info.song.album, &info.song.artist);
                let am_activity = activity::Activity::new()
                    .activity_type(activity::ActivityType::Listening)
                    .state(&state_msg)
                    .details(&info.song.title)
                    .assets(assets)
                    .timestamps(activity::Timestamps::new().start(info.start).end(info.end));

                discord
                    .set_activity(am_activity)
                    .expect("Failed to set activity");

                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }

    Ok(())
}
