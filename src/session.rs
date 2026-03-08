use anyhow::{Context, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};

const APPLE_MUSIC_APP_ID: &str = "ppleInc.AppleMusicWin_";

#[derive(Debug)]
pub struct AppleMusicSession {
    pub session: windows::Media::Control::GlobalSystemMediaTransportControlsSession,
}

impl AppleMusicSession {
    pub async fn get() -> Result<Option<Self>> {
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?
            .await
            .context("Failed to get session manager")?;

        let sessions = manager.GetSessions()?;

        Ok(sessions
            .into_iter()
            .find(|session| {
                session
                    .SourceAppUserModelId()
                    .map(|id| id.to_string().contains(APPLE_MUSIC_APP_ID))
                    .unwrap_or(false)
            })
            .map(|session| Self { session }))
    }
}

impl std::ops::Deref for AppleMusicSession {
    type Target = GlobalSystemMediaTransportControlsSession;
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}
