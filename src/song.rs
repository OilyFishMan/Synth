
pub trait Song: Send + Sync + 'static {
    fn amp_at(&self, time: f32) -> f32;
}
