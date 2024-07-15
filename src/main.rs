use std::error::Error;
use std::io::{BufWriter, Cursor};
use std::path::PathBuf;
use std::str::FromStr;
use noodles::htsget as htsget;
use noodles::core::Region;
use tokio::io::AsyncWriteExt;
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use futures::TryStreamExt;
use noodles::core::region::Interval;
use tokio::fs::File;
use crypt4gh::{body_decrypt, body_decrypt_parts, WriteInfo};

use crypt4gh::header::{deconstruct_header_info, DecryptedHeaderPackets, HeaderInfo};
use crypt4gh::header::deconstruct_header_body;
use crypt4gh::{Keys, keys};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let base_url = "http://localhost:8080/".parse()?;
    let client = htsget::Client::new(base_url);

    let mut reader = BufReader::new(File::open("../htsget-rs/data/crypt4gh/htsnexus_test_NA12878.bam.c4gh").await.unwrap());

    let header = get_unencrypted_header(&mut reader).await;

    dbg!(&header);

    let recipient_private_key = keys::get_private_key(&PathBuf::from("../htsget-rs/data/crypt4gh/keys/bob.sec".to_string()), || { Ok("".to_string()) }).unwrap();
    let sender_public_key = keys::get_public_key(&PathBuf::from("../htsget-rs/data/crypt4gh/keys/alice.pub".to_string())).unwrap();

    let keys = vec![Keys {method: 0, privkey: recipient_private_key, recipient_pubkey: vec![] }];

    let header_body = get_encrypted_header(&mut reader, header, keys.as_slice(), &Some(sender_public_key)).await;

    dbg!(&header_body.data_enc_packets);
    dbg!(&header_body.edit_list_packet);

    let enc_body = get_encrypted_body(&mut reader, header_body, 0, Some(8000)).await;

    dbg!(enc_body);

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

async fn get_encrypted_header(reader: &mut BufReader<File>, header: HeaderInfo, keys: &[Keys], sender_pubkey: &Option<Vec<u8>>) -> DecryptedHeaderPackets {
    let mut encrypted_data = vec![];
    for _ in 0..header.packets_count {
        let mut length_buffer = [0; 4];
        reader
            .read_exact(&mut length_buffer).await.unwrap();

        let length = bincode::deserialize::<u32>(&length_buffer).unwrap();
        let length = length - 4;

        // Get data
        let mut data: Vec<u8> = vec![0; length as usize];
        reader.read_exact(&mut data).await.unwrap();

        encrypted_data.push(data);
    }

    deconstruct_header_body(encrypted_data, keys, sender_pubkey).unwrap()
}

async fn get_encrypted_body(reader: &mut BufReader<File>, header_packets: DecryptedHeaderPackets, range_start: usize, range_span: Option<usize>) -> Vec<u8> {
    let mut write_buffer = BufWriter::new(Vec::new());
    let mut write_info = WriteInfo::new(range_start, range_span, &mut write_buffer);

    let mut read_buffer = vec![];
    let _ = reader.read_to_end(&mut read_buffer).await.unwrap();

    let reader = std::io::BufReader::new(Cursor::new(read_buffer));

    let data_enc_packets = header_packets.data_enc_packets;
    match header_packets.edit_list_packet {
        None => body_decrypt(reader, &data_enc_packets, &mut write_info, range_start).unwrap(),
        Some(edit_list_content) => body_decrypt_parts(reader, data_enc_packets, write_info, edit_list_content).unwrap(),
    }

    write_buffer.buffer().to_vec()
}