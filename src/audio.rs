//! A small synthesized bell, independent of notification-daemon sound support.
use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, Ordering};

static PLAYING: AtomicBool = AtomicBool::new(false);
const SAMPLE_RATE: u32 = 44_100;
const DURATION: f32 = 1.6;

/// Audio device access and decoding must never block the UI thread.
/// Ignore overlapping previews/completions rather than playing several bells.
pub(crate) fn play_completion() {
    if PLAYING.swap(true, Ordering::AcqRel) {
        return;
    }
    if let Err(error) = std::thread::Builder::new()
        .name("completion-audio".into())
        .spawn(|| {
            struct Reset;
            impl Drop for Reset {
                fn drop(&mut self) {
                    PLAYING.store(false, Ordering::Release);
                }
            }
            let _reset = Reset;
            if let Err(error) = play(&chime_wav()) {
                eprintln!("Unable to play completion chime: {error}");
            }
        })
    {
        PLAYING.store(false, Ordering::Release);
        eprintln!("Unable to start completion audio: {error}");
    }
}

/// Mono, 16-bit PCM WAV. A gentle attack and release avoid clicks; the second
/// bell enters after 240 ms, with quiet overtones and a decaying tail.
fn chime_wav() -> Vec<u8> {
    let samples = (SAMPLE_RATE as f32 * DURATION) as u32;
    let data_len = samples * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1_u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let mut value = 0.0;
        for (start, frequency) in [(0.0, 880.0), (0.24, 1174.66)] {
            let elapsed = t - start;
            if elapsed >= 0.0 {
                let attack = (elapsed / 0.008).min(1.0);
                let envelope = attack * (-4.0 * elapsed).exp();
                value += envelope
                    * ((TAU * frequency * elapsed).sin()
                        + 0.22 * (TAU * frequency * 2.76 * elapsed).sin());
            }
        }
        let release = ((DURATION - t) / 0.05).clamp(0.0, 1.0);
        let sample = (value * release * 0.32 * i16::MAX as f32) as i16;
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// Use the same decoder pipeline for the built-in WAV and future custom sounds.
/// Owning the bytes lets the decoder outlive the caller's buffer.
fn decode(bytes: Vec<u8>) -> Result<rodio::Decoder<std::io::Cursor<Vec<u8>>>, String> {
    rodio::Decoder::try_from(std::io::Cursor::new(bytes))
        .map_err(|error| format!("Unable to decode notification audio: {error}"))
}

fn play(wav: &[u8]) -> Result<(), String> {
    let source = decode(wav.to_vec())?;
    let device = rodio::DeviceSinkBuilder::open_default_sink()
        .map_err(|error| format!("Unable to open audio output: {error}"))?;
    let player = rodio::Player::connect_new(device.mixer());
    player.append(source);
    // Both player and device must stay alive until playback ends. This wait is
    // on our worker thread, never the UI thread. Reopen on the next chime so
    // output-device changes and temporary audio failures can recover.
    player.sleep_until_end();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rodio_decodes_the_builtin_chime_without_an_audio_device() {
        use rodio::Source;

        let source = decode(chime_wav()).unwrap();
        assert_eq!(source.channels().get(), 1);
        assert_eq!(source.sample_rate().get(), SAMPLE_RATE);
        let samples: Vec<_> = source.collect();
        assert_eq!(samples.len(), (SAMPLE_RATE as f32 * DURATION) as usize);
        assert!(samples.iter().any(|sample| sample.abs() > 0.1));
    }

    #[test]
    fn invalid_audio_returns_a_decode_error() {
        assert!(decode(b"not an audio file".to_vec()).is_err());
    }

    #[test]
    fn chime_is_a_complete_pcm_wav() {
        let wav = chime_wav();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(&wav[36..40], b"data");
        let u32_at = |offset| u32::from_le_bytes(wav[offset..offset + 4].try_into().unwrap());
        assert_eq!(u32_at(4) as usize, wav.len() - 8);
        assert_eq!(u32_at(40) as usize, wav.len() - 44);
        assert_eq!(u32_at(24), SAMPLE_RATE);
        assert_eq!(&wav[20..24], &[1, 0, 1, 0]);
        assert_eq!(&wav[32..36], &[2, 0, 16, 0]);
        assert_eq!(wav.len(), 44 + (SAMPLE_RATE as f32 * DURATION) as usize * 2);
    }

    #[test]
    fn chime_is_audible_without_clipping_and_fades_to_silence() {
        let wav = chime_wav();
        let samples: Vec<i16> = wav[44..]
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        let peak = samples
            .iter()
            .map(|sample| i32::from(*sample).abs())
            .max()
            .unwrap();
        assert!(peak > 8_000, "chime must not be silent or too quiet");
        assert!(peak < i32::from(i16::MAX), "chime must not clip");
        assert_eq!(samples[0], 0);
        assert!(samples.last().unwrap().abs() <= 1);
    }
}
