use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Result;
use iced::{Element, Subscription, Task, futures::stream, widget::text, window as iced_window};
use rfd::FileDialog;
use ulid::Ulid;

use crate::{
    PROJECT_DIRS,
    audio_cache::AudioCache,
    scene::{self, Scene, SceneData},
    window::{self, Window},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Initialized,
    Window(iced_window::Id, window::Message),
    SaveRequested(iced_window::Id),
    LoadRequested,
    CloseRequested(iced_window::Id),
}

pub struct App {
    windows: HashMap<iced_window::Id, Window>,
    audio_cache: AudioCache,
}

impl Default for App {
    fn default() -> Self {
        Self {
            windows: HashMap::new(),
            audio_cache: AudioCache::new(),
        }
    }
}

impl App {
    const FILE_EXTENSION: &'static str = "rst";

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Initialized => {
                let default_scene = Scene::default();
                let (window, open) = Window::open(default_scene, iced_window::Settings::default());
                self.windows.insert(window.id(), window);

                open.discard()
            }
            Message::Window(window_id, msg) => {
                if let Some(window) = self.windows.get_mut(&window_id)
                    && let Some(action) = window.update(msg)
                {
                    match action {
                        window::Action::Run(task) => {
                            task.map(move |msg| Message::Window(window_id, msg))
                        }
                        window::Action::AddTrack => {
                            log::info!("Received request to load new track for current scene.");

                            if let Some(path) = FileDialog::new()
                                .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                                .pick_file()
                            {
                                let msg = self
                                    .add_track(window_id, &path)
                                    .expect("should be able to add track");
                                Task::done(msg)
                            } else {
                                log::info!("Track load cancelled for current scene.");
                                Task::none()
                            }
                        }
                        window::Action::Save => Task::done(Message::SaveRequested(window_id)),
                        window::Action::Load => Task::done(Message::LoadRequested),
                        window::Action::Close => {
                            self.windows.remove(&window_id);

                            let close = iced_window::close(window_id);
                            let exit = if self.windows.is_empty() {
                                iced::exit()
                            } else {
                                Task::none()
                            };

                            close.chain(exit)
                        }
                    }
                } else {
                    Task::none()
                }
            }
            Message::SaveRequested(window_id) => {
                log::info!("Received request to save current scene.");

                if let Some(window) = self.windows.get(&window_id) {
                    if let Some(path) = FileDialog::new()
                        .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                        .set_file_name(format!(
                            "{}.{}",
                            window.scene().name(),
                            Self::FILE_EXTENSION
                        ))
                        .save_file()
                    {
                        self.save_scene(window.scene(), path)
                            .expect("should be able to save current scene");
                    } else {
                        log::info!("Save cancelled.");
                    }
                } else {
                    log::warn!("Scene not found.");
                }

                Task::none()
            }
            Message::LoadRequested => {
                log::info!("Received request to load scene savefile.");
                if let Some(path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .pick_file()
                {
                    let scene = self
                        .load_scene(&path)
                        .expect("should be able to load scene");

                    let (window, open) = Window::open(scene, iced_window::Settings::default());
                    let window_id = window.id();
                    self.windows.insert(window_id, window);

                    open.discard().chain(Task::done(Message::Window(
                        window_id,
                        window::Message::Scene(scene::Message::Loaded),
                    )))
                } else {
                    Task::none()
                }
            }
            Message::CloseRequested(id) => {
                Task::done(Message::Window(id, window::Message::CloseRequested))
            }
        }
    }

    #[must_use]
    pub fn view(&self, window_id: iced_window::Id) -> Element<'_, Message> {
        if let Some(window) = self.windows.get(&window_id) {
            window
                .view()
                .map(move |msg| Message::Window(window_id, msg))
        } else {
            text("Scene not found.").into()
        }
    }

    fn save_scene(&self, scene: &Scene, mut path: PathBuf) -> Result<()> {
        log::info!(
            "Ssving current scene to {}.",
            path.to_str().unwrap_or_default()
        );
        if path.extension().and_then(|s| s.to_str()) != Some(Self::FILE_EXTENSION) {
            path.add_extension(Self::FILE_EXTENSION);
        }

        let swapfile_path = PROJECT_DIRS
            .cache_dir()
            .with_file_name(path.file_name().unwrap());

        {
            let data = SceneData::new(scene, &self.audio_cache)?;
            let mut swapfile = File::create_buffered(&swapfile_path)?;
            rmp_serde::encode::write(&mut swapfile, &data)?;
            log::debug!("Wrote save to swapfile.");
        }

        std::fs::rename(&swapfile_path, &path)?;
        log::info!(
            "Current scene saved to {}.",
            path.to_str().unwrap_or_default()
        );

        Ok(())
    }

    fn load_scene(&mut self, path: &Path) -> Result<Scene> {
        log::info!("Loading scene at {}.", path.to_str().unwrap_or_default());
        let scene_name = path.file_stem().unwrap().to_string_lossy();

        let file = File::open(path)?;
        let data: SceneData = rmp_serde::decode::from_read(file)?;
        log::debug!("Loaded savefile data.");

        let scene: Scene = Scene::from_data(data.with_name(&scene_name), &mut self.audio_cache)?;
        log::info!("Loaded scene {}.", scene.name());

        Ok(scene)
    }

    fn add_track(&mut self, window_id: iced_window::Id, path: &Path) -> Result<Message> {
        log::info!(
            "Loading track at {} for current scene.",
            path.to_str().unwrap_or_default(),
        );

        let name = path.file_stem().map_or("Unknown filename".into(), |s| {
            s.to_string_lossy().to_string()
        });
        log::debug!("Track name: {name}");

        let mut file = File::open_buffered(path)?;

        let data = self.audio_cache.get_or_register(&mut file)?;
        log::debug!("Registered track.");

        let id = Ulid::new();
        log::debug!("Track ID: {id}");

        log::info!("Loaded track {id} ({name}).");

        Ok(Message::Window(
            window_id,
            window::Message::Scene(scene::Message::TrackAdded { id, name, data }),
        ))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let initialized = Subscription::run(|| stream::once(async { Message::Initialized }));
        let windows = Subscription::batch(self.windows.iter().map(|(window_id, window)| {
            let window_id = *window_id;
            window
                .subscription()
                .with(window_id)
                .map(|(window_id, msg)| Message::Window(window_id, msg))
        }));
        let window_close = iced_window::close_requests().map(Message::CloseRequested);
        Subscription::batch([initialized, windows, window_close])
    }
}
