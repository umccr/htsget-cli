use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use noodles::htsget as htsget;
use noodles::core::{Position, Region};
use tokio::io::AsyncWriteExt;
use tokio::io;
use futures::TryStreamExt;
use noodles::core::region::Interval;

use crypt4gh::header::{deconstruct_header_info, HeaderInfo};
use crypt4gh::header::deconstruct_header_body;
use reqwest::Body;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_url = "http://localhost:8080/".parse()?;
    let client = htsget::Client::new(base_url);

    dbg!(get_unencrypted_header("../htsget-rs/data/crypt4gh/htsnexus_test_NA12878.bam.c4gh".to_string()));
    dbg!(get_encrypted_body("../htsget-rs/data/crypt4gh/htsnexus_test_NA12878.bam.c4gh".to_string()),
                            "../htsget-rs/data/crypt4gh/keys/bob.pub");

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

fn get_unencrypted_header(file: String) -> HeaderInfo {
    let mut file = File::open(file).unwrap();

    let mut buf = [0; 16];
    file.read_exact(&mut buf).unwrap();

    deconstruct_header_info(&buf).unwrap()
}

fn get_encrypted_body(file: String) -> Body {
    let mut file = File::open(file).unwrap();

    let mut buf = [0; 16];
    file.read_exact(&mut buf).unwrap();

    deconstruct_header_body(&buf).unwrap()
}