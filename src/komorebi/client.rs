use std::cell::OnceCell;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use uds_windows::{UnixListener, UnixStream};

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum MaybeRingOrVec<T> {
    Ring(Ring<T>),
    Vec(Vec<T>),
}

impl<T> MaybeRingOrVec<T> {
    pub fn is_empty(&self) -> bool {
        match self {
            MaybeRingOrVec::Ring(ring) => ring.is_empty(),
            MaybeRingOrVec::Vec(vec) => vec.is_empty(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct KWorkspace {
    pub name: Option<String>,
    pub containers: Ring<serde_json::Value>,
    pub maximized_window: Option<serde_json::Value>,
    pub monocle_container: Option<serde_json::Value>,
    pub floating_windows: MaybeRingOrVec<serde_json::Value>,
}

impl KWorkspace {
    pub fn is_empty(&self) -> bool {
        self.containers.is_empty()
            && self.maximized_window.is_none()
            && self.monocle_container.is_none()
            && self.floating_windows.is_empty()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ring<T> {
    pub elements: Vec<T>,
    pub focused: usize,
}

impl<T> Ring<T> {
    pub fn focused_idx(&self) -> usize {
        self.focused
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

#[derive(Debug, Deserialize)]
pub struct KRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Deserialize)]
pub struct KMonitor {
    pub name: String,
    pub device_id: Option<String>,
    pub serial_number_id: Option<String>,
    pub workspaces: Ring<KWorkspace>,
    pub size: KRect,
}

#[derive(Debug, Deserialize)]
pub struct KState {
    pub monitors: Ring<KMonitor>,
}

#[derive(Debug, strum::Display, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum KSocketMessage {
    State,
    AddSubscriberSocket(String),
    FocusMonitorWorkspaceNumber(usize, usize),
}

#[derive(Debug, strum::Display, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum KSocketEvent {
    FocusWorkspaceNumber,
    FocusMonitorNumber,
    FocusMonitorWorkspaceNumber,
    FocusNamedWorkspace,
    FocusWorkspaceNumbers,
    CycleFocusMonitor,
    CycleFocusWorkspace,
    ReloadConfiguration,
    ReplaceConfiguration,
    CompleteConfiguration,
    ReloadStaticConfiguration,
    MoveContainerToMonitorNumber,
    MoveContainerToMonitorWorkspaceNumber,
    MoveContainerToNamedWorkspace,
    MoveContainerToWorkspaceNumber,
    MoveWorkspaceToMonitorNumber,
    CycleMoveContainerToMonitor,
    CycleMoveContainerToWorkspace,
    CycleMoveWorkspaceToMonitor,
    CloseWorkspace,
    SendContainerToMonitorNumber,
    SendContainerToMonitorWorkspaceNumber,
    SendContainerToNamedWorkspace,
    SendContainerToWorkspaceNumber,
    CycleSendContainerToMonitor,
    CycleSendContainerToWorkspace,
    Cloak,
    Uncloak,
    Destroy,
    FocusChange,
    Hide,
    Minimize,
    Show,
    TitleUpdate,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum KNotificationEvent {
    #[allow(unused)]
    Socket(KSocketEvent),
}

#[derive(Debug, Deserialize)]
pub struct KNotification {
    #[allow(unused)]
    pub event: KNotificationEvent,
    pub state: KState,
}

const KOMOREBI_SOCK: &str = "komorebi.sock";

fn komorebi_data_dir() -> anyhow::Result<Rc<PathBuf>> {
    thread_local! {
        static CELL: OnceCell<Option<Rc<PathBuf>>> = const { OnceCell::new() };
    }

    CELL.with(|cell| {
        cell.get_or_init(move || {
            dirs::data_local_dir()
                .map(|dir| dir.join("komorebi"))
                .map(Rc::new)
        })
        .clone()
        .context("couldn't find komorebi data dir")
    })
}

pub fn send_message(message: &KSocketMessage) -> anyhow::Result<()> {
    let socket = komorebi_data_dir()?.join(KOMOREBI_SOCK);

    let mut stream = UnixStream::connect(socket)?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    stream.write_all(serde_json::to_string(message)?.as_bytes())?;

    Ok(())
}

pub fn send_query(message: KSocketMessage) -> anyhow::Result<String> {
    let socket = komorebi_data_dir()?.join(KOMOREBI_SOCK);

    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    stream.write_all(serde_json::to_string(&message)?.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_to_string(&mut response)?;

    Ok(response)
}

pub fn subscribe(name: &str) -> anyhow::Result<UnixListener> {
    let socket = komorebi_data_dir()?.join(name);

    match std::fs::remove_file(&socket) {
        Ok(()) => {}
        Err(error) => match error.kind() {
            std::io::ErrorKind::NotFound => {}
            _ => {
                return Err(error.into());
            }
        },
    };

    let listener = UnixListener::bind(&socket)?;

    send_message(&KSocketMessage::AddSubscriberSocket(name.to_string()))?;

    Ok(listener)
}
