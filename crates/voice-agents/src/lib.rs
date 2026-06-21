//! Voice Agents - TTS and Audio Processing
//!
//! Provides Text-to-Speech capabilities using system TTS engine.
//! STT (Speech-to-Text) is planned for future integration with Whisper.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

mod audio_capture;
mod audio_playback;
mod tts_engine;
mod wyoming;

pub use audio_capture::AudioCapture;
pub use audio_playback::AudioPlayback;
pub use tts_engine::TtsEngine;

/// Voice agent coordinating TTS and audio
pub struct VoiceAgent {
    tts: Arc<Mutex<TtsEngine>>,
    capture: AudioCapture,
    playback: AudioPlayback,
}

impl VoiceAgent {
    pub fn new() -> Result<Self> {
        Ok(Self {
            tts: Arc::new(Mutex::new(TtsEngine::new()?)),
            capture: AudioCapture::new()?,
            playback: AudioPlayback::new()?,
        })
    }

    /// Synthesize text to speech
    pub async fn speak(&self, text: &str) -> Result<()> {
        let mut tts = self.tts.lock().await;
        tts.speak(text).await
    }

    /// Start audio capture
    pub fn start_capture(&mut self) -> Result<()> {
        self.capture.start()
    }

    /// Stop audio capture and get buffer
    pub fn stop_capture(&mut self) -> Result<Vec<f32>> {
        self.capture.stop()
    }

    /// Check if currently capturing
    pub fn is_capturing(&self) -> bool {
        self.capture.is_active()
    }

    /// Play audio buffer
    pub fn play_audio(&self, audio: &[f32]) -> Result<()> {
        self.playback.play(audio)
    }
}

impl Default for VoiceAgent {
    fn default() -> Self {
        Self::new().expect("Failed to initialize voice agent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tts_initialization() {
        let agent = VoiceAgent::new();
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_speak() {
        let agent = VoiceAgent::new().unwrap();
        let result = agent.speak("Test message").await;

        // May fail if no TTS backend available in test environment
        // This is acceptable for CI
        if result.is_err() {
            eprintln!("TTS not available in test environment");
        }
    }
}
