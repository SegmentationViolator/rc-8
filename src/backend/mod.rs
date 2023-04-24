use std::mem;
use std::num;

use crate::defaults;

mod error;
mod instruction;
pub mod interfaces;

pub use error::{BackendError, BackendErrorKind};
pub use instruction::Instruction;

pub const DISPLAY_BUFFER_ASPECT_RATIO: f32 = (DISPLAY_BUFFER_WIDTH / DISPLAY_BUFFER_HEIGHT) as f32;
pub const DISPLAY_BUFFER_HEIGHT: usize = 32;
pub const DISPLAY_BUFFER_WIDTH: usize = 64;
pub const CHARACTER_SIZE: usize = 5;
pub const FONT_SIZE: usize = CHARACTER_SIZE * KEY_COUNT;
pub const INSTRUCTIONS_PER_TICK: u16 = 700;
pub const KEY_COUNT: usize = 16;
pub const MEMORY_PADDING: usize = 512;
pub const MEMORY_SIZE: usize = 4096;
pub const REGISTER_COUNT: usize = 16;
pub const STACK_SIZE: usize = 12;

pub struct Backend {
    index: usize,
    loaded: bool,
    pub memory: [u8; MEMORY_SIZE],
    pub registers: Registers,
    pub stack: Vec<u16>,
    pub timers: Timers,
}
pub struct Registers {
    pub address: usize,
    pub general: [u8; REGISTER_COUNT],
}

pub struct Timers {
    pub delay: u8,
    pub sound: u8,
}

impl Backend {
    pub fn load(
        &mut self,
        font: Option<&[u8; FONT_SIZE]>,
        program: &[u8],
    ) -> Result<(), BackendError> {
        if program.len() > MEMORY_SIZE - MEMORY_PADDING || program.len() % 2 != 0 {
            return Err(BackendError {
                instruction: None,
                kind: BackendErrorKind::ProgramInvalid,
            });
        }

        if self.loaded {
            self.memory.fill(0);
        }

        self.memory[..FONT_SIZE].copy_from_slice(font.unwrap_or(&defaults::FONT));

        self.memory[MEMORY_PADDING..(MEMORY_PADDING + program.len())].copy_from_slice(program);
        self.loaded = true;

        Ok(())
    }

    #[inline]
    pub fn new() -> Self {
        Self {
            index: MEMORY_PADDING,
            loaded: false,
            memory: [0; MEMORY_SIZE],
            registers: Registers {
                address: 0,
                general: [0; REGISTER_COUNT],
            },
            stack: Vec::with_capacity(STACK_SIZE),
            timers: Timers { delay: 0, sound: 0 },
        }
    }

    pub fn reset(&mut self) {
        self.index = MEMORY_PADDING;

        self.registers.address = 0;
        self.registers.general.fill(0);

        self.stack.clear();

        self.timers.delay = 0;
        self.timers.delay = 0;
    }

