use std::fmt::Write;
use std::path;
use std::time;

use egui::color_picker;

use crate::backend;
use crate::frontend;

mod file_picker;

const ERROR_DISPLAY_DURATION: time::Duration = time::Duration::from_secs(2);
const MENU_SPACING: f32 = 2.5;
const PRIMARY_COLOR: egui::Color32 = egui::Color32::from_rgb(0x81, 0x5B, 0xA4);
const SECONDARY_COLOR: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x1C, 0x1C);

pub struct App {
    _stream: rodio::OutputStream,
    display_texture: egui::TextureId,
    file_picker: file_picker::FilePicker,
    frontend: frontend::FrontendHandle,
    state: State,
}

struct Error {
    message: String,
    timestamp: time::Instant,
}

enum Selection {
    Font,
    Program,
}

struct State {
    colors: frontend::Colors,
    debug_mode: bool,
    error: Error,
    fade_effect: bool,
    menu_raised: bool,
    font_path: Option<path::PathBuf>,
    program_path: Option<path::PathBuf>,
    selection: Selection,
}

impl App {
    fn handle_input(&mut self, ctx: &egui::Context) {
        if self.frontend.started() {
            let mut input = ctx.input_mut();

            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                if !self.state.menu_raised {
                    if !self.frontend.suspended() {
                        self.frontend.suspend();
                    }

                    self.state.menu_raised = true;
                    return;
                }

                if !self.state.debug_mode {
                    self.frontend.resume();
                }

                self.state.menu_raised = false;
            }

            if !self.state.debug_mode || input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
            {
                if self.state.debug_mode {
                    self.frontend.resume();
                }

                if let Some(message) = self.frontend.message() {
                    match message {
                        Ok(message) => {
                            eprintln!("{}", message);
                        }
                        Err(error) => {
                            if error.is_fatal() {
                                self.state.error.message.clear();
                                let _ = write!(self.state.error.message, "fatal error, {}", error);
                                return self.frontend.stop().reset();
                            }

                            eprintln!("{}", error);
                        }
                    }
                }
            }
        }
    }

    fn menu(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.file_picker.show(ctx) {
            match self.state.selection {
                Selection::Font => self.state.font_path.insert(path),
                Selection::Program => self.state.program_path.insert(path),
            };
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(
                !self.frontend.started() && !self.file_picker.is_open(),
                |ui| {
                    ui.add_visible_ui(
                        !self.state.error.message.is_empty()
                            && self.state.error.timestamp.elapsed() < ERROR_DISPLAY_DURATION,
                        |ui| {
                            ui.vertical_centered_justified(|ui| {
                                ui.colored_label(egui::Color32::RED, &self.state.error.message)
                            });

                            ctx.request_repaint_after(ERROR_DISPLAY_DURATION);
                        },
                    );

                    ui.heading("Backend Parameters");
                    ui.separator();

                    for item_data in [
                        ("Font", &mut self.state.font_path, Selection::Font),
                        ("Program", &mut self.state.program_path, Selection::Program),
                    ] {
                        menu_item(ui, item_data.0, |ui| {
                            if item_data.1.is_some()
                                && ui
                                    .add(
                                        egui::Label::new(
                                            egui::RichText::new("×").color(PRIMARY_COLOR),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                            {
                                *item_data.1 = None;
                            }

                            let file_name = item_data
                                .1
                                .as_ref()
                                .and_then(|path| path.file_name())
                                .and_then(|file_name| file_name.to_str());

                            ui.colored_label(
                                egui::Color32::LIGHT_GRAY,
                                file_name.unwrap_or("None"),
                            );
                        });
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            if ui
                                .selectable_label(false, format!("📂 Load {}", item_data.0))
                                .clicked()
                            {
                                self.state.error.message.clear();
                                self.file_picker.open();
                                self.state.selection = item_data.2;
                            }
                        });

                        ui.add_space(MENU_SPACING);
                    }

                    ui.add_space(MENU_SPACING.powi(3) - MENU_SPACING);

                    ui.heading("Frontend Parameters");
                    ui.separator();

                    for item_data in [
                        ("Active Color", &mut self.state.colors.active),
                        ("Inactive Color", &mut self.state.colors.inactive),
                    ] {
                        menu_item(ui, item_data.0, |ui| {
                            color_picker::color_edit_button_srgba(
                                ui,
                                item_data.1,
                                color_picker::Alpha::Opaque,
                            );
                        });

                        ui.add_space(MENU_SPACING);
                    }

                    menu_item(ui, "Fade Effect", |ui| {
                        ui.checkbox(&mut self.state.fade_effect, "");
                    });

                    ui.add_space(MENU_SPACING);

                    if self.state.program_path.is_some() && !self.frontend.started() {
                        ui.separator();

                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            if ui.button("▶ Start").clicked() {
                                self.start();
                            }
                        });
                    }
                },
            );

            if self.frontend.started() {
                ui.separator();

                ui.vertical_centered_justified(|ui| {
                    if ui.button("■ Stop").clicked() {
                        self.frontend.stop().reset();
                    }
                });
            }
        });
    }

    pub fn new(cc: &eframe::CreationContext, options: frontend::Options) -> Self {
        let mut visuals = cc.egui_ctx.style().visuals.clone();

        visuals.selection.bg_fill = PRIMARY_COLOR;
        visuals.selection.stroke.color = egui::Color32::WHITE;

        visuals.widgets.hovered.bg_fill = PRIMARY_COLOR;

        visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;

        visuals.window_fill = SECONDARY_COLOR;
        cc.egui_ctx.set_visuals(visuals);

        let (stream, handle) = rodio::OutputStream::try_default().unwrap();

        let debug_mode = options.debug_mode;
        let fade_effect = options.fade_effect;
        let frontend = frontend::Frontend::new(&cc.egui_ctx, options, handle);
        let state = State {
            colors: frontend.colors,
            debug_mode,
            fade_effect,
            error: Error {
                message: String::with_capacity(128),
                timestamp: time::Instant::now(),
            },
            menu_raised: false,
            font_path: None,
            program_path: None,
            selection: Selection::Font,
        };

        Self {
            _stream: stream,
            display_texture: frontend.display_texture(),
            file_picker: file_picker::FilePicker::new(),
            frontend: frontend::FrontendHandle::new(frontend),
            state,
        }
    }

    pub fn start(&mut self) {
        self.state.error.message.clear();

        let boxed;
        let frontend = self.frontend.get().unwrap();

        let font: Option<&[u8; backend::FONT_SIZE]> =
            match file_picker::FilePicker::load(self.state.font_path.as_ref()) {
                Ok(Some(font)) if font.len() == backend::FONT_SIZE => {
                    boxed = font.into_boxed_slice(); // store the boxed slice so that it is not dropped immediately

                    Some(boxed.as_ref().try_into().unwrap())
                }

                Ok(Some(_)) => {
                    self.state.font_path = None;
                    self.state.error.timestamp = time::Instant::now();
                    self.state
                        .error
                        .message
                        .push_str("couldn't load the font, attempt to load invalid font");

                    return;
                }

                Ok(None) => None,

                Err(error) => {
                    self.state.font_path = None;
                    self.state.error.timestamp = time::Instant::now();
                    let _ = write!(
                        self.state.error.message,
                        "couldn't load the font, {}",
                        error
                    );
                    return;
                }
            };
        let program = match file_picker::FilePicker::load(self.state.program_path.as_ref()) {
            Ok(program) => program.unwrap(),

            Err(error) => {
                self.state.program_path = None;
                self.state.error.timestamp = time::Instant::now();
                let _ = write!(
                    self.state.error.message,
                    "couldn't load the program, {}",
                    error
                );
                return;
            }
        };

        frontend.colors = self.state.colors;
        frontend.options.debug_mode = self.state.debug_mode;
        frontend.options.fade_effect = self.state.fade_effect;

        frontend.update_texture();
        match frontend.backend.load(font, &program) {
            Ok(()) => (),
            Err(error) => {
                self.state.program_path = None;
                self.state.error.timestamp = time::Instant::now();
                let _ = write!(
                    self.state.error.message,
                    "couldn't load the program, {}",
                    error
                );
                return;
            }
        };

        self.frontend.start();
        self.state.menu_raised = false;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.handle_input(ctx);

        if !self.frontend.started() || self.state.menu_raised {
            return self.menu(ctx);
        }

        let window_size = frame.info().window_info.size;
        let size;
        let margin;

        if window_size[0] / window_size[1] <= backend::DISPLAY_BUFFER_ASPECT_RATIO
            && window_size[0] > window_size[1]
        {
            size = window_size;
            margin = egui::style::Margin::same(0.0);
        } else {
            size = egui::vec2(
                window_size[0],
                window_size[0] / backend::DISPLAY_BUFFER_ASPECT_RATIO,
            );
            margin = egui::style::Margin::symmetric(0.0, (window_size[1] - size[1]) / 2.0);
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(margin))
            .show(ctx, |ui| {
                ui.add(egui::Image::new(self.display_texture, size));
            });
    }
}

pub fn menu_item(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.label(text)
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), add_contents);
    });
}
