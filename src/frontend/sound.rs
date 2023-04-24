use std::io;
use std::sync;

const SOUND_OGG: &'static [u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/sound.ogg"));

pub struct Sound(sync::Arc<&'static [u8]>);

impl Sound {
    pub fn decode(
        &self,
    ) -> Result<rodio::Decoder<io::Cursor<Sound>>, rodio::decoder::DecoderError> {
        rodio::Decoder::new_vorbis(io::Cursor::new(Sound(sync::Arc::clone(&self.0))))
    }

    pub fn new() -> Result<Self, rodio::decoder::DecoderError> {
        let sound = Self(sync::Arc::new(SOUND_OGG));
        sound.decode()?;

        Ok(sound)
    }

    pub fn play(&self, sink: &rodio::Sink) {
        sink.append(self.decode().unwrap());
    }
}

impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