    /// Executes `n` instructions and returns the index of the last instruction executed
    pub fn tick(
        &mut self,
        n: num::NonZeroU16,
        (display_buffer, keyboard_state): (
            &mut interfaces::DisplayBuffer,
            &interfaces::KeyboardState,
        ),
    ) -> Result<(usize, instruction::Instruction), BackendError> {
        if !self.loaded {
            return Err(BackendError {
                instruction: None,
                kind: BackendErrorKind::ProgramNotLoaded,
            });
        }

        self.timers.delay = self.timers.delay.saturating_sub(1);
        self.timers.sound = self.timers.sound.saturating_sub(1);

        let mut last_index = self.index;

        for _ in 0..n.get() {
            if self.index + 1 >= self.memory.len() {
                return Err(BackendError {
                    instruction: Some((self.index, None)),
                    kind: BackendErrorKind::MemoryOverflow,
                });
            }

            let instruction =
                Instruction::new([self.memory[self.index], self.memory[self.index + 1]]);

            last_index = self.index;
            self.index += mem::size_of::<Instruction>();

            match instruction.operator_code() {
                0x0 => match instruction.operand_nnn() {
                    0x0E0 => {
                        display_buffer.clear();
                    }

                    0x0EE => {
                        if self.stack.is_empty() {}

                        match self.stack.pop() {
                            None => {
                                return Err(BackendError {
                                    instruction: Some((last_index, Some(instruction))),
                                    kind: BackendErrorKind::StackUnderflow,
                                })
                            }
                            Some(address) => self.index = address as usize,
                        };
                    }
                    // Not implementing 0NNN, needs a 1802 or M6800 VM.
                    _ => {}
                },

                opcode @ (0x1 | 0x2) => {
                    if opcode == 2 {
                        if self.stack.len() == STACK_SIZE {
                            return Err(BackendError {
                                instruction: Some((last_index, Some(instruction))),
                                kind: BackendErrorKind::StackOverflow,
                            });
                        }

                        self.stack.push(self.index as u16);
                    }

                    self.index = instruction.operand_nnn() as usize;
                }

                opcode @ (0x3 | 0x4 | 0x5 | 0x9) => {
                    match opcode {
                        0x3 if self.registers.general[instruction.operand_x()]
                            == instruction.operand_nn() => {}

                        0x4 if self.registers.general[instruction.operand_x()]
                            != instruction.operand_nn() => {}

                        0x5 if self.registers.general[instruction.operand_x()]
                            == self.registers.general[instruction.operand_y()] => {}

                        0x9 if self.registers.general[instruction.operand_x()]
                            != self.registers.general[instruction.operand_y()] => {}

                        _ => continue,
                    }

                    self.index += mem::size_of::<Instruction>();
                }

                0x6 => self.registers.general[instruction.operand_x()] = instruction.operand_nn(),

                0x7 => {
                    self.registers.general[instruction.operand_x()] = self.registers.general
                        [instruction.operand_x()]
                    .wrapping_add(instruction.operand_nn())
                }

                0x8 => match instruction.operand_n() {
                    0x0 => {
                        self.registers.general[instruction.operand_x()] =
                            self.registers.general[instruction.operand_y()]
                    }

                    0x1 => {
                        self.registers.general[instruction.operand_x()] |=
                            self.registers.general[instruction.operand_y()]
                    }

                    0x2 => {
                        self.registers.general[instruction.operand_x()] &=
                            self.registers.general[instruction.operand_y()]
                    }

                    0x3 => {
                        self.registers.general[instruction.operand_x()] ^=
                            self.registers.general[instruction.operand_y()]
                    }

                    0x4 => {
                        let result = self.registers.general[instruction.operand_x()] as u16
                            + self.registers.general[instruction.operand_y()] as u16;

                        self.registers.general[15] = (result > u8::MAX as u16) as u8;
                        self.registers.general[instruction.operand_x()] =
                            (result & u8::MAX as u16) as u8
                    }

                    code @ (0x5 | 0x7) => {
                        let result;

                        match code {
                            0x5 => {
                                result = self.registers.general[instruction.operand_x()]
                                    .wrapping_sub(self.registers.general[instruction.operand_y()]);
                                self.registers.general[15] = (self.registers.general
                                    [instruction.operand_x()]
                                    > self.registers.general[instruction.operand_y()])
                                    as u8;
                            }

                            0x7 => {
                                result = self.registers.general[instruction.operand_y()]
                                    .wrapping_sub(self.registers.general[instruction.operand_x()]);
                                self.registers.general[15] = (self.registers.general
                                    [instruction.operand_y()]
                                    > self.registers.general[instruction.operand_x()])
                                    as u8;
                            }

                            _ => unreachable!(),
                        }

                        self.registers.general[instruction.operand_x()] = result
                    }

                    code @ (0x6 | 0xE) => {
                        let result;

                        match code {
                            0x6 => {
                                result = self.registers.general[instruction.operand_x()] >> 1;
                                self.registers.general[15] =
                                    self.registers.general[instruction.operand_x()] & 1;
                            }
                            0xE => {
                                result = self.registers.general[instruction.operand_x()] << 1;
                                self.registers.general[15] = self.registers.general
                                    [instruction.operand_x()]
                                    >> (u8::BITS - 1) as u8;
                            }
                            _ => unreachable!(),
                        }

                        self.registers.general[instruction.operand_x()] = result
                    }

                    _ => {
                        return Err(BackendError {
                            instruction: Some((last_index, Some(instruction))),
                            kind: BackendErrorKind::UnrecognizedInstruction,
                        })
                    }
                },

                0xA => self.registers.address = instruction.operand_nnn(),

                0xB => self.index = self.registers.general[0] as usize + instruction.operand_nnn(),

                0xC => {
                    self.registers.general[instruction.operand_x()] =
                        rand::random::<u8>() & instruction.operand_nn();
                }

                0xD => {
                    self.registers.general[15] = display_buffer.draw(
                        (
                            self.registers.general[instruction.operand_x()] as usize,
                            self.registers.general[instruction.operand_y()] as usize,
                        ),
                        &self.memory[self.registers.address as usize
                            ..self.registers.address as usize + instruction.operand_n() as usize],
                    ) as u8;
                }

                0xE => match instruction.operand_nn() {
                    0x9E => {
                        if keyboard_state
                            .pressed(self.registers.general[instruction.operand_x()] as usize)
                        {
                            self.index += mem::size_of::<instruction::Instruction>();
                        }

                        break;
                    }

                    0xA1 => {
                        if !keyboard_state
                            .pressed(self.registers.general[instruction.operand_x()] as usize)
                        {
                            self.index += mem::size_of::<instruction::Instruction>();
                        }

                        break;
                    }

                    _ => {
                        return Err(BackendError {
                            instruction: Some((last_index, Some(instruction))),
                            kind: BackendErrorKind::UnrecognizedInstruction,
                        })
                    }
                },

                0xF => match instruction.operand_nn() {
                    0x07 => self.registers.general[instruction.operand_x()] = self.timers.delay,

                    0x0A => {
                        if let Some(key) = keyboard_state.pressed_key() {
                            self.registers.general[instruction.operand_x()] = key as u8;
                        }

                        self.index = last_index;
                        break;
                    }

                    0x15 => self.timers.delay = self.registers.general[instruction.operand_x()],

                    0x18 => self.timers.sound = self.registers.general[instruction.operand_x()],

                    0x1E => {
                        self.registers.address = (self.registers.address
                            + self.registers.general[instruction.operand_x()] as usize)
                            & 0xFFF
                    }

                    0x29 => {
                        let character_code =
                            self.registers.general[instruction.operand_x()] as usize;

                        if character_code as usize >= KEY_COUNT {
                            return Err(BackendError {
                                instruction: Some((last_index, Some(instruction))),
                                kind: BackendErrorKind::UnrecognizedSprite,
                            });
                        }

                        self.registers.address = character_code * CHARACTER_SIZE;
                    }

                    0x33 => {
                        if self.registers.address + 2 >= self.memory.len() {
                            return Err(BackendError {
                                instruction: Some((self.index, None)),
                                kind: BackendErrorKind::MemoryOverflow,
                            });
                        }

                        let number = self.registers.general[instruction.operand_x()];

                        self.memory[self.registers.address] = (number / 10) / 10;
                        self.memory[self.registers.address + 1] = (number / 10) % 10;
                        self.memory[self.registers.address + 2] = number % 10;
                    }

                    0x55 => {
                        let x = instruction.operand_x() as usize;

                        if self.registers.address + x >= self.memory.len() {
                            return Err(BackendError {
                                instruction: Some((self.index, None)),
                                kind: BackendErrorKind::MemoryOverflow,
                            });
                        }

                        for i in 0..x + 1 {
                            self.memory[self.registers.address + i] = self.registers.general[i];
                        }
                    }

                    0x65 => {
                        let x = instruction.operand_x() as usize;

                        if self.registers.address + x >= self.memory.len() {
                            return Err(BackendError {
                                instruction: Some((self.index, None)),
                                kind: BackendErrorKind::MemoryOverflow,
                            });
                        }

                        for i in 0..x + 1 {
                            self.registers.general[i] = self.memory[self.registers.address + i];
                        }
                    }

                    _ => {
                        return Err(BackendError {
                            instruction: Some((last_index, Some(instruction))),
                            kind: BackendErrorKind::UnrecognizedInstruction,
                        })
                    }
                },

                _ => {
                    return Err(BackendError {
                        instruction: Some((last_index, Some(instruction))),
                        kind: BackendErrorKind::UnrecognizedInstruction,
                    })
                }
            }
        }

        Ok((
            last_index,
            instruction::Instruction::new([self.memory[last_index], self.memory[last_index + 1]]),
        ))
    }
}
