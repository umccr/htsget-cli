use std::error::Error;
use noodles_htsget as htsget;
use tokio::io::AsyncWriteExt;
use tokio::io;
use futures::TryStreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_url = "http://localhost:8080/".parse()?;
    let client = htsget::Client::new(base_url);

    let reads = client.reads("data/bam/htsnexus_test_NA12878");
    let reads = reads.send().await?;

    let mut chunks = reads.chunks();
    let mut stdout = io::stdout();

    while let Some(chunk) = chunks.try_next().await? {
        stdout.write_all(&chunk).await?;
    }

    Ok(())
}