//! 执行脚本 程序主要入口

use crate::runner::TRunner;
use crate::strategy::TStrategy;
use crate::user::SUser;

pub struct SScript<R: TRunner<S> + Default, S: TStrategy> {
    pub users: Vec<SUser<S>>,
    pub runner: R,
}

impl<R: TRunner<S> + Default, S: TStrategy> SScript<R, S> {
    pub fn new(users: Vec<SUser<S>>, runner: R) -> Self {
        Self {
            users,
            runner,
        }
    }

    pub fn run(&mut self) {
        self.runner.run(&self.users);
    }
}

impl<R: TRunner<S> + Default, S: TStrategy> Default for SScript<R, S>
{
    fn default() -> Self {
        SScript {
            users: Default::default(),
            runner: R::default(),
        }
    }
}