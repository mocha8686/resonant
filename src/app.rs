use std::fs::File;

use crate::{
    PROJECT_DIRS,
    audio_cache::AudioCache,
    scene::{self, Scene, SceneData},
};
use iced::{
    Element,
    Length::Fill,
    Subscription, Task,
    widget::{button, column, container, row, text},
};
use rfd::FileDialog;
use ulid::Ulid;

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
                            if let Some(path) = FileDialog::new()
                                .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                                .pick_file()
                            {
                                let name =
                                    path.file_stem().map_or("Unknown filename".into(), |s| {
                                        s.to_string_lossy().to_string()
                                    });
                                let mut file = File::open_buffered(&path)
                                    .expect("should be able to open audio file");

                                let data = self
                                    .audio_cache
                                    .get_or_register(&mut file)
                                    .expect("should be able to register new audio");

                                let id = Ulid::new();
                                let msg =
                                    Message::Scene(scene::Message::TrackAdded { id, name, data });

                                Task::done(msg)
                            } else {
                                Task::none()
                            }
                        }
                    }
                } else {
                    Task::none()
                }
            }
            Message::SwitchScene(index) => {
                self.active_index = index;
                Task::none()
            }
            Message::Save => {
                if let Some(mut path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .set_file_name(format!(
                        "{}.{}",
                        self.active_scene().name(),
                        Self::FILE_EXTENSION
                    ))
                    .save_file()
                {
                    if path.extension().and_then(|s| s.to_str()) != Some(Self::FILE_EXTENSION) {
                        path.add_extension(Self::FILE_EXTENSION);
                    }

                    let swapfile_path = PROJECT_DIRS
                        .cache_dir()
                        .with_file_name(path.file_name().unwrap());

                    {
                        let data = SceneData::new(self.active_scene(), &self.audio_cache)
                            .expect("should be able to convert scene to data");

                        let mut swapfile = File::create_buffered(&swapfile_path)
                            .expect("should be able to open swapfile");

                        rmp_serde::encode::write(&mut swapfile, &data)
                            .expect("should be able to write to swapfile");
                    }

                    std::fs::rename(&swapfile_path, &path)
                        .expect("should be able to commit swapfile");
                }
                Task::none()
            }
            Message::Load => {
                if let Some(path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .pick_file()
                {
                    let scene_name = path.file_stem().unwrap().to_string_lossy();

                    let file = File::open(&path).expect("should be able to open save file");
                    let data: SceneData = rmp_serde::decode::from_read(file)
                        .expect("should be able to read scene data");

                    let scene: Scene =
                        Scene::from_data(data.with_name(&scene_name), &mut self.audio_cache)
                            .expect("should be able to load scene from data");

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
