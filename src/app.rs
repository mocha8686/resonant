use std::{
    collections::HashMap,
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use iced::{
    Element, Subscription, Task,
    futures::stream,
    widget::text,
    window as iced_window,
};
use rfd::AsyncFileDialog;
use ulid::Ulid;

use crate::{
    PROJECT_DIRS,
    audio_cache::AudioCache,
    scene::{self, Scene, SceneData},
    window::{self, Window},
};

pub enum Message {
    Initialized,
    Window(iced_window::Id, window::Message),
    SaveRequested(iced_window::Id),
    Saved(iced_window::Id),
    LoadRequested,
    Loaded(Window),
    CloseRequested(iced_window::Id),
    Error(String),
}

pub struct App {
    windows: HashMap<iced_window::Id, Window>,
    audio_cache: Arc<Mutex<AudioCache>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            windows: HashMap::new(),
            audio_cache: Arc::new(Mutex::new(AudioCache::new())),
        }
    }
}

impl App {
    const SAVEFILE_EXTENSION: &'static str = "rst";

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
                        window::Action::AddTrack => self.add_track(window_id),
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
                        window::Action::Error(s) => Task::done(Message::Error(s)),
                    }
                } else {
                    Task::none()
                }
            }
            Message::SaveRequested(window_id) => self.save_scene(window_id),
            Message::Saved(window_id) => {
                Task::done(Message::Window(window_id, window::Message::Saved))
            }
            Message::LoadRequested => self.load_scene(),
            Message::Loaded(window) => {
                let window_id = window.id();
                self.windows.insert(window_id, window);
                Task::done(Message::Window(
                    window_id,
                    window::Message::Scene(scene::Message::Loaded),
                ))
            }
            Message::CloseRequested(id) => {
                Task::done(Message::Window(id, window::Message::CloseRequested))
            }
            Message::Error(s) => {
                // TODO: error toasts or smth
                log::error!("Error: {s}");
                Task::none()
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

    fn save_scene(&mut self, window_id: iced_window::Id) -> Task<Message> {
        if let Some(window) = self.windows.get(&window_id) {
            let scene_data = Arc::new(
                SceneData::new(window.scene(), &self.audio_cache)
                    .expect("should be able to create scene data"),
            );
            let scene_name = Arc::new(window.scene().name().to_string());
            log::info!("Received request to save scene {scene_name}.");

            Task::future(
                AsyncFileDialog::new()
                    .add_filter("resonant scene", &[Self::SAVEFILE_EXTENSION])
                    .set_file_name(format!("{}.{}", scene_name, Self::SAVEFILE_EXTENSION))
                    .save_file(),
            )
            .and_then(move |handle| {
                save_scene_task(
                    handle.path().to_owned(),
                    scene_name.to_string(),
                    scene_data.clone(),
                )
            })
            .map(move |res| match res {
                Ok(()) => Message::Saved(window_id),
                Err(e) => Message::Error(e.to_string()),
            })
        } else {
            log::warn!("Scene not found.");
            Task::none()
        }
    }

    fn load_scene(&mut self) -> Task<Message> {
        log::info!("Received request to load scene savefile.");
        let audio_cache = self.audio_cache.clone();

        Task::future(
            AsyncFileDialog::new()
                .add_filter(
                    concat!(env!("CARGO_PKG_NAME"), " scene"),
                    &[Self::SAVEFILE_EXTENSION],
                )
                .pick_file(),
        )
        .and_then(move |handle| load_scene_task(handle, audio_cache.clone()))
        .and_then(move |scene| {
            let (window, open) = Window::open(scene, iced_window::Settings::default());
            open.discard().chain(Task::done(Ok(window)))
        })
        .map(|res| match res {
            Ok(window) => Message::Loaded(window),
            Err(e) => Message::Error(e.to_string()),
        })
    }

    fn add_track(&mut self, window_id: iced_window::Id) -> Task<Message> {
        log::info!("Received request to load new track for current scene.");
        let audio_cache = self.audio_cache.clone();

        Task::future(
            AsyncFileDialog::new()
                .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                .pick_file(),
        )
        .and_then(move |handle| add_track_task(handle.path().to_owned(), audio_cache.clone()))
        .map(move |res| match res {
            Ok(msg) => Message::Window(window_id, window::Message::Scene(msg)),
            Err(e) => Message::Error(e.to_string()),
        })
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

fn add_track_task(path: PathBuf, audio_cache: Arc<Mutex<AudioCache>>) -> Task<Result<scene::Message>> {
    Task::future(async move {
        log::info!(
            "Loading track at {} for current scene.",
            path.to_str().unwrap_or_default(),
        );

        let name = path.file_stem().map_or("Unknown filename".into(), |s| {
            s.to_string_lossy().to_string()
        });
        log::debug!("Track name: {name}");

        let mut file = File::open_buffered(path)?;

        let data = audio_cache
            .clone()
            .lock()
            .unwrap()
            .get_or_register(&mut file)?;
        log::debug!("Registered track.");

        let id = Ulid::new();
        log::debug!("Track ID: {id}");
        log::info!("Loaded track {id} ({name}).");

        Ok(scene::Message::TrackAdded { id, name, data })
    })
}

fn save_scene_task(
    mut path: PathBuf,
    scene_name: String,
    scene_data: Arc<SceneData>,
) -> Task<Result<()>> {
    Task::future(async move {
        log::info!(
            "Saving scene {} to {}.",
            scene_name,
            path.to_str().unwrap_or_default()
        );

        if path.extension().and_then(|s| s.to_str()) != Some(App::SAVEFILE_EXTENSION) {
            path.add_extension(App::SAVEFILE_EXTENSION);
        }

        let swapfile_path = PROJECT_DIRS
            .cache_dir()
            .with_file_name(path.file_name().unwrap());

        {
            let mut swapfile = File::create_buffered(&swapfile_path)?;
            rmp_serde::encode::write(&mut swapfile, &*scene_data)?;
            log::debug!("Wrote save to swapfile.");
        }

        std::fs::rename(&swapfile_path, &path)?;
        log::info!(
            "Scene {} saved to {}.",
            scene_name,
            path.to_str().unwrap_or_default()
        );

        Ok(())
    })
}

fn load_scene_task(
    handle: rfd::FileHandle,
    audio_cache: Arc<Mutex<AudioCache>>,
) -> Task<Result<Scene>> {
    Task::future(async move {
        let path = handle.path();
        log::info!("Loading scene at {}.", path.to_str().unwrap_or_default());
        let scene_name = path.file_stem().unwrap().to_string_lossy();

        let file = File::open(path)?;
        let data: SceneData = rmp_serde::decode::from_read(file)?;
        log::debug!("Loaded savefile data.");

        let scene = {
            let mut audio_cache = audio_cache.lock().unwrap();
            Scene::from_data(data.with_name(&scene_name), &mut audio_cache)?
        };

        log::info!("Loaded scene {}.", scene.name());
        Ok(scene)
    })
}
