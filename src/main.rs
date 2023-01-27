// Used for testing.

use std::{sync::atomic::{AtomicUsize, Ordering}, time::Instant};

use dir_walkin::{Result, walk_dir_concurrently};


static POS: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
pub async fn main() -> Result<()> {
    let now = Instant::now();

    let _ = walk_dir_concurrently(
        "F:/omg",
        1,
        |entry| async move {
            let pos = POS.fetch_add(1, Ordering::Relaxed);

            let _ = entry.metadata().await;

            if pos > 100_000 {
                Err(Error::Invalid)
            } else {
                std::result::Result::<_, Error>::Ok(())
            }

        }
    ).await?;

    println!("{:?}", now.elapsed());

    Ok(())
}


enum Error {
    Invalid,
}