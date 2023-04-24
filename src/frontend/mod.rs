use std::num;
use std::sync::{self, mpsc};
use std::thread;
use std::time;

use crate::backend::{self, interfaces};
use crate::defaults;

mod error;
mod handle;
mod sound;

pub use error::FrontendError;
pub use handle::FrontendHandle;
pub use sound::Sound;

pub type Message = Result<String, FrontendError>;

const FADE_DURATION: time::Duration = time::Duration::from_millis(1000 / 60 * 2);
const INSTRUCTIONS_PER_TICK: u16 = 18;
const TICK_INTERVAL: time::Duration = time::Duration::from_millis(1000 / 60);

#[derive(Clone, Copy)]
pub struct Colors {
    pub active: egui::Color32,
    pub inactive: egui::Color32,
}

pub struct Frontend {
    pub backend: backend::Backend,
    pub colors: Colors,
    context: egui::Context,
    display_buffer: interfaces::DisplayBuffer,
    display_texture: egui::TextureHandle,
    pub options: Options,
    sound: Sound,
    stream: rodio::OutputStreamHandle,
}

#[derive(Default)]
pub struct Options {
    pub debug_mode: bool,
    pub fade_effect: bool,
    pub wrap_sprites: bool,
}

impl Colors {
    fn get(&self, pixel: bool) -> egui::Color32 {
        match pixel {
            true => self.active,
            false => self.inactive,
        }
    }
}

impl Frontend {
    #[inline]
    pub fn display_texture(&self) -> egui::TextureId {
        self.display_texture.id()
    }

    #[inline]
    pub fn new(ctx: &egui::Context, options: Options, stream: rodio::OutputStreamHandle) -> Self {
        Self {
            colors: defaults::COLORS,
            context: ctx.clone(),
            backend: backend::Backend::new(),
            display_buffer: backend::interfaces::DisplayBuffer::new(interfaces::Options {
                track_changes: options.fade_effect,
                wrap_sprites: options.wrap_sprites,
            }),
            display_texture: ctx.load_texture(
                "Display Texture",
                egui::ColorImage::new(
                    [
                        backend::DISPLAY_BUFFER_WIDTH,
                        backend::DISPLAY_BUFFER_HEIGHT,
                    ],
                    defaults::COLORS.inactive,
                ),
                egui::TextureOptions::default(),
            ),
            options,
            sound: Sound::new().unwrap(),
            stream,
        }
    }

    pub fn reset(&mut self) {
        self.backend.reset();
        self.display_buffer.clear();
    }

    pub(self) fn run(
        mut self,
        command_handle: sync::Arc<(sync::Mutex<handle::Command>, sync::Condvar)>,
        keyboard_handle: sync::Arc<sync::Mutex<interfaces::KeyboardState>>,
        sender: mpsc::SyncSender<Message>,
    ) -> Self {
        let n = num::NonZeroU16::new(match self.options.debug_mode {
            true => 1,
            false => INSTRUCTIONS_PER_TICK,
        })
        .unwrap();

        let sink = match rodio::Sink::try_new(&self.stream) {
            Ok(sink) => sink,
            Err(error) => {
                let error = FrontendError::Play(error);
                sender
                    .send(Err(error))
                    .expect("receiver dropped before the frontend thread is stopped");

                return self;
            }
        };

        loop {
            let command = command_handle.0.lock().unwrap();

            match *command {
                handle::Command::None => drop(command),
                handle::Command::Stop => break,
                handle::Command::Suspend => {
                    let _ = command_handle.1.wait(command);
                    continue;
                }
            }

            if self.backend.timers.sound > 0 {
                self.sound.play(&sink)
            }

            let keyboard_state = keyboard_handle.lock().unwrap();

            match self
                .backend
                .tick(n, (&mut self.display_buffer, &keyboard_state))
            {
                Ok((index, instruction)) => {
                    if self.options.debug_mode {
                        sender
                            .send(Ok(format!(
                                "Executed intruction {} at 0x{:03x}",
                                instruction, index
                            )))
                            .expect("receiver dropped before the frontend thread is stopped");

                        let mut command = command_handle.0.lock().unwrap();
                        *command = handle::Command::Suspend;
                    }
                }
                Err(error) => {
                    let error = FrontendError::Backend(error);
                    let fatal = error.is_fatal();

                    sender
                        .send(Err(error))
                        .expect("receiver dropped before the frontend thread is stopped");

                    if fatal || self.options.debug_mode {
                        self.context.request_repaint();
                        break;
                    }

                    let mut command = command_handle.0.lock().unwrap();
                    *command = handle::Command::Suspend;
                }
            }

            if self.display_buffer.dirty {
                self.display_buffer.dirty = false;

                self.update_texture();
            }

            if !self.options.debug_mode {
                thread::sleep(TICK_INTERVAL);
            }
        }

        self
    }

    pub fn update_texture(&mut self) {
        let mut pixels: Vec<egui::Color32> =
            Vec::with_capacity(backend::DISPLAY_BUFFER_WIDTH * backend::DISPLAY_BUFFER_HEIGHT);

        for (y, row) in self.display_buffer.buffer.iter().enumerate() {
            for (x, pixel) in row.iter().enumerate() {
                if self.options.fade_effect {
                    let changed = self.display_buffer.changed.remove(&(x, y));

                    if let Some(timestamp) = changed {
                        let elapsed = timestamp.elapsed();

                        if elapsed < FADE_DURATION {
                            pixels.push(fade(
                                self.colors.active,
                                self.colors.inactive,
                                match elapsed.as_secs_f32() / FADE_DURATION.as_secs_f32() {
                                    x if x < 0.5 => 4.0,
                                    x if x < 0.75 => 2.0,
                                    _ => 1.3,
                                },
                            ));
                            self.display_buffer.changed.insert((x, y), timestamp);
                            self.display_buffer.dirty = true;

                            continue;
                        }
                    }
                }

                pixels.push(self.colors.get(*pixel));
            }
        }

        self.display_texture.set(
            egui::ColorImage {
                size: [
                    backend::DISPLAY_BUFFER_WIDTH,
                    backend::DISPLAY_BUFFER_HEIGHT,
                ],
                pixels,
            },
            egui::TextureOptions::NEAREST,
        );

        self.context.request_repaint();
    }
}

fn fade(src: egui::Color32, dst: egui::Color32, stp: f32) -> egui::Color32 {
    egui::Color32::from_rgb(
        (src.r().saturating_add(dst.r()) as f32 / stp) as u8,
        (src.g().saturating_add(dst.g()) as f32 / stp) as u8,
        (src.b().saturating_add(dst.b()) as f32 / stp) as u8,
    )
}
