use eframe::egui::ColorImage;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Starting,
    LoadingManifest,
    LoadingBanners,
    Downloading,
    Verifying,
    Extracting,
    Installing,
    Completed,
    Error,
    Preview,
}

impl State {
    pub fn name(&self) -> &'static str {
        match self {
            State::Starting => "STARTING",
            State::LoadingManifest => "LOADING_MANIFEST",
            State::LoadingBanners => "LOADING_BANNERS",
            State::Downloading => "DOWNLOADING",
            State::Verifying => "VERIFYING",
            State::Extracting => "EXTRACTING",
            State::Installing => "INSTALLING",
            State::Completed => "COMPLETED",
            State::Error => "ERROR",
            State::Preview => "PREVIEW",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StateData {
    pub message: String,
    pub up_to_date: bool,
    pub current: String,
    pub target: String,
}

impl StateData {
    pub fn message(message: impl Into<String>) -> Self {
        Self { message: message.into(), ..Default::default() }
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({
            "message": self.message,
            "up_to_date": self.up_to_date,
            "current": self.current,
            "target": self.target,
        })
        .to_string()
    }
}

pub enum Event {
    State(State, StateData),
    Progress { ratio: f32 },
    Versions { current: String, target: String },
    BannersPartial(Vec<ColorImage>),
    BannersReady(Vec<ColorImage>),
}
