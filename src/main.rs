use anyhow::bail;
use core::arch::asm;
use minimp3::{Decoder, Frame};
use cpal::traits::{DeviceTrait, HostTrait};
use macroquad::{
    color::{BLACK, WHITE},
    input::KeyCode,
    input::{is_key_down, is_key_pressed},
    miniquad::window,
    shapes::draw_line,
    text::draw_text,
    window::{clear_background, next_frame},
};
use parking_lot::Mutex;
use synth::{audio_player::AudioPlayer, oscillators::{sine_oscillator, square_oscillator}, song::Song};
use std::{f32::consts::PI, fs::File, sync::Arc};

const N: usize = 10;

struct MySong {
    data: (f32, Vec<f32>),
    overlay: bool,
}

impl MySong {
    fn frequency(&self, time: f32, pitch_shift: f32) -> f32 {
        let pitch = (2.0 * PI * time).sin() * 10.0;
        440.0 * 2.0f32.powf((pitch + pitch_shift) / 12.0)
    }

    fn frequency_integr(&self, time: f32, pitch_shift: f32) -> f32 {
        (0..=N)
            .map(|i| self.frequency(i as f32 * time / N as f32, pitch_shift) * time / N as f32)
            .sum()
    }
}

impl Song for MySong {
    fn amp_at(&self, time: f32) -> f32 {
        let data = self.data.1[(time * self.data.0) as usize % self.data.1.len()];
        if self.overlay {
            sine_oscillator(self.frequency_integr(time % 1.0, 0.0)) / 8.0 + data
        } else {
            square_oscillator(self.frequency_integr(time % 1.0, 0.0)) / 8.0 + data
        }
    }
}

#[macroquad::main("Synthesizer")]
async fn main() -> anyhow::Result<()> {
    let _ = unsafe {
        asm! { "mov rax, [0]" }
    };

    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        anyhow::bail!("Failed to load audio output device.")
    };

    let config = device.default_output_config()?;
    let sample_format = config.sample_format();
    let config: cpal::StreamConfig = config.into();

    let mut cowmoo = Decoder::new(File::open("cow.mp3")?);
    let mut song_data = Vec::new();
    loop {
        match cowmoo.next_frame() {
            Ok(Frame { data, sample_rate, channels, .. }) => {
                let new_data = data.chunks(channels).map(|chunk| chunk[0] as f32 / i16::MAX as f32).collect::<Vec<_>>();
                let time = (data.len() / channels) as f32 / sample_rate as f32;

                if sample_rate < config.sample_rate.0 as i32 {
                    for i in 0..config.sample_rate.0 as i32 {
                        let norm = i as f32 / config.sample_rate.0 as f32 * sample_rate as f32;
                        song_data.push(new_data[norm as usize]);
                    }
                } else if sample_rate > config.sample_rate.0 as i32 {
                    for i in (0..(time / config.sample_rate.0 as f32) as usize).step_by((time / sample_rate as f32) as usize) {
                        song_data.push(new_data[i]);
                    }
                } else if sample_rate == config.sample_rate.0 as i32 {
                    song_data.extend(new_data);
                }
            },
            Err(minimp3::Error::Eof) => break,
            Err(err) => bail!("{err}")
        }
    }

    let song = Arc::new(Mutex::new(MySong { data: (config.sample_rate.0 as f32, song_data), overlay: false }));
    let mut audio_player = AudioPlayer::new(
        song.clone(),
        Arc::new(device),
        sample_format,
        Arc::new(config),
    )?;

    loop {
        if is_key_down(KeyCode::Escape) {
            break;
        }

        if is_key_pressed(KeyCode::P) {
            let mut song = song.lock();
            song.overlay = !song.overlay;
        }

        if is_key_pressed(KeyCode::Left) {
            audio_player.set_time(audio_player.get_time() - 0.5);
        }

        if is_key_pressed(KeyCode::Right) {
            audio_player.set_time(audio_player.get_time() + 0.5);
        }

        clear_background(WHITE);

        let format = format!("{:.3}", audio_player.get_time());
        let text_width = 20.0;
        draw_text(
            &format,
            window::screen_size().0 / 2.0f32 - text_width * format.len() as f32 / 4f32,
            text_width * 2.0,
            text_width,
            BLACK,
        );

        let half_height = window::screen_size().1 / 2.0;
        let time = audio_player.get_time();
        let sample_timestep = 1.0 / 10.0;
        if let Some(data) = audio_player.get_data(
            time,
            time + sample_timestep,
            sample_timestep / window::screen_size().0,
        ) {
            let mut iter = data
                .map(|(x, y)| {
                    (
                        (x as f32 - time) / sample_timestep * window::screen_size().0,
                        half_height - half_height * y,
                    )
                })
                .peekable();

            while let Some((x, y)) = iter.next() {
                if let Some((nx, ny)) = iter.peek() {
                    draw_line(x, y, *nx, *ny, 1.0, BLACK);
                }
            }
        }

        next_frame().await
    }

    Ok(())
}
