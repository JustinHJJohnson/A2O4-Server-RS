use crate::config::{Config, Device};
use crate::ao3::work::Work;
use crate::ao3::common::{self, DownloadFormat};

use std::{cmp::min, fs::File, io::{Read, Write}, net::TcpStream};
use ssh2::Session;
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};

pub fn upload_file(work: &Work, device: &Device, config: &Config, series_id: Option<&String>) {
    let tcp = TcpStream::connect((device.ip.clone(), device.port)).unwrap();
    let mut session = Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    session.userauth_password(&device.username, &device.password).unwrap();
    session.set_blocking(true);

    let sftp = session.sftp().unwrap();

    let filename = work.get_filename(DownloadFormat::EPUB, series_id);
    let file_path = Path::new(&config.download_path).join(&filename);

    let mut file = File::open(file_path).unwrap();
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents).unwrap();
    

    println!("Starting to upload file");
    let file_length = file_contents.len();
    println!("file is {} bytes", file_length);

    let fandom_folder = common::filter_fandoms(&work.fandoms(), config);

    let remote_download_folder = Path::new(&device.download_folder);
    let remote_file_path = if series_id != None {
        remote_download_folder.join(fandom_folder).join(&work.get_series_link(series_id.unwrap()).unwrap().series_name).join(&filename)
    } else {
        remote_download_folder.join(fandom_folder).join(&filename)
    };
    

    let remote_download_folder_num_ancestors = remote_download_folder.ancestors().count();
    let remote_file_ancestors = remote_file_path.ancestors().skip(1).collect::<Vec<&Path>>();
    let remote_file_iterator = remote_file_ancestors.iter().rev().skip(remote_download_folder_num_ancestors);

    for path in remote_file_iterator {
        if sftp.lstat(path).is_err() {
            let _ = sftp.mkdir(path, 0); //TODO handle this error
        }
    }

    let mut remote_file = sftp.create(Path::new(&remote_file_path)).unwrap();

    let chunk_size = 20000;
    let mut counter = 1;

    let pb = ProgressBar::new(file_length.try_into().unwrap());
    pb.set_style(
        ProgressStyle::with_template("{msg} {spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("##-")
    );

    for chunk in file_contents.chunks(chunk_size) {
        remote_file.write_all(chunk).unwrap();
        counter += 1;
        pb.set_position(min(counter * chunk_size, file_length).try_into().unwrap());
    }

    pb.finish_with_message("Finished writing file");
}
