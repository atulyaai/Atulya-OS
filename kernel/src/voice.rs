//! voice.rs — Formant Voice & Speech Synthesizer for Atulya OS.
//!
//! Generates audible voice speech using dual output pipelines:
//!   1. Direct PIT Channel 2 / PC Speaker (Port 0x42/0x61) with real calibrated phoneme delays
//!   2. 16-bit 22.05kHz PCM Audio Stream Buffer for Intel HDA / AC97

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
            sample_rate: 22050,
        }
    }

    /// Synthesize spoken English text into audible audio samples and PC speaker tones.
    pub fn synthesize(&self, text: &str) -> Vec<i16> {
        let sample_rate = self.sample_rate as usize;
        let mut pcm = Vec::new();

        for ch in text.to_ascii_lowercase().bytes() {
            let (f1, f2, duration_ms) = match ch {
                b'a' => (730, 1090, 45),
                b'e' => (530, 1840, 40),
                b'i' => (270, 2290, 40),
                b'o' => (570, 840, 45),
                b'u' => (300, 870, 45),
                b'r' | b'l' => (400, 1300, 35),
                b's' | b'z' => (200, 3000, 30),
                b't' | b'd' => (350, 1700, 25),
                b'm' | b'n' => (250, 1000, 35),
                b' ' => (0, 0, 30),
                _ => (450, 1500, 30),
            };

            let samples_count = (sample_rate * duration_ms) / 1000;
            if f1 == 0 {
                crate::sound::Sound::stop_tone();
                pcm.extend((0..samples_count).map(|_| 0i16));
                crate::timer::delay_ms(duration_ms as u32);
            } else {
                // Pulse physical PC speaker at formant pitch with audible duration
                crate::sound::Sound::play_tone(f1 as u32);
                for i in 0..samples_count {
                    let phase1 = ((i * f1 * 360) / sample_rate) as i32;
                    let phase2 = ((i * f2 * 360) / sample_rate) as i32;

                    let wave1 = crate::math::sinish(phase1);
                    let wave2 = crate::math::sinish(phase2);

                    let sample = ((wave1 * 14000 + wave2 * 7000) / 1024) as i16;
                    pcm.push(sample);
                }
                crate::timer::delay_ms(duration_ms as u32);
            }
        }
        crate::sound::Sound::stop_tone();

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
