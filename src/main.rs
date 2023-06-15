use std::error::Error;
use std::str::FromStr;
use noodles::htsget as htsget;
use noodles::core::{Position, Region};
use tokio::io::AsyncWriteExt;
use tokio::io;
use futures::TryStreamExt;
use noodles::core::region::Interval;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_url = "http://localhost:8080/".parse()?;
    let client = htsget::Client::new(base_url);

    let reads = client.reads("data/crypt4gh/htsnexus_test_NA12878").add_region(Region::new(
        "11",
        Interval::from_str("4999976-5003981").unwrap()
    ));
    let reads = reads.send().await?;

    let mut chunks = reads.chunks();
    let mut stdout = io::stdout();

    while let Some(chunk) = chunks.try_next().await? {
        stdout.write_all(&chunk).await?;
    }

    Ok(())
}