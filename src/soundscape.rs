use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};

use iced::{Element, Length::Fill, Subscription, window};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Vector2, track::Track};

mod canvas;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Translated {
        new_position: Vector2,
    },
    Scaled {
        new_scale: f32,
        new_position: Option<Vector2>,
    },
    NewFrame(Instant),
    NewWaypoint(Vector2),
    NewTrack {
        id: Ulid,
        name: String,
        position: Vector2,
        radius: f32,
    },
    TrackRemoved(Ulid),
    TrackMoved {
        id: Ulid,
        new_position: Vector2,
    },
    TrackSelected(Option<Ulid>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    MoveTrack(Ulid, Vector2),
    MoveListener(Vector2),
    ChangeSelection {
        deselected: Option<Ulid>,
        selected: Option<Ulid>,
    },
}

impl From<&Track> for Message {
    fn from(track: &Track) -> Self {
        Self::NewTrack {
            id: track.id(),
            name: track.name().to_string(),
            position: track.position(),
            radius: track.radius(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TrackZone {
    id: Ulid,
    name: String,
    position: Vector2,
    radius: f32,
}

impl TrackZone {
    const BORDER_WIDTH: f32 = 5.0;

    fn is_on_border(&self, point: Vector2) -> bool {
        let delta = self.position - point;
        let threshold = (delta.magnitude() - self.radius).abs();
        threshold <= Self::BORDER_WIDTH
    }

    fn contains(&self, point: Vector2) -> bool {
        let delta = self.position - point;
        delta.square_magnitude() <= self.radius * self.radius
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soundscape {
    listener: Listener,
    tracks: HashMap<Ulid, TrackZone>,
    camera: Vector2,
    scale: f32,
    #[serde(skip, default = "Instant::now")]
    current: Instant,
    waypoints: VecDeque<Vector2>,
    #[serde(skip)]
    selected_track: Option<Ulid>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
struct Listener {
    position: Vector2,
}

impl Soundscape {
    const OVERLAP_THRESHOLD: f32 = 30.0;
    const SPEED: f32 = 100.0;

    #[must_use]
    pub fn new() -> Self {
        Self {
            listener: Listener::default(),
            tracks: HashMap::new(),
            camera: Vector2::ZERO,
            scale: 1.0,
            current: Instant::now(),
            waypoints: VecDeque::new(),
            selected_track: None,
        }
    }

    pub fn update(&mut self, msg: Message) -> Option<Action> {
        match msg {
            Message::Translated { new_position } => {
                self.camera = new_position;
                None
            }
            Message::Scaled {
                new_scale,
                new_position,
            } => {
                self.scale = new_scale;
                if let Some(new_position) = new_position {
                    self.camera = new_position;
                }
                None
            }
            Message::NewFrame(instant) => {
                let dt = instant - self.current;
                self.current = instant;

                if let Some(next_waypoint) = self.waypoints.front() {
                    let velocity =
                        (*next_waypoint - self.listener.position).normalized() * Self::SPEED;
                    let dv = velocity * dt.as_secs_f32();
                    self.listener.position += dv;

                    while let Some(next_waypoint) = self.waypoints.front()
                        && (*next_waypoint - self.listener.position).square_magnitude()
                            < dv.square_magnitude()
                    {
                        self.waypoints.pop_front();
                    }

                    Some(Action::MoveListener(self.listener.position))
                } else {
                    None
                }
            }
            Message::NewWaypoint(point) => {
                if let Some(waypoint) = self.waypoints.back()
                    && (point - *waypoint).square_magnitude()
                        < Self::OVERLAP_THRESHOLD * Self::OVERLAP_THRESHOLD
                {
                    self.waypoints.pop_back();
                }
                self.waypoints.push_back(point);
                None
            }
            Message::NewTrack {
                id,
                name,
                position,
                radius,
            } => {
                self.tracks.insert(
                    id,
                    TrackZone {
                        id,
                        name,
                        position,
                        radius,
                    },
                );
                None
            }
            Message::TrackRemoved(id) => {
                self.tracks.remove(&id);
                None
            }
            Message::TrackMoved { id, new_position } => {
                if let Some(track) = self.tracks.get_mut(&id) {
                    track.position = new_position;
                }
                Some(Action::MoveTrack(id, new_position))
            }
            Message::TrackSelected(id) => {
                if id == self.selected_track {
                    None
                } else {
                    let deselected = self.selected_track;
                    let selected = id;
                    self.selected_track = id;

                    Some(Action::ChangeSelection {
                        deselected,
                        selected,
                    })
                }
            }
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::canvas(self).width(Fill).height(Fill).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        window::frames().map(Message::NewFrame)
    }

    #[must_use]
    pub fn listener_position(&self) -> Vector2 {
        self.listener.position
    }
}

impl Default for Soundscape {
    fn default() -> Self {
        Self::new()
    }
}
