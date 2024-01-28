use cpal::traits::{DeviceTrait, StreamTrait};
use parking_lot::{Mutex, RwLock};

use crate::song::Song;
use std::sync::Arc;

fn build_stream_impl<'a, Sample>(
    device: Arc<cpal::Device>,
    config: Arc<cpal::StreamConfig>,
    mut gen_sample: impl FnMut() -> f32 + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    Sample: cpal::SizedSample + 'static + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;

    device.build_output_stream(
        &config,
        move |data: &mut [Sample], _| {
            for frame in data.chunks_mut(channels) {
                frame.fill(Sample::from_sample(gen_sample()));
            }
        },
        |err| eprintln!("{err}"),
        None,
    )
}

pub struct AudioPlayer<S: Song> {
    current_time: Arc<RwLock<f32>>,
    _stream: cpal::Stream,
    song: Arc<Mutex<S>>,
}

impl<S: Song> AudioPlayer<S> {
    pub fn new(
        song: Arc<Mutex<S>>,
        device: Arc<cpal::Device>,
        sample_format: cpal::SampleFormat,
        config: Arc<cpal::StreamConfig>,
    ) -> anyhow::Result<Self> {
        let current_time = Arc::new(RwLock::new(0.0));

        let inner_current_time = current_time.clone();

        let inner_song = song.clone();

        let sample_rate_float = config.sample_rate.0 as f32;
        let gen_sample = move || {
            let mut time = inner_current_time.write();
            *time += 1.0 / sample_rate_float as f32;
            (*inner_song).lock().amp_at(*time)
        };
        let stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream_impl::<f32>(device, config, gen_sample),
            cpal::SampleFormat::F64 => build_stream_impl::<f64>(device, config, gen_sample),
            cpal::SampleFormat::U8 => build_stream_impl::<u8>(device, config, gen_sample),
            cpal::SampleFormat::U16 => build_stream_impl::<u16>(device, config, gen_sample),
            cpal::SampleFormat::U32 => build_stream_impl::<u32>(device, config, gen_sample),
            cpal::SampleFormat::U64 => build_stream_impl::<u64>(device, config, gen_sample),
            cpal::SampleFormat::I8 => build_stream_impl::<i8>(device, config, gen_sample),
            cpal::SampleFormat::I16 => build_stream_impl::<i16>(device, config, gen_sample),
            cpal::SampleFormat::I32 => build_stream_impl::<i32>(device, config, gen_sample),
            cpal::SampleFormat::I64 => build_stream_impl::<i64>(device, config, gen_sample),
            _ => unreachable!(),
        }?;
        stream.play()?;

        Ok(Self {
            current_time,
            _stream: stream,
            song,
        })
    }

    pub fn get_data(
        &mut self,
        start: f32,
        end: f32,
        sample_length: f32,
    ) -> Option<impl Iterator<Item = (f32, f32)> + '_> {
        if start < 0.0 || end < start {
            None
        } else {
            Some(
                (0..=((end - start) / sample_length).ceil() as usize).map(move |i| {
                    (
                        start + i as f32 * sample_length,
                        self.song.lock().amp_at(start + i as f32 * sample_length),
                    )
                }),
            )
        }
    }

    pub fn get_time(&self) -> f32 {
        *self.current_time.read()
    }

    pub fn set_time(&mut self, time: f32) {
        *self.current_time.write() = time.max(0.0);
    }
}