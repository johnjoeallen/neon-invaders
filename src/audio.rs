use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

pub struct SoundBank {
    player_shoot: Sound,
    enemy_shoot: Sound,
    enemy_explosion: Sound,
    player_hit: Sound,
    wave_clear: Sound,
    game_over: Sound,
    march_low: Sound,
    march_high: Sound,
}

impl SoundBank {
    pub async fn load() -> Option<Self> {
        Some(Self {
            player_shoot: load_sound_from_bytes(&build_wav_bytes(&build_player_shoot()))
                .await
                .ok()?,
            enemy_shoot: load_sound_from_bytes(&build_wav_bytes(&build_enemy_shoot()))
                .await
                .ok()?,
            enemy_explosion: load_sound_from_bytes(&build_wav_bytes(&build_enemy_explosion()))
                .await
                .ok()?,
            player_hit: load_sound_from_bytes(&build_wav_bytes(&build_player_hit()))
                .await
                .ok()?,
            wave_clear: load_sound_from_bytes(&build_wav_bytes(&build_wave_clear()))
                .await
                .ok()?,
            game_over: load_sound_from_bytes(&build_wav_bytes(&build_game_over()))
                .await
                .ok()?,
            march_low: load_sound_from_bytes(&build_wav_bytes(&build_march_low()))
                .await
                .ok()?,
            march_high: load_sound_from_bytes(&build_wav_bytes(&build_march_high()))
                .await
                .ok()?,
        })
    }

    pub fn play_player_shoot(&self) {
        play(&self.player_shoot, 0.45);
    }

    pub fn play_enemy_shoot(&self) {
        play(&self.enemy_shoot, 0.35);
    }

    pub fn play_enemy_explosion(&self) {
        play(&self.enemy_explosion, 0.6);
    }

    pub fn play_player_hit(&self) {
        play(&self.player_hit, 0.55);
    }

    pub fn play_wave_clear(&self) {
        play(&self.wave_clear, 0.55);
    }

    pub fn play_game_over(&self) {
        play(&self.game_over, 0.6);
    }

    pub fn play_march(&self, high: bool) {
        if high {
            play(&self.march_high, 0.28);
        } else {
            play(&self.march_low, 0.28);
        }
    }
}

fn play(sound: &Sound, volume: f32) {
    play_sound(
        sound,
        PlaySoundParams {
            looped: false,
            volume,
        },
    );
}

fn build_wav_bytes(samples: &[i16]) -> Vec<u8> {
    let sample_rate = 44_100u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let block_align = channels * (bits_per_sample / 8);
    let byte_rate = sample_rate * u32::from(block_align);
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn build_player_shoot() -> Vec<i16> {
    synth(0.12, |t| 1250.0 - t * 5200.0, Wave::Pulse(0.25), 0.55, 0.05)
}

fn build_enemy_shoot() -> Vec<i16> {
    synth(0.18, |t| 680.0 + t * 420.0, Wave::Triangle, 0.32, 0.02)
}

fn build_enemy_explosion() -> Vec<i16> {
    synth(0.34, |t| 240.0 - t * 180.0, Wave::Noise, 0.6, 0.12)
}

fn build_player_hit() -> Vec<i16> {
    synth(0.28, |t| 820.0 - t * 640.0, Wave::Saw, 0.42, 0.03)
}

fn build_wave_clear() -> Vec<i16> {
    let mut out = Vec::new();
    out.extend(synth_segment(
        0.10,
        420.0,
        520.0,
        Wave::Triangle,
        0.24,
        0.01,
    ));
    out.extend(synth_segment(
        0.10,
        560.0,
        700.0,
        Wave::Triangle,
        0.24,
        0.01,
    ));
    out.extend(synth_segment(
        0.16,
        760.0,
        980.0,
        Wave::Pulse(0.5),
        0.28,
        0.02,
    ));
    out
}

fn build_game_over() -> Vec<i16> {
    let mut out = Vec::new();
    out.extend(synth_segment(0.18, 460.0, 320.0, Wave::Saw, 0.25, 0.02));
    out.extend(synth_segment(0.18, 320.0, 220.0, Wave::Saw, 0.25, 0.02));
    out.extend(synth_segment(0.24, 220.0, 110.0, Wave::Noise, 0.32, 0.08));
    out
}

fn build_march_low() -> Vec<i16> {
    build_march_crunch(74.0, 54.0, 0.14, 0.40)
}

fn build_march_high() -> Vec<i16> {
    build_march_crunch(86.0, 62.0, 0.12, 0.38)
}

fn build_march_crunch(
    body_start: f32,
    body_end: f32,
    body_gain: f32,
    crunch_gain: f32,
) -> Vec<i16> {
    let thud = synth_segment(
        0.032,
        body_start,
        body_end,
        Wave::Pulse(0.64),
        body_gain,
        0.22,
    );
    let crunch = synth_segment(0.045, 240.0, 60.0, Wave::Noise, crunch_gain, 0.96);
    let scrape = synth_segment(0.024, 110.0, 48.0, Wave::Noise, 0.16, 1.0);
    let click = synth_segment(0.010, 460.0, 180.0, Wave::Pulse(0.12), 0.06, 0.30);
    mix_segments(&[thud, crunch, scrape, click])
}

fn mix_segments(segments: &[Vec<i16>]) -> Vec<i16> {
    let len = segments.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = vec![0i16; len];

    for segment in segments {
        for (i, sample) in segment.iter().enumerate() {
            let mixed = out[i] as i32 + *sample as i32;
            out[i] = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        }
    }

    out
}

fn synth(duration: f32, freq: impl Fn(f32) -> f32, wave: Wave, gain: f32, noise: f32) -> Vec<i16> {
    let sample_count = (44_100.0 * duration) as usize;
    let mut phase = 0.0f32;
    let mut state = 0x1234_5678u32;
    let mut samples = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        let t = i as f32 / sample_count as f32;
        let hz = freq(t).max(40.0);
        phase = (phase + hz / 44_100.0) % 1.0;
        let env = envelope(t);
        let value = wave.sample(phase);
        let noise_value = if noise > 0.0 {
            noise * (next_noise(&mut state) * 2.0 - 1.0)
        } else {
            0.0
        };
        let sample = ((value * (1.0 - noise) + noise_value) * env * gain * i16::MAX as f32)
            .clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        samples.push(sample);
    }

    samples
}

fn synth_segment(
    duration: f32,
    start_freq: f32,
    end_freq: f32,
    wave: Wave,
    gain: f32,
    noise: f32,
) -> Vec<i16> {
    synth(
        duration,
        move |t| start_freq + (end_freq - start_freq) * t,
        wave,
        gain,
        noise,
    )
}

fn envelope(t: f32) -> f32 {
    let attack = (t / 0.06).min(1.0);
    let decay = (1.0 - t).powf(1.8);
    attack * decay
}

fn next_noise(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*state >> 8) as f32 / ((u32::MAX >> 8) as f32)
}

#[derive(Clone, Copy)]
enum Wave {
    Triangle,
    Saw,
    Pulse(f32),
    Noise,
}

impl Wave {
    fn sample(self, phase: f32) -> f32 {
        match self {
            Self::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            Self::Saw => phase * 2.0 - 1.0,
            Self::Pulse(width) => {
                if phase < width {
                    1.0
                } else {
                    -1.0
                }
            }
            Self::Noise => phase * 2.0 - 1.0,
        }
    }
}
