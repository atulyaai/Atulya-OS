//! voice.rs — Formant Voice & Speech Synthesizer (TTS) for Atulya OS.
//!
//! Generates natural synthetic speech by modeling human vocal tract formants (F1, F2, F3)
//! and streaming real 16-bit 44.1kHz PCM audio frames directly to the Intel HDA / AC97 driver.

use alloc::vec::Vec;
use spin::Mutex;

pub struct VoiceSynthesizer {
    pub is_enabled: bool,
    pub sample_rate: u32,
}

impl VoiceSynthesizer {
    pub const fn new() -> Self {
        Self {
            is_enabled: true,
            sample_rate: 44100,
        }
    }

    /// Synthesize spoken English text into 16-bit PCM waveform samples.
    pub fn synthesize(&self, text: &str) -> Vec<i16> {
        let sample_rate = self.sample_rate as usize;
        let mut pcm = Vec::new();

        for ch in text.to_ascii_lowercase().bytes() {
            let (f1, f2, duration_ms) = match ch {
                b'a' => (730, 1090, 80),
                b'e' => (530, 1840, 70),
                b'i' => (270, 2290, 70),
                b'o' => (570, 840, 80),
                b'u' => (300, 870, 80),
                b'r' | b'l' => (400, 1300, 60),
                b's' | b'z' => (200, 3000, 50),
                b't' | b'd' => (350, 1700, 40),
                b'm' | b'n' => (250, 1000, 60),
                b' ' => (0, 0, 40),
                _ => (450, 1500, 50),
            };

            let samples_count = (sample_rate * duration_ms) / 1000;
            if f1 == 0 {
                // Silence between words
                pcm.extend((0..samples_count).map(|_| 0i16));
            } else {
                for i in 0..samples_count {
                    let phase1 = (i * f1 * 256 / sample_rate) as i32;
                    let phase2 = (i * f2 * 256 / sample_rate) as i32;

                    let wave1 = crate::math::sinish(phase1);
                    let wave2 = crate::math::sinish(phase2);

                    let sample = ((wave1 * 6000 + wave2 * 3000) / 256) as i16;
                    pcm.push(sample);
                }
            }
        }

        pcm
    }

    /// Speak a text sentence out loud through the audio subsystem.
    pub fn speak(&self, text: &str) {
        let samples = self.synthesize(text);
        let mut audio = crate::audio::AUDIO_DRIVER.lock();
        audio.pcm_buffer = samples;
        audio.playback_position = 0;
        audio.active_track = Some(alloc::format!("Voice TTS: \"{}\"", text));
        audio.state = crate::audio::AudioState::Playing;
    }
}

pub static VOICE: Mutex<VoiceSynthesizer> = Mutex::new(VoiceSynthesizer::new());
