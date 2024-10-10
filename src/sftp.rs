use crate::ao3::common::DownloadFormat;
use crate::ao3::series::Series;
use crate::ao3::work::Work;
use crate::config::{Config, Device};

use indicatif::{ProgressBar, ProgressStyle};
use ssh2::{Session, Sftp};
use std::path::Path;
use std::{
    cmp::min,
    fs::File,
    io::{Read, Write},
    net::TcpStream,
};

pub fn upload_file(
    work: &Work,
    device: &Device,
    config: &Config,
    download_format: DownloadFormat,
    existing_sftp: Option<&Sftp>,
    series_id: Option<&String>,
) {
    let using_existing_connection = existing_sftp.is_some();

    let sftp = if using_existing_connection {
        existing_sftp.unwrap()
    } else {
        &create_sftp_connection(device)
    };

    let filename = work.get_filename(download_format, series_id);
    let file_path = if series_id != None {
        Path::new(&config.download_path)
            .join(
                &work
                    .get_series_link(series_id.unwrap())
                    .unwrap()
                    .series_name,
            )
            .join(&filename)
    } else {
        Path::new(&config.download_path).join(&filename)
    };

    let mut file = File::open(file_path).unwrap();
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents).unwrap();

    println!();
    println!("Starting to upload file: {}", &filename);
    let file_length = file_contents.len();
    println!("file is {} bytes", file_length);

    let remote_download_folder = Path::new(&device.download_folder);
    let remote_file_path = if series_id != None {
        remote_download_folder
            .join(&work.filtered_fandom)
            .join(
                &work
                    .get_series_link(series_id.unwrap())
                    .unwrap()
                    .series_name,
            )
            .join(&filename)
    } else {
        remote_download_folder
            .join(&work.filtered_fandom)
            .join(&filename)
    };

    if !using_existing_connection {
        create_missing_folders_on_remote(sftp, &remote_file_path, remote_download_folder);
    }

    let mut remote_file = sftp.create(Path::new(&remote_file_path)).unwrap();

    let chunk_size = 20000;
    let mut counter = 1;

    let pb = ProgressBar::new(file_length.try_into().unwrap());
    pb.set_style(
        ProgressStyle::with_template(
            "{msg} {spinner:.green} [{elapsed_precise}] [{bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})"
        )
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

pub fn upload_series(
    series: &Series,
    device: &Device,
    config: &Config,
    download_format: DownloadFormat,
) {
    let sftp = create_sftp_connection(device);

    let remote_series_folder = Path::new(&device.download_folder)
        .join(&series.filtered_fandom)
        .join(&series.title);

    create_missing_folders_on_remote(
        &sftp,
        &remote_series_folder,
        &Path::new(&device.download_folder),
    );

    for work in &series.works {
        upload_file(
            &work,
            device,
            config,
            download_format,
            Some(&sftp),
            Some(&series.id),
        );
    }
}

fn create_missing_folders_on_remote(
    sftp: &Sftp,
    path_to_create: &Path,
    remote_download_folder: &Path,
) {
    let remote_download_folder_num_ancestors = remote_download_folder.ancestors().count();
    let remote_file_ancestors = path_to_create.ancestors().collect::<Vec<&Path>>();
    let remote_file_iterator = remote_file_ancestors
        .iter()
        .rev()
        .skip(remote_download_folder_num_ancestors);

    for path in remote_file_iterator {
        if sftp.lstat(path).is_err() {
            let _ = sftp.mkdir(path, 0); // TODO handle this error
        } else {
            break;
        }
    }
}

pub fn create_sftp_connection(device: &Device) -> Sftp {
    let tcp = TcpStream::connect((device.ip.clone(), device.port)).unwrap();
    let mut session = Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    session
        .userauth_password(&device.username, &device.password)
        .unwrap();
    session.set_blocking(true);
    session.sftp().unwrap()
}
