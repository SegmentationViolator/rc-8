use std::sync::{self, mpsc};
use std::thread;

use crate::backend::interfaces;

const MESSAGE_BUFFER_SIZE: usize = 8;

pub struct FrontendHandle {
    command_handle: sync::Arc<(sync::Mutex<Command>, sync::Condvar)>,
    frontend: Option<super::Frontend>,
    join_handle: Option<thread::JoinHandle<super::Frontend>>,
    keyboard_handle: sync::Arc<sync::Mutex<interfaces::KeyboardState>>,
    receiver: Option<mpsc::Receiver<super::Message>>,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Command {
    None,
    Suspend,
    Stop,
}

impl FrontendHandle {
    pub fn resume(&mut self) {
        if !self.suspended() {
            panic!("attempt to resume the frontend thread while it's not suspended");
        }

        *self.command_handle.0.lock().unwrap() = Command::None;
        self.command_handle.1.notify_one();
    }

    pub fn start(&mut self) {
        if self.started() {
            panic!("attempt to start the already started frontend thread");
        }

        let frontend = self.frontend.take().unwrap();
        let command_handle = sync::Arc::clone(&self.command_handle);
        let keyboard_handle = sync::Arc::clone(&self.keyboard_handle);

        let (sender, receiver) = mpsc::sync_channel(MESSAGE_BUFFER_SIZE);

        let _ = self.receiver.insert(receiver);

        let _ = self.join_handle.insert(thread::spawn(|| {
            frontend.run(command_handle, keyboard_handle, sender)
        }));
    }

    pub fn stop(&mut self) -> &mut super::Frontend {
        if !self.started() {
            panic!("attempt to stop the already stopped frontend thread");
        }

        *self.command_handle.0.lock().unwrap() = Command::Stop;
        self.command_handle.1.notify_one();

        let join_handle = self.join_handle.take().unwrap();
        let frontend = self.frontend.insert(join_handle.join().unwrap());

        self.receiver.take();

        *self.command_handle.0.lock().unwrap() = Command::None;

        frontend
    }

    pub fn suspend(&mut self) {
        if !self.started() {
            panic!("attempt to suspend the frontend thread while it not started");
        }

        if self.suspended() {
            panic!("attempt to suspend the already suspended frontend thread");
        }

        *self.command_handle.0.lock().unwrap() = Command::Suspend;
    }
}

impl FrontendHandle {
    #[inline]
    pub fn get(&mut self) -> Option<&mut super::Frontend> {
        self.frontend.as_mut()
    }

    #[inline]
    pub fn keyboard_state<'a>(&'a mut self) -> sync::MutexGuard<'a, interfaces::KeyboardState> {
        self.keyboard_handle.lock().unwrap()
    }

    #[inline]
    pub fn message(&self) -> Option<super::Message> {
        self.receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok())
    }

    #[inline]
    pub fn new(frontend: super::Frontend) -> Self {
        Self {
            command_handle: (sync::Mutex::new(Command::None), sync::Condvar::new()).into(),
            frontend: Some(frontend),
            join_handle: None,
            keyboard_handle: sync::Arc::new(sync::Mutex::new(interfaces::KeyboardState::new())),
            receiver: None,
        }
    }

    #[inline]
    pub fn started(&self) -> bool {
        self.frontend.is_none()
    }

    #[inline]
    pub fn suspended(&self) -> bool {
        *self.command_handle.0.lock().unwrap() == Command::Suspend
    }
}
