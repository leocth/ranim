pub mod creation;

pub trait Animation {}

pub struct AnimationSettings {
    run_time: f32,
    rate_func: fn(f32) -> f32, // TODO: replace with Fn/FnMut?

}