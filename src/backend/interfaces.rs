use std::collections;
use std::time;

use bitvec::view::BitViewSized;

pub struct DisplayBuffer {
    pub buffer: [bitvec::BitArr!(for super::DISPLAY_BUFFER_WIDTH, in u64, bitvec::order::Msb0);
        super::DISPLAY_BUFFER_HEIGHT],
    pub changed: collections::HashMap<(usize, usize), time::Instant>,
    pub dirty: bool,
    pub options: Options,
}

pub struct KeyboardState([bool; super::KEY_COUNT]);

pub struct Options {
    pub track_changes: bool,
    pub wrap_sprites: bool,
}

impl DisplayBuffer {
    pub fn clear(&mut self) {
        for row in self.buffer.iter_mut() {
            row.fill(false);
        }

        self.dirty = true;
    }

    pub fn draw(&mut self, coordinates: (usize, usize), sprite: &[u8]) -> bool {
        let coordinates = (
            coordinates.0 % super::DISPLAY_BUFFER_WIDTH,
            coordinates.1 % super::DISPLAY_BUFFER_HEIGHT,
        );

        let mut collided = false;

        for (y, byte) in sprite.iter().enumerate() {
            let cy = (coordinates.1 + y) % super::DISPLAY_BUFFER_HEIGHT;

            for (x, bit) in byte
                .into_bitarray::<bitvec::order::Msb0>()
                .iter()
                .enumerate()
            {
                let cx = (coordinates.0 + x) % super::DISPLAY_BUFFER_WIDTH;

                if *bit {
                    let mut pixel = self.buffer[cy]
                        .get_mut(cx)
                        .unwrap();

                    if *pixel {
                        collided = true;

                        if self.options.track_changes {
                            self.changed
                                .insert((cx, cy), time::Instant::now());
                        }
                    }

                    pixel.set(!*pixel);
                };

                if !self.options.wrap_sprites && cx == super::DISPLAY_BUFFER_WIDTH - 1 {
                    break;
                }
            }

            if !self.options.wrap_sprites && cy == super::DISPLAY_BUFFER_HEIGHT - 1 {
                break;
            }
        }

        self.dirty = true;

        collided
    }

    #[inline]
    pub fn new(options: Options) -> Self {
        Self {
            buffer: [bitvec::array::BitArray::ZERO; super::DISPLAY_BUFFER_HEIGHT],
            changed: collections::HashMap::with_capacity(match options.track_changes {
                true => super::DISPLAY_BUFFER_WIDTH * super::DISPLAY_BUFFER_HEIGHT,
                false => 0,
            }),
            dirty: false,
            options,
        }
    }
}

impl KeyboardState {
    #[inline]
    pub fn hold(&mut self, key: usize) {
        self.0[key] = true
    }

    #[inline]
    pub fn new() -> Self {
        Self([false; super::KEY_COUNT])
    }

    #[inline]
    pub fn pressed(&self, key: usize) -> bool {
        self.0.get(key).copied().unwrap_or(false)
    }

    #[inline]
    pub fn pressed_key(&self) -> Option<usize> {
        self.0.iter().position(|pressed| *pressed)
    }

    #[inline]
    pub fn release(&mut self, key: usize) {
        self.0[key] = false
    }
}
