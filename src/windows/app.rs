use std::collections::HashMap;

use windows_reactor::{
	Application, ApplicationContext, ApplicationView, ApplicationViewContext, NotifyIcon,
};

use super::menu::MenuCommand;
use super::message_window;
use super::taskbar::Taskbar;
use super::utils::OwnedHWND;
use super::windows::settings::{SettingsInput, SettingsWindow};
use super::windows::switcher::{SwitcherInput, SwitcherWindow};
use crate::config::Config;
use crate::komorebi::{self, State};

#[derive(Debug, Clone)]
pub enum AppMessage {
	UpdateKomorebiState(State),
	PreviewConfig(Option<Config>),
	SavedConfig(Config),
	TaskbarRecreated,
	MenuCommand(MenuCommand),
}

pub struct App {
	config: Config,
	preview: Option<Config>,
	komorebi_state: komorebi::State,
	taskbars: HashMap<String, Taskbar>,
	_message_window: OwnedHWND,
}

impl App {
	fn switcher_key(monitor_id: &str) -> String {
		format!("switcher:{monitor_id}")
	}

	fn sync_switchers(&mut self, context: &ApplicationContext<Self>, force_recreate: bool) {
		let taskbars = super::taskbar::all();
		let mut next = HashMap::new();

		for monitor in &self.komorebi_state.monitors {
			if let Some(taskbar) = taskbars.iter().find(|tb| monitor.rect.contains(tb.rect)) {
				match self.taskbars.get(&monitor.id) {
					Some(previous) if force_recreate || previous.hwnd != taskbar.hwnd => {
						context.recreate_window(Self::switcher_key(&monitor.id));
					}
					None => context.show_window(Self::switcher_key(&monitor.id)),
					_ => {}
				}

				next.insert(monitor.id.clone(), *taskbar);
			}
		}

		self.taskbars = next;
	}
}

impl Application for App {
	type Input = (Config, State);
	type Message = AppMessage;

	fn create((config, state): &Self::Input, context: &ApplicationContext<Self>) -> Self {
		let sender = context.thread_sender();
		let hwnd = unsafe { message_window::create(sender.clone()) }
			.expect("Failed to create message window");

		{
			let sender = sender.clone();
			std::thread::spawn(move || {
				crate::komorebi::listen_for_state(move |state| {
					let _ = sender.send(AppMessage::UpdateKomorebiState(state));
				});
			});
		}

		let mut app = Self {
			config: config.clone(),
			preview: None,
			komorebi_state: state.clone(),
			taskbars: HashMap::new(),
			_message_window: hwnd,
		};

		app.sync_switchers(context, false);

		app
	}

	fn update(&mut self, message: AppMessage, context: &ApplicationContext<Self>) {
		match message {
			AppMessage::UpdateKomorebiState(state) => {
				self.komorebi_state = state;
				self.sync_switchers(context, false);
			}

			AppMessage::MenuCommand(command) => match command {
				MenuCommand::Settings => context.show_window("settings"),
				MenuCommand::Refresh => self.sync_switchers(context, false),
				MenuCommand::Quit => context.exit(),
			},

			AppMessage::PreviewConfig(config) => self.preview = config,
			AppMessage::SavedConfig(config) => {
				self.config = config;
				self.preview = None;
			}

			AppMessage::TaskbarRecreated => {
				self.sync_switchers(context, true);
			}
		}
	}

	fn view(&self, _: &Self::Input, context: &ApplicationViewContext<Self>) -> ApplicationView {
		let menu = super::menu::create(context);

		let mut view = ApplicationView::new()
			.notify_icon(
				"tray",
				NotifyIcon::from_resource(1)
					.tooltip(crate::APP_NAME)
					.menu(menu.clone()),
			)
			.window::<SettingsWindow>(
				"settings",
				SettingsInput {
					config: self.config.clone(),
					monitors: self.komorebi_state.monitors.clone(),
					preview: context.callback(|config| AppMessage::PreviewConfig(Some(config))),
					saved: context.callback(AppMessage::SavedConfig),
					closed: context.message(AppMessage::PreviewConfig(None)),
				},
			);

		for monitor in &self.komorebi_state.monitors {
			if let Some(taskbar) = self.taskbars.get(&monitor.id) {
				view = view.window::<SwitcherWindow>(
					Self::switcher_key(&monitor.id),
					SwitcherInput {
						monitor: monitor.clone(),
						taskbar: *taskbar,
						config: self.preview.as_ref().unwrap_or(&self.config).clone(),
						context_menu: menu.clone(),
					},
				);
			}
		}

		view
	}
}
