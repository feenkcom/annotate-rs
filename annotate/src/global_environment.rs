use crate::{Environment, Function, Module};
use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::OnceCell;

pub fn global_environment() -> &'static GlobalEnvironment {
    static INSTANCE: OnceCell<GlobalEnvironment> = OnceCell::new();
    INSTANCE.get_or_init(GlobalEnvironment::new)
}

pub fn register_environment(name: &str, environment: &'static Environment) {
    global_environment().register(name, environment);
}

#[derive(Debug)]
pub struct GlobalEnvironment {
    environments: Mutex<HashMap<String, &'static Environment>>,
}

impl GlobalEnvironment {
    pub fn new() -> Self {
        Self {
            environments: Mutex::new(Default::default()),
        }
    }

    pub fn register(&self, name: &str, environment: &'static Environment) {
        self.environments
            .lock()
            .unwrap()
            .insert(name.to_string(), environment);
    }

    pub fn find_modules_such_that(&self, f: &impl Fn(&Module) -> bool) -> Vec<Module> {
        self.environments
            .lock()
            .unwrap()
            .values()
            .flat_map(|each| each.find_modules_such_that(f))
            .collect()
    }

    pub fn find_functions_such_that(&self, f: &impl Fn(&Function) -> bool) -> Vec<Function> {
        self.environments
            .lock()
            .unwrap()
            .values()
            .flat_map(|each| each.find_functions_such_that(f))
            .collect()
    }
}

impl Default for GlobalEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
