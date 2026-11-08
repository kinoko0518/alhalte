use std::path::Path;

use tokio::{
    fs::{File, rename},
    io::AsyncWriteExt,
};
use toml::Value;

pub struct State(toml::Value);

type StateHash = u32;

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
