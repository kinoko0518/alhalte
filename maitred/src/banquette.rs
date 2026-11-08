mod events;
mod log;
mod permission;
mod state;

use std::collections::HashMap;

use crate::{
    UserID,
    banquette::{log::LogWriter, state::State},
};

pub use events::{Events, Read, Write};
pub use permission::{Permission, PermissionMask};

const BANQUETTE_ROOT: &str = "/var/lib/alhalte/banquette";

type SceneID = u32;

struct Scene {
    writer: LogWriter,
    state: State,
}

struct Banquette {
    default_permission: Permission,
    permissions: HashMap<UserID, Permission>,
    scenes: HashMap<SceneID, Scene>,
}

impl Banquette {
    pub fn get_permission(&self, user: UserID) -> Permission {
        self.permissions
            .get(&user)
            .cloned()
            .unwrap_or(self.default_permission)
    }
}
