use std::error::Error;
use std::str::FromStr;
use noodles::htsget as htsget;
use noodles::core::{Position, Region};
use tokio::io::AsyncWriteExt;
use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncBufRead;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use futures::TryStreamExt;
use noodles::core::region::Interval;
use tokio::fs::File;

use crypt4gh::header::{deconstruct_header_info, DecryptedHeaderPackets, HeaderInfo};
use crypt4gh::header::deconstruct_header_body;
use crypt4gh::Keys;
use reqwest::Body;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_url = "http://localhost:8080/".parse()?;
    let client = htsget::Client::new(base_url);

    let mut reader = BufReader::new(File::open("../htsget-rs/data/crypt4gh/htsnexus_test_NA12878.bam.c4gh").await.unwrap());

    let header = get_unencrypted_header(&mut reader).await;

    dbg!(&header);

    let mut privkey_buf = vec![];
    File::open("../htsget-rs/data/crypt4gh/keys/alice.sec").await.unwrap().read_to_end(&mut privkey_buf).await.unwrap();

    let mut recipient_pubkey_buf = vec![];
    File::open("../htsget-rs/data/crypt4gh/keys/bob.pub").await.unwrap().read_to_end(&mut recipient_pubkey_buf).await.unwrap();

    let mut sender_pubkey_buf = vec![];
    File::open("../htsget-rs/data/crypt4gh/keys/alice.pub").await.unwrap().read_to_end(&mut sender_pubkey_buf).await.unwrap();


    let keys = vec![Keys {method: 0, privkey: privkey_buf, recipient_pubkey: recipient_pubkey_buf }];

    let body = get_encrypted_body(&mut reader, header, keys.as_slice(), &Some(sender_pubkey_buf)).await;

    let data_enc_packets = body.data_enc_packets;
    let edit_list_packet = body.edit_list_packet;

    dbg!(&data_enc_packets);
    dbg!(&edit_list_packet);

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

async fn get_unencrypted_header(reader: &mut BufReader<File>) -> HeaderInfo {
    let mut buf = [0; 16];
    reader.read_exact(&mut buf).await.unwrap();

    deconstruct_header_info(&buf).unwrap()
}

async fn get_encrypted_body(reader: &mut BufReader<File>, header: HeaderInfo, keys: &[Keys], sender_pubkey: &Option<Vec<u8>>) -> DecryptedHeaderPackets {
    // Get packet count from header to infer body size
    // let mut buf = [0; 16];
    // file.read_exact(&mut buf).unwrap();
    //
    //
    // let packets = header.packets_count;

    let mut length_buffer = [0; 4];
    reader
        .read_exact(&mut length_buffer).await.unwrap();

    let length = bincode::deserialize::<u32>(&length_buffer).unwrap();
    let length = length - 4;

    // Get data
    let mut encrypted_data: Vec<u8> = vec![0; length as usize];
    reader.read_exact(&mut encrypted_data).await.unwrap();

    deconstruct_header_body(vec![encrypted_data], keys, sender_pubkey).unwrap()
}