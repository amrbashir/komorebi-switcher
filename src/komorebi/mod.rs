use std::io::{BufReader, Read};
use std::sync::mpsc;
use std::time::Duration;

use client::*;

pub use crate::komorebi::client::KCycleDirection as CycleDirection;

mod client;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
#[allow(unused)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[cfg(windows)]
impl From<windows::Win32::Foundation::RECT> for Rect {
    fn from(rect: windows::Win32::Foundation::RECT) -> Self {
        Self {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        }
    }
}

impl Rect {
    #[allow(unused)]
    pub fn contains(&self, other: impl Into<Rect>) -> bool {
        let other = other.into();

        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workspace {
    pub name: String,
    pub index: usize,
    pub focused: bool,
    pub is_empty: bool,
    pub layout: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(unused)]
pub struct Monitor {
    pub name: String,
    pub index: usize,
    pub id: String,
    pub workspaces: Vec<Workspace>,
    pub rect: Rect,
}

impl Monitor {
    pub fn focused_workspace(&self) -> Option<&Workspace> {
        self.workspaces.iter().find(|ws| ws.focused)
    }

    fn from(monitor: KMonitor, index: usize) -> Self {
        let workspaces = monitor
            .workspaces
            .elements
            .iter()
            .enumerate()
            .map(|(idx, workspace)| Workspace {
                index: idx,
                focused: idx == monitor.workspaces.focused_idx(),
                is_empty: workspace.is_empty(),
                name: workspace
                    .name
                    .clone()
                    .unwrap_or_else(|| (idx + 1).to_string()),
                layout: workspace.layout.default.clone(),
            })
            .collect();

        let name = monitor.name.unwrap_or_default();

        let id = monitor
            .serial_number_id
            .or(monitor.device_id)
            .unwrap_or(name.clone());

        Self {
            index,
            name,
            id,
            workspaces,
            rect: Rect {
                left: monitor.size.left,
                top: monitor.size.top,
                // komorebi uses right and bottom as width and height
                // so we need to convert them to right and bottom coordinates
                right: monitor.size.left + monitor.size.right,
                bottom: monitor.size.top + monitor.size.bottom,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub monitors: Vec<Monitor>,
}

impl State {
    fn focused_workspaces_summary(&self) -> String {
        self.monitors
            .iter()
            .map(|monitor| {
                let monitor_name = if monitor.name.is_empty() {
                    monitor.id.as_str()
                } else {
                    monitor.name.as_str()
                };

                let workspace = monitor
                    .focused_workspace()
                    .map(|workspace| workspace.name.as_str())
                    .unwrap_or("<none>");

                format!("{monitor_name}:{workspace}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl From<KState> for State {
    fn from(state: KState) -> Self {
        Self {
            monitors: state
                .monitors
                .elements
                .into_iter()
                .enumerate()
                .map(|(idx, monitor)| Monitor::from(monitor, idx))
                .collect(),
        }
    }
}

pub fn read_state() -> anyhow::Result<State> {
    tracing::info!("Reading komorebi workspaces");

    let response = client::send_query(KSocketMessage::State)?;
    let state: KState = serde_json::from_str(&response)?;
    Ok(state.into())
}

pub fn change_workspace(monitor_idx: usize, workspace_idx: usize) {
    tracing::info!("Changing komorebi workspace to {workspace_idx} on monitor {monitor_idx}");

    let change_msg = KSocketMessage::FocusMonitorWorkspaceNumber(monitor_idx, workspace_idx);
    send_message_async(change_msg, "change workspace");
}

pub fn cycle_layout(direction: CycleDirection) {
    tracing::info!("Changing to {direction} komorebi layout");

    let change_msg = KSocketMessage::CycleLayout(direction);
    send_message_async(change_msg, "cycle layout");
}

fn send_message_async(message: KSocketMessage, action: &'static str) {
    std::thread::spawn(move || {
        if let Err(e) = client::send_message(&message) {
            tracing::error!("Failed to {action}: {e}");
        }
    });
}

#[cfg(debug_assertions)]
const SOCK_NAME: &str = "komorebi-switcher-debug.sock";
#[cfg(not(debug_assertions))]
const SOCK_NAME: &str = "komorebi-switcher.sock";

pub fn listen_for_state(on_new_state: impl Fn(State) + Send + 'static) {
    let (state_tx, state_rx) = mpsc::channel::<(String, State)>();
    std::thread::spawn(move || {
        let mut last_state = None;

        while let Ok((mut event, mut state)) = state_rx.recv() {
            let mut event_count = 1;

            while let Ok((next_event, next_state)) =
                state_rx.recv_timeout(Duration::from_millis(50))
            {
                event = next_event;
                state = next_state;
                event_count += 1;
            }

            if event == "AddSubscriberSocket" && last_state.is_none() {
                tracing::debug!(
                    "Initialized komorebi subscription state, focused: {}",
                    state.focused_workspaces_summary()
                );
                last_state = Some(state);
                continue;
            }

            if last_state.as_ref() == Some(&state) {
                tracing::debug!(
                    "Ignoring unchanged komorebi state after {event_count} event(s), last event: {event}"
                );
                continue;
            }

            tracing::debug!(
                "Applying komorebi state update after {event_count} event(s), last event: {event}, focused: {}",
                state.focused_workspaces_summary()
            );

            last_state = Some(state.clone());
            on_new_state(state);
        }
    });

    let socket = loop {
        match client::subscribe(SOCK_NAME) {
            Ok(socket) => break socket,
            Err(_) => std::thread::sleep(Duration::from_secs(1)),
        };
    };

    tracing::info!("Listenting for messages from komorebi");

    for client in socket.incoming() {
        let client = match client {
            Ok(i) => i,
            Err(e) => {
                tracing::error!("Error while receiving a client from komorebi: {e}");
                continue;
            }
        };

        match client.set_read_timeout(Some(Duration::from_secs(1))) {
            Ok(()) => {}
            Err(error) => tracing::error!("{}", error),
        }

        let mut buffer = Vec::new();
        let mut reader = BufReader::new(client);

        // this is when we know a shutdown has been sent
        if matches!(reader.read_to_end(&mut buffer), Ok(0)) {
            tracing::info!("Disconnected from komorebi");

            // keep trying to reconnect to komorebi
            let connect_message = KSocketMessage::AddSubscriberSocket(SOCK_NAME.into());
            while let Err(e) = client::send_message(&connect_message) {
                tracing::info!("Failed to reconnect to komorebi: {e}");
                std::thread::sleep(Duration::from_secs(1));
            }

            tracing::info!("Reconnected to komorebi");

            continue;
        }

        let value = match serde_json::from_slice::<serde_json::Value>(&buffer) {
            Ok(value) => value,
            Err(e) => {
                tracing::error!("Failed to parse komorebi message: {e}");
                continue;
            }
        };

        tracing::trace!("Received komorebi message: {value}");

        let event = value
            .get("event")
            .and_then(|o| o.as_object())
            .and_then(|o| o.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>")
            .to_string();

        tracing::trace!("Received an event from komorebi: {event}");

        let notification = match serde_json::from_value::<KNotification>(value) {
            Ok(notification) => notification,
            Err(e) => {
                tracing::error!("Failed to parse komorebi notification: {e}");
                continue;
            }
        };

        if let Err(e) = state_tx.send((event, State::from(notification.state))) {
            tracing::error!("Failed to queue komorebi state update: {e}");
        }
    }
}
