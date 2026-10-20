use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Compose,
    Volumes,
    Containers,
    Images,
    Networks,
    System,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Compose,
        Tab::Volumes,
        Tab::Containers,
        Tab::Images,
        Tab::Networks,
        Tab::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Compose => "Compose",
            Tab::Volumes => "Volumes",
            Tab::Containers => "Containers",
            Tab::Images => "Images",
            Tab::Networks => "Networks",
            Tab::System => "System",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub compose: Vec<ComposeProject>,
    pub volumes: Vec<VolumeInfo>,
    pub containers: Vec<ContainerInfo>,
    pub images: Vec<ImageInfo>,
    pub networks: Vec<NetworkInfo>,
    pub system: Vec<SystemDf>,
    pub engine_version: String,
    pub refreshed_at: Option<Instant>,
}

#[derive(Clone, Debug, Default)]
pub struct ComposeProject {
    pub name: String,
    pub status: String,
    pub config_files: Vec<String>,
    pub ports: Vec<PortLink>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortLink {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

impl PortLink {
    pub fn label(&self) -> String {
        if self.protocol.eq_ignore_ascii_case("tcp") {
            format!(":{}", self.host_port)
        } else {
            format!(":{}/{}", self.host_port, self.protocol)
        }
    }

    pub fn url(&self) -> Option<String> {
        if self.protocol.eq_ignore_ascii_case("tcp") {
            Some(format!("http://127.0.0.1:{}", self.host_port))
        } else {
            None
        }
    }
}

impl ComposeProject {
    pub fn is_running(&self) -> bool {
        self.status.starts_with("running")
    }
}

#[derive(Clone, Debug, Default)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub size: String,
    pub links: u64,
    pub labels: String,
    pub mountpoint: String,
}

impl VolumeInfo {
    pub fn in_use(&self) -> bool {
        self.links > 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: String,
    pub created: String,
    pub health: String,
    pub compose_project: Option<String>,
}

impl ContainerInfo {
    pub fn is_running(&self) -> bool {
        self.state.eq_ignore_ascii_case("running")
    }

    pub fn short_id(&self) -> &str {
        &self.id[..self.id.len().min(12)]
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImageInfo {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created: String,
    pub dangling: bool,
}

impl ImageInfo {
    pub fn display_name(&self) -> String {
        if self.dangling {
            format!("<none> ({})", self.short_id())
        } else {
            format!("{}:{}", self.repository, self.tag)
        }
    }

    pub fn short_id(&self) -> &str {
        let id = self.id.trim_start_matches("sha256:");
        &id[..id.len().min(12)]
    }
}

#[derive(Clone, Debug, Default)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub labels: String,
    pub builtin: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SystemDf {
    pub kind: String,
    pub total: u64,
    pub active: u64,
    pub size: String,
    pub reclaimable: String,
}

#[derive(Clone, Debug, Default)]
pub struct LogsView {
    pub title: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct ConfirmRequest {
    pub title: String,
    pub message: String,
    pub action: PendingAction,
    pub confirm_label: String,
    pub danger: bool,
    pub volume_checkbox: bool,
    pub remove_volumes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposeAction {
    Up,
    Stop,
    Start,
    Restart,
    Down,
    DownVolumes,
}

impl ComposeAction {
    pub fn label(self) -> &'static str {
        match self {
            ComposeAction::Up => "up",
            ComposeAction::Stop => "stop",
            ComposeAction::Start => "start",
            ComposeAction::Restart => "restart",
            ComposeAction::Down => "down",
            ComposeAction::DownVolumes => "down -v",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerAction {
    Start,
    Stop,
    Restart,
    Remove,
}

impl ContainerAction {
    pub fn label(self) -> &'static str {
        match self {
            ContainerAction::Start => "start",
            ContainerAction::Stop => "stop",
            ContainerAction::Restart => "restart",
            ContainerAction::Remove => "remove",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PruneKind {
    Containers,
    Images,
    Volumes,
    Networks,
    BuildCache,
}

impl PruneKind {
    pub fn label(self) -> &'static str {
        match self {
            PruneKind::Containers => "stopped containers",
            PruneKind::Images => "dangling images",
            PruneKind::Volumes => "unused volumes",
            PruneKind::Networks => "unused networks",
            PruneKind::BuildCache => "build cache",
        }
    }
}

#[derive(Clone, Debug)]
pub enum PendingAction {
    Compose {
        name: String,
        files: Vec<String>,
        action: ComposeAction,
    },
    VolumeRemove(Vec<String>),
    Container {
        id: String,
        name: String,
        action: ContainerAction,
    },
    ImageRemove(String),
    NetworkRemove(String),
    Prune(PruneKind),
}

#[derive(Clone, Debug)]
pub enum LogTarget {
    Compose { name: String, files: Vec<String> },
    Container { id: String, name: String },
}

#[derive(Clone, Debug)]
pub enum DockerCommand {
    Refresh,
    Compose {
        name: String,
        files: Vec<String>,
        action: ComposeAction,
    },
    VolumeRemove(Vec<String>),
    Container {
        id: String,
        name: String,
        action: ContainerAction,
    },
    ImageRemove(String),
    NetworkRemove(String),
    Prune(PruneKind),
    LoadLogs(LogTarget),
}

#[derive(Clone, Debug)]
pub enum DockerEvent {
    Snapshot(Box<Snapshot>),
    Done(String),
    Error(String),
    Logs(LogsView),
}
