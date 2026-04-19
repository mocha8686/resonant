use std::fs::File;

use crate::{
    PROJECT_DIRS,
    scene::{self, Scene, SceneData},
};
use iced::{
    Element,
    Length::Fill,
    Subscription, Task,
    widget::{button, column, container, row, text},
};
use rfd::FileDialog;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Scene(scene::Message),
    Save,
    Load,
}

pub struct App {
    scenes: Vec<Scene>,
    active_index: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            scenes: vec![Scene::default()],
            active_index: 0,
        }
    }
}

impl App {
    const FILE_EXTENSION: &'static str = "rst";

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Scene(msg) => Some(self.active_scene_mut().update(msg).map(Message::Scene)),
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

                    let swapfile_path =
                        PROJECT_DIRS.cache_dir().with_file_name(path.file_name().unwrap());

                    {
                        let data = SceneData::try_from(self.active_scene())
                            .expect("should be able to convert scene to data");

                        let mut swapfile = File::create_buffered(&swapfile_path)
                            .expect("should be able to open swapfile");

                        rmp_serde::encode::write(&mut swapfile, &data)
                            .expect("should be able to write to swapfile");
                    }

                    std::fs::rename(&swapfile_path, &path).expect("should be able to commit swapfile");
                }
                None
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

                    let scene: Scene = data
                        .with_name(&scene_name)
                        .try_into()
                        .expect("should be able to load scene from data");

                    self.scenes.push(scene);

                    Some(
                        self.active_scene_mut()
                            .update(scene::Message::Loaded)
                            .map(Message::Scene),
                    )
                } else {
                    None
                }
            }
        }
        .unwrap_or_else(Task::none)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let topbar = container(row![
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
