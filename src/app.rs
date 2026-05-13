use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::Result;
use iced::{
    Element,
    Length::Fill,
    Subscription, Task,
    futures::stream,
    widget::{button, column, container, row, text},
    window as iced_window,
};
use log::{debug, info, warn};
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
    Save(iced_window::Id),
    Load,
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
                            info!("Received request to load new track for current scene.");

                            if let Some(path) = FileDialog::new()
                                .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                                .pick_file()
                            {
                                let msg = self
                                    .add_track(window_id, &path)
                                    .expect("should be able to add track");
                                Task::done(msg)
                            } else {
                                info!("Track load cancelled for current scene.");
                                Task::none()
                            }
                        }
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
            Message::Save(window_id) => {
                info!("Received request to save current scene.");

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
                        info!("Save cancelled.");
                    }
                } else {
                    warn!("Scene not found.");
                }

                Task::none()
            }
            Message::Load => {
                info!("Received request to load scene savefile.");
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

    pub fn view(&self, window_id: iced_window::Id) -> Element<'_, Message> {
        if let Some(window) = self.windows.get(&window_id) {
            let scene_info = container(row![
                text(window.scene().name()),
                button("Save")
                    .on_press(Message::Save(window_id))
                    .style(button::background),
                button("Load")
                    .on_press(Message::Load)
                    .style(button::background),
            ])
            .style(container::primary)
            .padding(4)
            .width(Fill);

            column![
                scene_info,
                window
                    .view()
                    .map(move |msg| Message::Window(window_id, msg)),
            ]
            .into()
        } else {
            text("Scene not found.").into()
        }
    }

    fn save_scene(&self, scene: &Scene, mut path: PathBuf) -> Result<()> {
        info!(
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

    fn add_track(&mut self, window_id: iced_window::Id, path: &Path) -> Result<Message> {
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
