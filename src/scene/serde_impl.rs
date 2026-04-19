use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::Scene;
use crate::{
    soundscape::Soundscape,
    track::{Track, TrackData},
};

#[derive(Serialize, Deserialize)]
pub struct SceneData {
    #[serde(skip)]
    name: String,
    tracks: OrderMap<Ulid, TrackData>,
    soundscape: Soundscape,
}

impl SceneData {
    #[must_use]
    pub fn with_name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}

impl TryFrom<&Scene> for SceneData {
    type Error = anyhow::Error;

    fn try_from(scene: &Scene) -> Result<Self, Self::Error> {
        let tracks = scene
            .tracks
            .iter()
            .map(|(id, track)| TrackData::try_from(track).map(|t| (*id, t)))
            .try_collect()?;

        Ok(Self {
            name: scene.name.clone(),
            tracks,
            soundscape: scene.soundscape.clone(),
        })
    }
}

impl TryFrom<SceneData> for Scene {
    type Error = anyhow::Error;

    fn try_from(scene_data: SceneData) -> Result<Self, Self::Error> {
        let tracks = scene_data
            .tracks
            .into_iter()
            .map(|(id, track_data)| Track::try_from(track_data).map(|t| (id, t)))
            .try_collect()?;

        Ok(Self {
            name: scene_data.name.clone(),
            tracks,
            soundscape: scene_data.soundscape,
        })
    }
}
