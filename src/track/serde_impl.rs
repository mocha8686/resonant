use std::sync::Arc;

use anyhow::Result;
use kira::{AudioManager, AudioManagerSettings};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{Handle, Track, looping::Loop, play_pause::PlayPause, progress::Progress};
use crate::{
    Vector2,
    audio_cache::{AudioData, FileHash},
};

#[derive(Serialize, Deserialize)]
pub struct TrackData {
    id: Ulid,
    name: String,
    hash: FileHash,
    position: Vector2,
    radius: f32,
    duration: f32,
}

impl TrackData {
    pub fn new(track: &Track) -> Self {
        Self {
            id: track.id,
            name: track.name.clone(),
            hash: track.hash(),
            position: track.position,
            radius: track.radius,
            duration: track.progress.duration(),
        }
    }

    #[must_use]
    pub fn hash(&self) -> FileHash {
        self.hash
    }
}

impl Track {
    pub fn from_data(track_data: TrackData, audio_data: Arc<AudioData>) -> Result<Self> {
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        let data = audio_data.load()?;
        let duration = data.unsliced_duration().as_secs_f32();

        Ok(Track {
            id: track_data.id,
            name: track_data.name,
            audio_data,
            position: track_data.position,
            radius: track_data.radius,
            selected: false,
            manager,
            handle: Handle::Uninitialized(Some(data)),
            play_pause: PlayPause::new(),
            progress: Progress::new(duration),
            looping: Loop::new(),
        })
    }
}
