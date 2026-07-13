use nyaterm_core::{AppRuntime, NativeServices};

#[derive(Debug)]
pub struct RuntimeStore {
    runtime: AppRuntime,
    services: NativeServices,
}

impl RuntimeStore {
    pub fn new(runtime: AppRuntime) -> Self {
        Self {
            runtime,
            services: NativeServices::new(),
        }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn services(&self) -> &NativeServices {
        &self.services
    }
}
