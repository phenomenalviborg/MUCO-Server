use core::str;
use std::{fs, net::{IpAddr, Ipv4Addr, SocketAddr}};

use bytes::Bytes;
use discoverable_service::register_msdn;
use local_ip_address::local_ip;
use warp::Filter;

const FOLDER_NAME: &str = "photos";

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

async fn handle_upload(bytes: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let end_header = find_subsequence(&bytes[..], b"\r\n\r\n").unwrap();
    let header_bytes = &bytes[0..end_header];
    let header_string = str::from_utf8(header_bytes).unwrap();
    let mut lines = header_string.lines();
    lines.next().unwrap();
    lines.next().unwrap();
    lines.next().unwrap();
    let line = lines.next().unwrap();
    let mut parts = line.split(';');
    parts.next().unwrap();
    parts.next().unwrap();
    let part = parts.next().unwrap();
    let mut quotes = part.split('"');
    quotes.next().unwrap();
    let name = quotes.next().unwrap();
    let begin_data = end_header + 4;
    let end_data = bytes.len() - 48;
    let data = &bytes[begin_data..end_data];
    let path = format!("{FOLDER_NAME}\\{name}");
    fs::write(path, data).unwrap();
    Ok(warp::reply())
}

#[tokio::main]
async fn main() {
    let port = 3030;
    let my_local_ip = local_ip().unwrap();

    let _mdns = register_msdn(my_local_ip, port, "muco-photo");

    fs::create_dir_all(FOLDER_NAME).unwrap();

    let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

    let upload_photo = warp::post()
        .and(warp::path("upload_photo"))
        .and(warp::path::end())
        .and(warp::body::bytes())
        .and_then(handle_upload);
    
    let routes = hello.or(upload_photo);
    let addr = SocketAddr::new(IpAddr::from(Ipv4Addr::UNSPECIFIED), port);

    warp::serve(routes)
        .run(addr)
        .await;
}
