//! Native desktop UI for EBD. Uses the offline mock adapter until an authorised
//! client adapter exists; all adapter access goes through the capability gate.

use std::fs;
use std::path::{Path, PathBuf};

use ebd_config::{Config, LogFormat, LogLevel};
use ebd_contract::{Capability, Channel, Guild, Message};
use ebd_core::{AdapterManager, CompatibilityReport};
use ebd_mock_client::MockClient;
use ebd_plugin::{PluginGrant, PluginManifest, PluginPermission};
use ebd_registry::{Registry, TrustStore, TRUST_FILE};
use ebd_runtime::Runtime as PluginRuntime;
use ebd_theme::{ThemeManifest, ThemePreview};
use eframe::egui::{self, Color32};

const DEFAULT_THEME: &str = include_str!("../../../examples/theme/midnight.json");

const DEMO_GUEST: &str = r#"(module
    (import "env" "ebd_log" (func $log (param i32 i32)))
    (memory (export "memory") 1)
    (data (i32.const 0) "sandboxed plugin says hello")
    (func (export "run") (result i32)
        i32.const 0 i32.const 28 call $log
        i32.const 0))"#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Chat,
    Capabilities,
    Plugins,
    Themes,
    Packages,
    Config,
}

impl Tab {
    const ALL: [(Self, &'static str); 6] = [
        (Tab::Chat, "Chat"),
        (Tab::Capabilities, "Capabilities"),
        (Tab::Plugins, "Plugins"),
        (Tab::Themes, "Themes"),
        (Tab::Packages, "Packages"),
        (Tab::Config, "Config"),
    ];
}

struct App {
    manager: AdapterManager,
    config: Config,
    config_path: Option<PathBuf>,

    guilds: Vec<Guild>,
    channels: Vec<Channel>,
    messages: Vec<Message>,
    selected_guild: Option<String>,
    selected_channel: Option<String>,
    draft: String,
    status: String,

    tab: Tab,
    report: CompatibilityReport,

    plugin_path: String,
    plugin_result: String,
    sandbox_log: Vec<String>,

    theme_input: String,
    theme_preview: Option<ThemePreview>,
    theme_error: Option<String>,
    theme_applied: bool,
    dark_mode: bool,

    package_path: String,
    package_id: String,
    packages: Vec<ebd_registry::InstalledPackage>,
    trust_keys: Vec<String>,
}

fn config_path() -> Option<PathBuf> {
    ebd_config::default_path().ok()
}

fn storage_paths(path: Option<&Path>) -> Option<(PathBuf, PathBuf)> {
    let base = path
        .and_then(Path::parent)
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .or_else(|| config_path().and_then(|path| path.parent().map(Path::to_path_buf)))?;
    Some((base.join("packages"), base.join(TRUST_FILE)))
}

fn guilds(manager: &AdapterManager) -> Vec<Guild> {
    manager
        .adapter_for(Capability::ReadGuilds)
        .and_then(|adapter| adapter.guilds())
        .unwrap_or_default()
}

fn channels(manager: &AdapterManager, guild_id: &str) -> Vec<Channel> {
    manager
        .adapter_for(Capability::ReadChannels)
        .and_then(|adapter| adapter.channels(guild_id))
        .unwrap_or_default()
}

fn messages(manager: &AdapterManager, channel_id: &str) -> Vec<Message> {
    manager
        .adapter_for(Capability::ReadMessages)
        .and_then(|adapter| adapter.messages(channel_id, 50))
        .unwrap_or_default()
}

impl App {
    fn new() -> Self {
        let mut manager = AdapterManager::new();
        manager.attach(Box::new(MockClient));

        let config_path = config_path();
        let config = config_path
            .as_ref()
            .and_then(|path| Config::load(path).ok())
            .unwrap_or_default();

        let guilds = guilds(&manager);
        let selected_guild = guilds.first().map(|guild| guild.id.clone());
        let initial_channels = selected_guild
            .as_deref()
            .map(|id| channels(&manager, id))
            .unwrap_or_default();
        let selected_channel = initial_channels.first().map(|channel| channel.id.clone());
        let initial_messages = selected_channel
            .as_deref()
            .map(|id| messages(&manager, id))
            .unwrap_or_default();
        let report = manager.report().expect("adapter attached");

        let mut app = Self {
            manager,
            config,
            config_path,
            guilds,
            channels: initial_channels,
            messages: initial_messages,
            selected_guild,
            selected_channel,
            draft: String::new(),
            status: "Connected to the offline mock client.".into(),
            tab: Tab::Chat,
            report,
            plugin_path: String::new(),
            plugin_result: "Validate or run a plugin manifest to see results here.".into(),
            sandbox_log: Vec::new(),
            theme_input: DEFAULT_THEME.to_owned(),
            theme_preview: None,
            theme_error: None,
            theme_applied: false,
            dark_mode: true,
            package_path: String::new(),
            package_id: String::new(),
            packages: Vec::new(),
            trust_keys: Vec::new(),
        };

        app.apply_theme_text(DEFAULT_THEME);
        app.refresh_registry();
        app
    }

