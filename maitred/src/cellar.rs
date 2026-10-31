use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use canner::{Tin, TinID, WrappedKey};
use sqlx::{SqlitePool, query, query_as};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{RoomID, SessionID, UserID};

enum CellarEvent {
    Upload(Tin, UserID),
    Download(RoomID, TinID),
    Bind(RoomID, TinID, Option<WrappedKey>),
    Unbind(RoomID, TinID),
}

enum CellarResponce {
    SuccessfullyUploaded,
    SuccessfullyDownloaded(Tin, Option<WrappedKey>),
    SuccessfullyBinded,
    SuccessfullyUnbinded,
}

enum CellarError {
    IoError(std::io::Error),
    SqlError(sqlx::Error),
    CannerError(canner::CannerError),
    PermissionDenied,
    NotFound,
}

struct CellarOrder {
    from: SessionID,
    event: CellarEvent,
    sender: oneshot::Sender<Result<CellarResponce, CellarError>>,
}

type CellarPool = SqlitePool;
type CellarResult = Result<CellarResponce, CellarError>;

#[derive(sqlx::FromRow)]
struct Ownerships {
    owner: Uuid,
    tin_id: TinID,
    timestamp: i64,
}

#[derive()]
struct Binding {
    room: RoomID,
    tin_id: TinID,
    wkey: Option<WrappedKey>,
}

pub async fn setup_cellar_db(pool: &CellarPool) -> Result<(), sqlx::Error> {
    // 所有権テーブルを作成
    query(
        "CREATE TABLE IF NOT EXISTS ownerships (
        	owner UUID NOT NULL,
        	tin_id BLOB NOT NULL,
        	timestamp INTEGER NOT NULL,
        	size INTEGER NOT NULL,
        	PRIMARY KEY (owner, tin_id),
        )",
    )
    .execute(pool)
    .await?;
    // 束縛テーブルを作成
    query(
        "CREATE TABLE IF NOT EXISTS wkeys (
            room UUID NOT NULL,
            tin_id BLOB NOT NULL,
            wkey BLOB,
            PRIMARY KEY (room, tin_id),
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn cellar(mut rx: mpsc::Receiver<CellarOrder>, pool: &CellarPool, save_path: &Path) {
    while let Some(order) = rx.recv().await {
        let _ = order.sender.send(match order.event {
            CellarEvent::Upload(tin, user) => upload(user, tin, save_path, pool).await,
            CellarEvent::Download(room, id) => download(room, id, save_path, pool).await,
            CellarEvent::Bind(room, tin_id, wkey) => bind(room, &tin_id, &wkey, pool).await,
            CellarEvent::Unbind(room, tin_id) => unbind(room, &tin_id, pool).await,
        });
    }
}

async fn bind(
    room: RoomID,
    tin_id: &TinID,
    wkey: &Option<WrappedKey>,
    pool: &CellarPool,
) -> CellarResult {
    query("INSERT INTO wkeys (room, tin_id, wkey)　VALUES ($1, $2, $3)")
        .bind(room)
        .bind(&tin_id[..])
        .bind(wkey.as_deref())
        .execute(pool)
        .await
        .map_err(CellarError::SqlError);

    Ok(CellarResponce::SuccessfullyBinded)
}

pub async fn unbind(room: UserID, tin_id: &TinID, pool: &CellarPool) -> CellarResult {
    query("DELETE FROM wkeys WHERE room = $1 AND tin_id = $2")
        .bind(room)
        .bind(&tin_id[..])
        .execute(pool)
        .await
        .map_err(CellarError::SqlError);

    Ok(CellarResponce::SuccessfullyUnbinded)
}

pub async fn download(
    room: RoomID,
    tin_id: TinID,
    save_path: &Path,
    pool: &CellarPool,
) -> CellarResult {
    let wrapped_key = query_as("SELECT wkey FROM wkeys WHERE room = $1 AND tin_id = $2")
        .bind(room)
        .bind(&tin_id[..])
        .fetch_optional(pool)
        .await
        .map_err(CellarError::SqlError)?
        .ok_or(CellarError::NotFound)?;

    let tin = tokio::fs::read(save_path)
        .await
        .map_err(CellarError::IoError)
        .and_then(|bin| Tin::from_bytes(bin).map_err(CellarError::CannerError))?;

    Ok(CellarResponce::SuccessfullyDownloaded(tin, wrapped_key))
}

pub async fn upload(user: UserID, tin: Tin, save_path: &Path, pool: &CellarPool) -> CellarResult {
    let tin_id = tin.get_id();
    let path = save_path.join(format!("{}.tin", hex::encode(tin_id)));
    let bytes = tin.into_bytes();
    let size = bytes.len();

    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| CellarError::IoError(e))?;

    query(
        "INSERT INTO ownerships (
            owner, tin_id, timestamp, size
        )　VALUES ($1, $2, $3, $4)",
    )
    .bind(user)
    .bind(&tin_id[..])
    .bind({
        let unix_time: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("How can it exits before unix epoch!?")
            .as_secs();
        unix_time as i64
    })
    .bind(size as i32)
    .execute(pool)
    .await
    .map_err(CellarError::SqlError);

    Ok(CellarResponce::SuccessfullyUploaded)
}
