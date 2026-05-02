use std::{
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Result;
use iced::{
    Element,
    Length::Fill,
    Subscription, Task,
    widget::{button, column, container, row, text},
};
use log::{debug, info};
use rfd::FileDialog;
use ulid::Ulid;

use crate::{
    PROJECT_DIRS,
    audio_cache::AudioCache,
    scene::{self, Scene, SceneData},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Scene(scene::Message),
    SwitchScene(usize),
    Save,
    Load,
}

pub struct App {
    scenes: Vec<Scene>,
    active_index: usize,
    audio_cache: AudioCache,
}

impl Default for App {
    fn default() -> Self {
        Self {
            scenes: vec![Scene::default()],
            active_index: 0,
            audio_cache: AudioCache::new(),
        }
    }
}

impl App {
    const FILE_EXTENSION: &'static str = "rst";

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Scene(msg) => {
                if let Some(action) = self.active_scene_mut().update(msg) {
                    match action {
                        scene::Action::Run(task) => task.map(Message::Scene),
                        scene::Action::AddTrack => {
                            info!("Received request to load new track for current scene.");

                            if let Some(path) = FileDialog::new()
                                .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                                .pick_file()
                            {
                                let msg = self.add_track(&path).expect("should be able to add track");
                                Task::done(msg)
                            } else {
                                info!("Track load cancelled for current scene.");
                                Task::none()
                            }
                        }
                    }
                } else {
                    Task::none()
                }
            }
            Message::SwitchScene(index) => {
                info!("Switching to scene #{index}.");
                self.active_index = index;
                Task::none()
            }
            Message::Save => {
                info!("Received request to save current scene.");
                if let Some(path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .set_file_name(format!(
                        "{}.{}",
                        self.active_scene().name(),
                        Self::FILE_EXTENSION
                    ))
                    .save_file()
                {
                    self.save_active_scene(path).expect("should be able to save current scene");
                } else {
                    info!("Save cancelled.");
                }
                Task::none()
            }
            Message::Load => {
                info!("Received request to load scene savefile.");
                if let Some(path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .pick_file()
                {
                    let scene = self.load_scene(&path).expect("should be able to load scene");
                    self.scenes.push(scene);

                    Task::done(Message::Scene(scene::Message::Loaded))
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let tabs = row(self.scenes.iter().enumerate().map(|(i, scene)| {
            let style = if self.active_index == i {
                button::primary
            } else {
                button::background
            };
            button(scene.name())
                .on_press(Message::SwitchScene(i))
                .style(style)
                .width(200)
                .into()
        }));

        let scene_info = container(row![
            text(self.active_scene().name()),
            button("Save")
                .on_press(Message::Save)
                .style(button::background),
            button("Load")
                .on_press(Message::Load)
                .style(button::background),
        ])
        .style(container::primary)
        .padding(4)
        .width(Fill);

        let topbar = column![tabs, scene_info];

        column![topbar, self.active_scene().view().map(Message::Scene),].into()
    }

    fn save_active_scene(&mut self, mut path: PathBuf) -> Result<()> {
        info!(
            "Saving current scene to {}.",
            path.to_str().unwrap_or_default()
        );
        if path.extension().and_then(|s| s.to_str()) != Some(Self::FILE_EXTENSION) {
            path.add_extension(Self::FILE_EXTENSION);
        }

        let swapfile_path = PROJECT_DIRS
            .cache_dir()
            .with_file_name(path.file_name().unwrap());

        {
            let data = SceneData::new(self.active_scene(), &self.audio_cache)?;
            let mut swapfile = File::create_buffered(&swapfile_path)?;
            rmp_serde::encode::write(&mut swapfile, &data)?;
            debug!("Wrote save to swapfile.");
        }

        std::fs::rename(&swapfile_path, &path)?;
        info!(
            "Current scene saved to {}.",
            path.to_str().unwrap_or_default()
        );

        Ok(())
    }

    fn load_scene(&mut self, path: &Path) -> Result<Scene> {
        info!("Loading scene at {}.", path.to_str().unwrap_or_default());
        let scene_name = path.file_stem().unwrap().to_string_lossy();

        let file = File::open(path)?;
        let data: SceneData = rmp_serde::decode::from_read(file)?;
        debug!("Loaded savefile data.");

        let scene: Scene = Scene::from_data(data.with_name(&scene_name), &mut self.audio_cache)?;
        info!("Loaded scene {}.", scene.name());

        Ok(scene)
    }

    fn add_track(&mut self, path: &Path) -> Result<Message> {
        info!(
            "Loading track at {} for current scene.",
            path.to_str().unwrap_or_default(),
        );

        let name = path.file_stem().map_or("Unknown filename".into(), |s| {
            s.to_string_lossy().to_string()
        });
        debug!("Track name: {name}");

        let mut file = File::open_buffered(path)?;

        let data = self.audio_cache.get_or_register(&mut file)?;
        debug!("Registered track.");

        let id = Ulid::new();
        debug!("Track ID: {id}");

        info!("Loaded track {id} ({name}).");

        Ok(Message::Scene(scene::Message::TrackAdded {
            id,
            name,
            data,
        }))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.active_scene().subscription().map(Message::Scene)
    }

    fn active_scene(&self) -> &Scene {
        &self.scenes[self.active_index]
    }

    fn active_scene_mut(&mut self) -> &mut Scene {
        &mut self.scenes[self.active_index]
    }
}