    fn select_guild(&mut self, id: String) {
        self.selected_guild = Some(id.clone());
        self.channels = channels(&self.manager, &id);
        self.selected_channel = self.channels.first().map(|channel| channel.id.clone());
        self.refresh_messages();
    }

    fn select_channel(&mut self, id: String) {
        self.selected_channel = Some(id);
        self.refresh_messages();
    }

    fn refresh_messages(&mut self) {
        let found = self
            .selected_channel
            .as_deref()
            .map(|id| messages(&self.manager, id));
        self.messages = found.unwrap_or_default();
    }

    fn send_message(&mut self) {
        let content = self.draft.trim().to_owned();
        let channel_id = match (self.selected_channel.as_deref(), content.is_empty()) {
            (Some(_), true) => {
                self.status = "Type a message first.".into();
                return;
            }
            (Some(id), false) => id.to_owned(),
            (None, _) => {
                self.status = "Select a channel first.".into();
                return;
            }
        };

        let result = self
            .manager
            .adapter_for(Capability::SendMessages)
            .and_then(|adapter| adapter.send_message(&channel_id, &content));
        match result {
            Ok(message) => {
                self.messages.push(message);
                self.draft.clear();
                self.status = "Message sent through the mock adapter.".into();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn registry(&self) -> Option<(Registry, TrustStore, PathBuf)> {
        let (root, trust_path) = storage_paths(self.config_path.as_deref())?;
        let trust = TrustStore::load(&trust_path).ok()?;
        Some((Registry::new(root, trust.clone()), trust, trust_path))
    }

    fn refresh_registry(&mut self) {
        if let Some((registry, trust, _)) = self.registry() {
            self.packages = registry.list().unwrap_or_default();
            self.trust_keys = trust
                .publishers()
                .iter()
                .map(|(name, key)| format!("{name}: {key}"))
                .collect();
        }
    }

    fn apply_theme_text(&mut self, source: &str) {
        match ThemeManifest::parse(source) {
            Ok(theme) => match theme.preview() {
                Ok(preview) => {
                    self.theme_preview = Some(preview);
                    self.theme_error = None;
                }
                Err(error) => self.theme_error = Some(error.to_string()),
            },
            Err(error) => self.theme_error = Some(error.to_string()),
        }
    }

    fn apply_visuals(&self, ctx: &egui::Context) {
        let mut visuals = if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        if let Some(preview) = &self.theme_preview {
            if let Some(color) = preview
                .swatches
                .get("background")
                .and_then(|hex| parse_hex(hex))
            {
                visuals.panel_fill = color;
                visuals.window_fill = color;
                visuals.extreme_bg_color = color;
            }
            if let Some(color) = preview
                .swatches
                .get("foreground")
                .and_then(|hex| parse_hex(hex))
            {
                visuals.override_text_color = Some(color);
                visuals.widgets.noninteractive.fg_stroke.color = color;
                visuals.widgets.inactive.fg_stroke.color = color;
            }
            if let Some(color) = preview
                .swatches
                .get("accent")
                .and_then(|hex| parse_hex(hex))
            {
                visuals.widgets.active.bg_fill = color;
                visuals.widgets.hovered.bg_fill = color;
                visuals.selection.bg_fill = color;
            }
            if let Some(color) = preview.swatches.get("muted").and_then(|hex| parse_hex(hex)) {
                visuals.widgets.noninteractive.bg_fill = color;
                visuals.widgets.inactive.bg_fill = color;
            }
        }
        ctx.set_visuals(visuals);
    }

    fn save_config(&mut self) {
        match &self.config_path {
            Some(path) => match self.config.save(path) {
                Ok(()) => self.status = format!("Saved configuration to {}", path.display()),
                Err(error) => self.status = error.to_string(),
            },
            None => self.status = "No config path; set EBD_CONFIG first.".into(),
        }
    }

    fn ui_chat(&mut self, ctx: &egui::Context) {
        let mut guild_click = None;
        egui::SidePanel::left("guilds")
            .resizable(true)
            .default_width(170.0)
            .show(ctx, |ui| {
                ui.heading("Communities");
                ui.separator();
                for guild in &self.guilds {
                    let selected = self.selected_guild.as_deref() == Some(guild.id.as_str());
                    if ui.selectable_label(selected, &guild.name).clicked() {
                        guild_click = Some(guild.id.clone());
                    }
                }
            });
        if let Some(id) = guild_click {
            self.select_guild(id);
        }

        let mut channel_click = None;
        egui::SidePanel::left("channels")
            .resizable(true)
            .default_width(190.0)
            .show(ctx, |ui| {
                ui.heading("Channels");
                ui.separator();
                for channel in &self.channels {
                    let selected = self.selected_channel.as_deref() == Some(channel.id.as_str());
                    if ui
                        .selectable_label(selected, format!("#{}", channel.name))
                        .clicked()
                    {
                        channel_click = Some(channel.id.clone());
                    }
                }
            });
        if let Some(id) = channel_click {
            self.select_channel(id);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let title = self
                .channels
                .iter()
                .find(|channel| Some(channel.id.as_str()) == self.selected_channel.as_deref())
                .map(|channel| format!("#{}", channel.name))
                .unwrap_or_else(|| "No channel selected".into());
            ui.heading(&title);
            ui.separator();

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for message in &self.messages {
                        ui.horizontal(|ui| {
                            ui.strong(format!("<{}>", message.author_id));
                            ui.label(&message.content);
                        });
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.text_edit_singleline(&mut self.draft).lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    self.send_message();
                }
                if ui.button("Send").clicked() {
                    self.send_message();
                }
            });
            ui.label(&self.status);
        });
    }

