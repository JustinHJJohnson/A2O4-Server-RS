use crate::ao3::common::DownloadFormat;
use crate::ao3::series::Series;
use crate::ao3::work::Work;
use crate::config::{Config, Device};

use ssh2::{Session, Sftp};
use std::path::Path;
use std::{
    fs::File,
    io::{Read, Write},
    net::TcpStream,
};

pub async fn upload_work(
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
    let file_path = if let Some(unwrapped_series_id) = series_id {
        Path::new(&config.download_path)
            .join(
                &work
                    .get_series_link(unwrapped_series_id)
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
    
    println!("Starting to upload file: {}", &filename);
    let file_length = file_contents.len();
    println!("file is {} bytes", file_length);

    let remote_download_folder = Path::new(&device.download_folder);
    let remote_file_path = if let Some(unwrapped_series_id) = series_id {
        remote_download_folder
            .join(&work.filtered_fandom)
            .join(
                &work
                    .get_series_link(unwrapped_series_id)
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
        create_missing_folders_on_remote(
            sftp,
            remote_file_path.parent().unwrap(),
            remote_download_folder,
        );
    }
    
    //TODO handle if remote path doesn't exist
    let mut remote_file = sftp.create(Path::new(&remote_file_path)).unwrap();

    let chunk_size = 15000;

    for chunk in file_contents.chunks(chunk_size).enumerate() {
        remote_file.write_all(chunk.1).unwrap();
    }
}

pub async fn upload_series(
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
        Path::new(&device.download_folder),
    );

    for work in &series.works {
        upload_work(
            work,
            device,
            config,
            download_format,
            Some(&sftp),
            Some(&series.id),
        ).await
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
            sftp.mkdir(path, 0o777).unwrap(); // TODO handle this error and maybe set more restrictive permissions
        }
    }
}

pub fn create_sftp_connection(device: &Device) -> Sftp {
    //TODO proper error handling here if host is unreachable
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
