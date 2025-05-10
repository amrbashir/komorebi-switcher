use std::io::{BufReader, Read};
use std::time::Duration;

use client::*;
use windows::Win32::Foundation::RECT;
use winit::event_loop::EventLoopProxy;

use crate::app::AppMessage;

mod client;

#[derive(Debug, Clone, Default)]
pub struct Workspace {
    pub name: String,
    pub index: usize,
    pub focused: bool,
    pub is_empty: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Monitor {
    pub name: String,
    pub index: usize,
    pub serial_number_id: String,
    pub workspaces: Vec<Workspace>,
    pub rect: RECT,
}

impl Monitor {
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
            })
            .collect();

        Self {
            index,
            name: monitor.name,
            serial_number_id: monitor.serial_number_id,
            workspaces,
            rect: RECT {
                left: monitor.size.left,
                top: monitor.size.top,
                right: monitor.size.right,
                bottom: monitor.size.bottom,
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub monitors: Vec<Monitor>,
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
    if let Err(e) = client::send_message(&change_msg) {
        tracing::error!("Failed to change workspace: {e}")
    }
}

#[cfg(debug_assertions)]
const SOCK_NAME: &str = "komorebi-switcher-debug.sock";
#[cfg(not(debug_assertions))]
const SOCK_NAME: &str = "komorebi-switcher.sock";

pub fn listen_for_state(proxy: EventLoopProxy<AppMessage>) {
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

        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&buffer) else {
            continue;
        };

        tracing::debug!(
            "Received an event from komorebi: {}",
            value
                .get("event")
                .and_then(|o| o.as_object())
                .and_then(|o| o.get("type"))
                .map(|v| v.to_string())
                .unwrap_or_default()
        );

        let Ok(notification) = serde_json::from_value::<KNotification>(value) else {
            continue;
        };

        if let Err(e) = proxy.send_event(AppMessage::UpdateKomorebiState(notification.state.into()))
        {
            tracing::error!("Failed to send `AppMessage::UpdateWorkspaces`: {e}")
        }
    }
}
