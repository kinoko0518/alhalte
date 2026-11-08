use std::path::Path;

use tokio::{
    fs::{OpenOptions, rename},
    io::AsyncWriteExt,
};

use crate::banquette::SceneID;

const CHUNK_SIZE: u32 = 2048;
const CHUNK_BITWIDTH: usize = size_of::<SceneID>() * 8 - CHUNK_SIZE.ilog2() as usize;

pub struct Head {
    pub scene: SceneID,
    index: u32,
}

impl Head {
    #[inline]
    pub fn scene_id(&self) -> u32 {
        self.index / CHUNK_SIZE
    }

    #[inline]
    pub fn message_id(&self) -> u32 {
        self.index % CHUNK_SIZE
    }

    pub fn map_id(&self, f: fn(u32) -> u32) -> (bool, Self) {
        let y = f(self.index);
        let overchunk = y / CHUNK_SIZE != self.index / CHUNK_SIZE;
        let newself = Self {
            scene: self.scene,
            index: y,
        };

        (overchunk, newself)
    }
}

pub struct LogWriter {
    head: Head,
}

impl LogWriter {
    pub async fn refrigerate(
        &self,
        scene_path: &Path,
        hotchunk: &Path,
    ) -> Result<(), std::io::Error> {
        const HEX_WIDTH: usize = CHUNK_BITWIDTH / 4;
        rename(
            &hotchunk,
            scene_path.join(format!("chunks/{:0HEX_WIDTH$x}.log", self.head.scene_id())),
        )
        .await?;

        Ok(())
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
        file.write_all(b"\n\n").await?;

        let (overchunk, newhead) = self.head.map_id(|x| x + 1);
        self.head = newhead;

        if overchunk {
            self.refrigerate(scene_path, &hotchunk).await?
        }

        Ok(())
    }
}
