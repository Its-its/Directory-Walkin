use std::{path::Path, sync::Arc};
use tokio::{sync::Mutex, fs::{DirEntry, ReadDir}, task::JoinSet};

mod error;

pub use error::*;

/// Walk directories linearly.
///
/// If an error occurs it'll propagate the main function.
pub async fn walk_dir_linear<Func, Fut, E>(
    path: impl AsRef<Path>,
    mut entry_function: Func,
) -> Result<std::result::Result<(), E>>
where
    Func: FnMut(tokio::fs::DirEntry) -> Fut,
    Fut: std::future::Future<Output = std::result::Result<(), E>>,
{
    let mut cached_dirs = vec![tokio::fs::read_dir(path).await?];

    while let Some(mut dir) = cached_dirs.pop() {
        if let Some(entry) = dir.next_entry().await? {
            cached_dirs.push(dir);

            if entry.file_type().await?.is_dir() {
                // We'll read from it in a second.
                cached_dirs.push(tokio::fs::read_dir(entry.path()).await?);
            } else if let Err(e) = entry_function(entry).await {
                return Ok(Err(e));
            }
        }
    }

    Ok(Ok(()))
}

/// Walk directories concurrently.
///
/// If an error occurs it'll propagate the main function.
pub async fn walk_dir_concurrently<Func, Fut, E>(
    path: impl AsRef<Path>,
    count: usize,
    entry_function: Func,
) -> Result<std::result::Result<(), E>>
where
    Func: Fn(tokio::fs::DirEntry) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = std::result::Result<(), E>> + Send + Sync,
    E: Send + Sync + 'static,
{
    let cached_dirs = Arc::new(Mutex::new(vec![tokio::fs::read_dir(path).await?]));
    let entry_function = Arc::new(entry_function);

    let mut join_set = JoinSet::new();

    for _ in 0..count {
        let entry_function = entry_function.clone();

        let mut cached_dirs = cached_dirs.lock().await;

        if let Some(entry) = get_next_file(&mut cached_dirs).await? {
            join_set.spawn(async move {
                entry_function(entry).await
            });
        }
    }

    while let Some(v) = join_set.join_next().await {
        if let Err(e) = v? {
            return Ok(Err(e));
        }

        let entry_function = entry_function.clone();

        let mut cached_dirs = cached_dirs.lock().await;

        if let Some(entry) = get_next_file(&mut cached_dirs).await? {
            join_set.spawn(async move {
                entry_function(entry).await
            });
        }
    }

    Ok(Ok(()))
}

/// Walk directories concurrently.
///
/// If an error occurs you can handle it through `error_function`.
///
/// If an error occurs in `error_function` it'll propagate the main function.
pub async fn walk_dir_concurrently_with_error_handler<EntryFunc, ErrorFunc, EntryFut, E>(
    path: impl AsRef<Path>,
    count: usize,
    entry_function: EntryFunc,
    error_function: ErrorFunc,
) -> Result<std::result::Result<(), E>>
where
    EntryFunc: Fn(tokio::fs::DirEntry) -> EntryFut + Send + Sync + 'static,
    ErrorFunc: Fn(E) -> std::result::Result<(), E>,
    EntryFut: std::future::Future<Output = std::result::Result<(), E>> + Send + Sync,
    E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
{
    let cached_dirs = Arc::new(Mutex::new(vec![tokio::fs::read_dir(path).await?]));
    let entry_function = Arc::new(entry_function);

    let mut join_set = JoinSet::new();

    for _ in 0..count {
        let entry_function = entry_function.clone();

        let mut cached_dirs = cached_dirs.lock().await;

        if let Some(entry) = get_next_file(&mut cached_dirs).await? {
            join_set.spawn(async move {
                entry_function(entry).await
            });
        }
    }

    while let Some(v) = join_set.join_next().await {
        if let Err(e) = v? {
            if let Err(e) = error_function(e) {
                return Ok(Err(e));
            }
        }

        let entry_function = entry_function.clone();

        let mut cached_dirs = cached_dirs.lock().await;

        if let Some(entry) = get_next_file(&mut cached_dirs).await? {
            join_set.spawn(async move {
                entry_function(entry).await
            });
        }
    }

    Ok(Ok(()))
}



async fn get_next_file(cached_dirs: &mut Vec<ReadDir>) -> Result<Option<DirEntry>> {
    while let Some(mut dir) = cached_dirs.pop() {
        if let Some(entry) = dir.next_entry().await? {
            cached_dirs.push(dir);

            if entry.file_type().await?.is_dir() {
                // We'll read from it in a second.
                cached_dirs.push(tokio::fs::read_dir(entry.path()).await?);
            } else {
                return Ok(Some(entry));
            }
        }
    }

    Ok(None)
}