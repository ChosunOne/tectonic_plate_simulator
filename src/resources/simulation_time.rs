use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

use bevy::ecs::resource::Resource;

use crate::constants::{BASE_DT, MAX_SPEED, MIN_SPEED};

#[derive(Resource, Clone)]
pub struct SimulationTime(Arc<SimulationTimeInner>);

struct SimulationTimeInner {
    paused: AtomicBool,
    step_flag: AtomicBool,
    speed: AtomicU32,
}

impl SimulationTime {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn paused(&self) -> bool {
        self.0.paused.load(Ordering::Acquire)
    }

    pub fn pause(&self) {
        self.0.paused.store(true, Ordering::Release);
    }

    pub fn unpause(&self) {
        self.0.paused.store(false, Ordering::Release);
    }

    pub fn toggle_pause(&self) {
        let current = self.0.paused.load(Ordering::Acquire);
        self.0.paused.store(!current, Ordering::Release);
    }

    pub fn step(&self) {
        self.0.step_flag.store(true, Ordering::Release);
    }

    pub fn clear_step(&self) -> bool {
        self.0.step_flag.swap(false, Ordering::AcqRel)
    }

    #[must_use]
    pub fn step_flag(&self) -> bool {
        self.0.step_flag.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn speed(&self) -> f32 {
        f32::from_bits(self.0.speed.load(Ordering::Acquire))
    }

    pub fn set_speed(&self, speed: f32) {
        let clamped = speed.clamp(MIN_SPEED, MAX_SPEED);
        self.0.speed.store(clamped.to_bits(), Ordering::Release);
    }

    pub fn double_speed(&self) {
        let current = f32::from_bits(self.0.speed.load(Ordering::Acquire));
        let clamped = (current * 2.0).min(MAX_SPEED);
        self.0.speed.store(clamped.to_bits(), Ordering::Release);
    }

    pub fn half_speed(&self) {
        let current = f32::from_bits(self.0.speed.load(Ordering::Acquire));
        let clamped = (current / 2.0).max(MIN_SPEED);
        self.0.speed.store(clamped.to_bits(), Ordering::Release);
    }

    #[must_use]
    pub fn dt(&self) -> f32 {
        let max = if self.should_run() { f32::MAX } else { 0.0 };
        (BASE_DT * self.speed()).min(max)
    }

    #[must_use]
    pub fn should_run(&self) -> bool {
        !self.paused() || self.step_flag()
    }
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self(Arc::new(SimulationTimeInner {
            paused: AtomicBool::new(true),
            step_flag: AtomicBool::new(false),
            speed: AtomicU32::new(1.0f32.to_bits()),
        }))
    }
}
