use std::f32::consts::PI;

pub fn sine_oscillator(time: f32) -> f32 {
    (2.0 * PI * time).sin()
}

pub fn square_oscillator(time: f32) -> f32 {
    if time % 1.0 < 0.5 {
        1.0
    } else {
        -1.0
    }
}

pub fn triangle_oscillator(time: f32) -> f32 {
    if time % 1.0 < 0.5 {
        1.0 - 4.0 * (time % 1.0 - 0.25).abs()
    } else {
        4.0 * ((time - 0.5) % 1.0 - 0.25).abs() - 1.0
    }
}

pub fn sawtooth_oscillator(time: f32) -> f32 {
    ((time - 0.5) % 1.0) * 2.0 - 1.0
}