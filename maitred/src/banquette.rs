mod events;

use std::{collections::HashMap, path::Path};

use crate::UserID;

pub use events::{Events, Read, Write};
use tokio::{
    fs::{File, OpenOptions, rename},
    io::AsyncWriteExt,
};
use toml::Value;

type SceneID = u32;
type Head = u32;

const CHUNK_SIZE: u32 = 128;
const BANQUETTE_ROOT: &str = "/var/lib/alhalte/banquette";

struct LogWriter {
    head: Head,
}

impl LogWriter {
    pub async fn refrigerate(
        &self,
        scene_path: &Path,
        hotchunk: &Path,
    ) -> Result<Head, std::io::Error> {
        let nth = self.head >> 16;

        rename(
            &hotchunk,
            scene_path.join(format!("chunks/{:016x}.log", nth)),
        )
        .await?;

        Ok((self.head & 0xffff0000) + (1 << 16))
    }

    pub async fn append(
        &mut self,
        scene_path: &Path,
        message: &[u8],
    ) -> Result<(), std::io::Error> {
        let hotchunk = scene_path.join("hotchunk.log");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&hotchunk)
            .await?;

        file.write_all(message).await?;
        file.write_u8(b'\n').await?;

        self.head = if (self.head & 0xffff) < CHUNK_SIZE {
            self.head + 1
        } else {
            self.refrigerate(scene_path, &hotchunk).await?
        };

        Ok(())
    }
}

struct State(toml::Value);

impl State {
    pub fn update(&mut self, diff: Value) {
        if let (Some(now), Value::Table(diff)) = (self.0.as_table_mut(), diff) {
            for (k, v) in diff {
                now.insert(k, v);
            }
        }
    }

    pub async fn write(&mut self, scene_path: &Path) -> Result<(), std::io::Error> {
        let tmp = scene_path.join("state.toml.tmp");
        let mut file = File::create(&tmp).await?;

        file.write_all(self.0.to_string().as_bytes()).await?;
        rename(tmp, scene_path.join("state.toml")).await?;

        Ok(())
    }
}

struct Scene {
    writer: LogWriter,
    state: State,
}

#[derive(Clone, Copy)]
enum Permission {
    Deny,
    Spectator,
    Player,
    GameMaster,
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