    fn ui_capabilities(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Adapter capabilities");
            ui.label(format!(
                "{} {} — contract {}",
                self.report.client.client_name,
                self.report.client.client_version,
                self.report.contract_version
            ));
            ui.separator();
            for capability in Capability::ALL {
                let supported = self.report.supports(capability);
                let icon = if supported { "✔" } else { "✘" };
                ui.horizontal(|ui| {
                    ui.label(icon);
                    ui.label(capability.as_str());
                });
            }
        });
    }

    fn ui_plugins(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Plugins");
            ui.label("Manifest path (plugin.json):");
            ui.text_edit_singleline(&mut self.plugin_path);

            ui.horizontal(|ui| {
                if ui.button("Validate").clicked() {
                    self.validate_plugin();
                }
                if ui.button("Run WASM").clicked() {
                    self.run_plugin();
                }
                if ui.button("Run sandbox demo").clicked() {
                    self.run_demo_guest();
                }
            });
            ui.separator();
            ui.label(&self.plugin_result);
            if !self.sandbox_log.is_empty() {
                ui.heading("Guest output");
                for line in &self.sandbox_log {
                    ui.label(format!("[plugin] {line}"));
                }
            }
        });
    }

    fn validate_plugin(&mut self) {
        let path = PathBuf::from(&self.plugin_path);
        match fs::read_to_string(&path) {
            Ok(contents) => match PluginManifest::parse(&contents) {
                Ok(manifest) => {
                    let grant = manifest.default_grant();
                    let permissions = grant
                        .permissions()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.plugin_result = format!(
                        "Valid manifest: {} {}. Permissions: {permissions}",
                        manifest.id, manifest.version
                    );
                }
                Err(error) => self.plugin_result = error.to_string(),
            },
            Err(error) => self.plugin_result = error.to_string(),
        }
    }

    fn run_plugin(&mut self) {
        let path = PathBuf::from(&self.plugin_path);
        let wasm_path = match fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|contents| {
                PluginManifest::parse(&contents).map_err(|error| error.to_string())
            }) {
            Ok(manifest) => path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&manifest.entry),
            Err(error) => {
                self.plugin_result = error;
                return;
            }
        };

        let grant = PluginGrant::new("ui.plugin", [PluginPermission::Log]);
        match fs::read(&wasm_path)
            .map_err(|error| error.to_string())
            .and_then(|wasm| {
                PluginRuntime::default()
                    .run(&wasm, &grant)
                    .map_err(|error| error.to_string())
            }) {
            Ok(execution) => {
                self.sandbox_log = execution.log;
                self.plugin_result = "Guest completed within sandbox limits.".into();
            }
            Err(error) => self.plugin_result = error,
        }
    }

    fn run_demo_guest(&mut self) {
        let wasm = match wat::parse_str(DEMO_GUEST) {
            Ok(wasm) => wasm,
            Err(error) => {
                self.plugin_result = error.to_string();
                return;
            }
        };
        let grant = PluginGrant::new("demo.plugin", [PluginPermission::Log]);
        match PluginRuntime::default().run(&wasm, &grant) {
            Ok(execution) => {
                self.sandbox_log = execution.log;
                self.plugin_result =
                    "Demo guest completed with fuel and memory limits active.".into();
            }
            Err(error) => self.plugin_result = error.to_string(),
        }
    }

    fn ui_themes(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Themes");
            ui.label("Theme JSON:");
            ui.text_edit_multiline(&mut self.theme_input);
            ui.horizontal(|ui| {
                if ui.button("Validate & preview").clicked() {
                    self.apply_theme_text(&self.theme_input.clone());
                }
                if ui.button("Apply to window").clicked() {
                    self.apply_theme_text(&self.theme_input.clone());
                    self.apply_visuals(ctx);
                }
                ui.checkbox(&mut self.dark_mode, "Dark base");
            });

            if let Some(error) = &self.theme_error {
                ui.colored_label(Color32::RED, error);
            }
            if let Some(preview) = &self.theme_preview {
                ui.heading("Swatches");
                ui.horizontal_wrapped(|ui| {
                    for (name, hex) in &preview.swatches {
                        if let Some(color) = parse_hex(hex) {
                            ui.colored_label(color, format!("{name}: {hex}"));
                        }
                    }
                });
                ui.heading("Generated CSS");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.code(&preview.css);
                    });
            }
        });
    }

    fn ui_packages(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Local packages");
            ui.label("Package directory:");
            ui.text_edit_singleline(&mut self.package_path);
            ui.horizontal(|ui| {
                if ui.button("Verify").clicked() {
                    self.package_action("verify");
                }
                if ui.button("Install").clicked() {
                    self.package_action("install");
                }
                if ui.button("Update").clicked() {
                    self.package_action("update");
                }
            });
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.package_id);
                if ui.button("Uninstall").clicked() {
                    self.package_action("uninstall");
                }
            });
            ui.separator();

            ui.heading("Installed");
            if self.packages.is_empty() {
                ui.label("No packages installed.");
            } else {
                for package in &self.packages {
                    ui.label(format!("{} {}", package.id, package.version));
                }
            }
            ui.heading("Trusted publishers");
            if self.trust_keys.is_empty() {
                ui.label("No trusted publishers.");
            } else {
                for key in &self.trust_keys {
                    ui.label(key);
                }
            }
            ui.separator();
            ui.label(&self.status);
        });
    }

    fn package_action(&mut self, action: &str) {
        let Some((registry, _, _)) = self.registry() else {
            self.status = "No package storage path available.".into();
            return;
        };
        let path = PathBuf::from(&self.package_path);

        let result = match action {
            "verify" => registry.verify(&path).map(|manifest| {
                format!(
                    "Verified {} {} by {}",
                    manifest.id, manifest.version, manifest.publisher
                )
            }),
            "install" => registry
                .install(&path)
                .map(|installed| format!("Installed {} {}", installed.id, installed.version)),
            "update" => registry
                .update(&path)
                .map(|installed| format!("Updated {} {}", installed.id, installed.version)),
            "uninstall" => registry
                .uninstall(&self.package_id)
                .map(|()| format!("Uninstalled {}", self.package_id)),
            other => Err(ebd_registry::RegistryError::Malformed(other.to_owned())),
        };

        self.status = match result {
            Ok(message) => message,
            Err(error) => error.to_string(),
        };
        self.refresh_registry();
    }

    fn ui_config(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Configuration");
            if let Some(path) = &self.config_path {
                ui.label(format!("File: {}", path.display()));
            }

            egui::ComboBox::from_label("Log level")
                .selected_text(self.config.logging.level.as_str())
                .show_ui(ui, |ui| {
                    for level in [
                        LogLevel::Error,
                        LogLevel::Warn,
                        LogLevel::Info,
                        LogLevel::Debug,
                        LogLevel::Trace,
                    ] {
                        ui.selectable_value(&mut self.config.logging.level, level, level.as_str());
                    }
                });

            let format_label = match self.config.logging.format {
                LogFormat::Text => "text",
                LogFormat::Json => "json",
            };
            egui::ComboBox::from_label("Log format")
                .selected_text(format_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.config.logging.format, LogFormat::Text, "text");
                    ui.selectable_value(&mut self.config.logging.format, LogFormat::Json, "json");
                });

            if ui.button("Save configuration").clicked() {
                self.save_config();
            }
            ui.label(&self.status);
            ui.separator();
            if let Ok(json) = serde_json::to_string_pretty(&self.config) {
                ui.code(&json);
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            self.apply_visuals(ctx);
            self.theme_applied = true;
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("EvenBetterDiscord");
                ui.separator();
                for (tab, label) in Tab::ALL {
                    if ui.selectable_label(self.tab == tab, label).clicked() {
                        self.tab = tab;
                    }
                }
            });
        });

        match self.tab {
            Tab::Chat => self.ui_chat(ctx),
            Tab::Capabilities => self.ui_capabilities(ctx),
            Tab::Plugins => self.ui_plugins(ctx),
            Tab::Themes => self.ui_themes(ctx),
            Tab::Packages => self.ui_packages(ctx),
            Tab::Config => self.ui_config(ctx),
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("EvenBetterDiscord"),
        ..Default::default()
    };

    eframe::run_native(
        "EvenBetterDiscord",
        options,
        Box::new(|_context| Ok(Box::new(App::new()))),
    )
}

fn parse_hex(value: &str) -> Option<Color32> {
    let hex = value.strip_prefix('#')?;
    let expand = |byte: u8| byte * 17;
    let component = |range: std::ops::Range<usize>| -> Option<u8> {
        u8::from_str_radix(hex.get(range)?, 16).ok()
    };

    match hex.len() {
        3 => Some(Color32::from_rgb(
            expand(component(0..1)?),
            expand(component(1..2)?),
            expand(component(2..3)?),
        )),
        6 => Some(Color32::from_rgb(
            component(0..2)?,
            component(2..4)?,
            component(4..6)?,
        )),
        8 => Some(Color32::from_rgba_premultiplied(
            component(0..2)?,
            component(2..4)?,
            component(4..6)?,
            component(6..8)?,
        )),
        _ => None,
    }
}
