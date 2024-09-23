use std::{cmp::min, fs::File, io::{Read, Write}, net::TcpStream};
use ssh2::Session;
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};

pub fn upload_file() {
    let tcp = TcpStream::connect("192.168.2.6:2222").unwrap();
    let mut session = Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    session.userauth_password("root", "").unwrap();

    let sftp = session.sftp().unwrap();

    let mut test_file = File::open("downloads/Hey Jude.epub").unwrap();
    let mut file_contents = Vec::new();
    test_file.read_to_end(&mut file_contents).unwrap();
    

    println!("Starting to upload file");
    let file_length = file_contents.len();
    println!("file is {} bytes", file_length);

    let mut remote_file = sftp.create(Path::new("/mnt/us/documents/Hey Jude.epub")).unwrap();

    let chunk_size = 24461;
    //let num_chunks = file_length / chunk_size;
    let mut counter = 1;

    let pb = ProgressBar::new(file_length.try_into().unwrap());
    pb.set_style(ProgressStyle::with_template("{msg} {spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("##-"));

    for chunk in file_contents.chunks(chunk_size) {
        remote_file.write_all(chunk).unwrap();
        counter += 1;
        pb.set_position(min(counter * chunk_size, file_length).try_into().unwrap());
    }

    pb.finish_with_message("Finished writing file");
}
