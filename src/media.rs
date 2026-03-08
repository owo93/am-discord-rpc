use anyhow::{Context, Result};
use std::time::Duration;
use windows::Media::Control::GlobalSystemMediaTransportControlsSession;

#[derive(Debug)]
pub struct ITunesResult {
    pub artwork_url: Option<String>,
    pub track_url: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Artwork(pub String);

#[derive(Debug, PartialEq)]
pub struct Cache {
    pub title: String,
    pub artwork: Artwork,
    pub track_url: Option<String>,
}

#[derive(Debug)]
pub struct Song {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork: Option<Artwork>,
}

#[derive(Debug, Default)]
pub struct MediaInfo {
    pub song: Song,
    pub start: i64,
    pub end: i64,
    pub track_url: Option<String>,
}

impl MediaInfo {
    pub fn new(song: Song, start: i64, end: i64, track_url: Option<String>) -> Self {
        Self {
            song,
            start,
            end,
            track_url,
        }
    }

    pub async fn from(
        session: &GlobalSystemMediaTransportControlsSession,
        cache: Option<&Cache>,
    ) -> Result<(Self, Option<Cache>)> {
        let (song, track_url) = Song::from(session, cache).await?;
        let timeline = session.GetTimelineProperties()?;
        let now = chrono::Utc::now().timestamp();
        let elapsed = Duration::from_nanos(timeline.Position()?.Duration as u64 * 100);
        let total = Duration::from_nanos(timeline.EndTime()?.Duration as u64 * 100);
        let start = now - elapsed.as_secs() as i64;
        let end = now + total.as_secs() as i64;

        let new_cache = song
            .artwork
            .as_ref()
            .map(|a| Cache::new(song.title.clone(), Artwork(a.0.clone()), track_url.clone()));
        Ok((Self::new(song, start, end, track_url), new_cache))
    }
}

impl Song {
    pub fn new(title: String, artist: String, album: String, artwork: Option<Artwork>) -> Self {
        Self {
            title,
            artist,
            album,
            artwork,
        }
    }

    async fn from(
        session: &GlobalSystemMediaTransportControlsSession,
        cache: Option<&Cache>,
    ) -> Result<(Self, Option<String>)> {
        let media = session.TryGetMediaPropertiesAsync()?.await?;
        let title = media.Title().unwrap_or_default().to_string();
        let artist = media.Artist().unwrap_or_default().to_string();
        let album = media.AlbumTitle().unwrap_or_default().to_string();

        let (artwork, track_url) = match cache {
            Some(c) if c.matches(&title) => {
                (Some(Artwork(c.artwork.0.clone())), c.track_url.clone())
            }
            _ => match Artwork::fetch(&title, &artist).await {
                Ok((a, url)) => (Some(a), url),
                Err(_) => (None, None),
            },
        };

        Ok((Self::new(title, artist, album, artwork), track_url))
    }
}

impl Default for Song {
    fn default() -> Self {
        Self {
            title: "Unknown song".to_string(),
            artist: "Unknown artist".to_string(),
            album: "Unknown album".to_string(),
            artwork: None,
        }
    }
}

impl Artwork {
    fn new(url: &str) -> Self {
        Self(url.replace("100x100", "128x128"))
    }

    pub async fn fetch(title: &str, artist: &str) -> Result<(Self, Option<String>)> {
        let res = ITunesResult::fetch(title, artist).await?;
        let artwork = res.artwork_url.map(Artwork).context("Artwork not found")?;
        Ok((artwork, res.track_url))
    }
}

impl Cache {
    pub fn new(title: String, artwork: Artwork, track_url: Option<String>) -> Self {
        Self {
            title,
            artwork,
            track_url,
        }
    }

    pub fn matches(&self, title: &str) -> bool {
        self.title == title
    }
}

impl ITunesResult {
    pub async fn fetch(title: &str, artist: &str) -> Result<Self> {
        let query = format!("{} {}", title, artist);
        let url = format!(
            "https://itunes.apple.com/search?term={}&media=music&limit=1",
            urlencoding::encode(&query)
        );

        let res = reqwest::get(&url)
            .await
            .context("Failed to fetch iTunes")?
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse iTunes response")?;

        Ok(Self {
            artwork_url: res["results"][0]["artworkUrl100"]
                .as_str()
                .map(|u| u.replace("100x100bb", "256x256bb")),
            track_url: res["results"][0]["trackViewUrl"].as_str().map(String::from),
        })
    }
}
