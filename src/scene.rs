use crate::{anim::Animation, mobj::MObject};

pub struct Scene<'a> {
    animations: Vec<&'a dyn Animation>,
    mobjects: Vec<&'a dyn MObject>,
}

impl<'a> Scene<'a> {
    pub fn new() -> Self {
        Self {
            animations: vec![],
            mobjects: vec![],
        }
    }

    pub fn play<A: Animation>(&mut self, animation: &'a A) {
        self.animations.push(animation);
    }

    pub fn add<M: MObject>(&mut self, mobject: &'a M) {
        self.mobjects.push(mobject);
    }
}

impl Default for Scene<'_> {
    fn default() -> Self {
        Self::new()
    }
}
