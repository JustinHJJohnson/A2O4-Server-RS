use crate::ao3::common::DownloadFormat;
use crate::ao3::series::Series;
use crate::ao3::work::Work;
use crate::config::{Config, Device};

use anyhow::{Context, Result};
use ssh2::{Session, Sftp};
use std::path::{Path, PathBuf};
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
    series: Option<&Series>,
) -> Result<()> {
    let using_existing_connection = existing_sftp.is_some();

    let sftp = if using_existing_connection {
        existing_sftp.unwrap()
    } else {
        &create_sftp_connection(device)?
    };

    let filename = work.get_filename(download_format, series.map(|x| &x.id));
    let file_path = if let Some(unwrapped_series) = series {
        Path::new(&config.download_path)
            .join(unwrapped_series.title.clone())
            .join(&filename)
    } else {
        Path::new(&config.download_path).join(&filename)
    };

    let mut file = File::open(&file_path).with_context(|| {
        format!(
            "Failed to open file {} for work {}",
            file_path.display(),
            work.title
        )
    })?;
    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents).with_context(|| {
        format!(
            "Failed to read file {} for work {}",
            file_path.display(),
            work.title
        )
    })?;

    println!("Starting to upload file: {}", &filename);
    let file_length = file_contents.len();
    println!("file is {file_length} bytes");

    let remote_download_folder = Path::new(&device.download_folder);
    let mut remote_file_path = PathBuf::from(remote_download_folder);
    remote_file_path.push(if let Some(unwrapped_series) = series {
        unwrapped_series.filtered_fandom.clone()
    } else {
        work.filtered_fandom.clone()
    });
    if work.filtered_fandom == "Original Work" {
        remote_file_path.push(work.author.clone());
    }
    if let Some(unwrapped_series) = series {
        remote_file_path.push(unwrapped_series.title.clone());
    }
    remote_file_path.push(filename);

    if !using_existing_connection {
        create_missing_folders_on_remote(
            sftp,
            remote_file_path.parent().unwrap(),
            remote_download_folder,
        )?;
    }

    let mut remote_file = sftp.create(Path::new(&remote_file_path)).with_context(|| {
        format!(
            "Failed to create remote file {} for work {}",
            &remote_file_path.to_str().unwrap(),
            work.title
        )
    })?;

    let chunk_size = 15000;

    for chunk in file_contents.chunks(chunk_size).enumerate() {
        remote_file.write_all(chunk.1).with_context(|| {
            format!(
                "Failed while writing remote file chunk for file {} for work {}",
                file_path.to_str().unwrap(),
                work.title
            )
        })?;
    }

    Ok(())
}

pub async fn upload_series(
    series: &Series,
    device: &Device,
    config: &Config,
    download_format: DownloadFormat,
) -> Result<()> {
    let sftp = create_sftp_connection(device)?;

    let remote_series_folder = Path::new(&device.download_folder)
        .join(&series.filtered_fandom)
        .join(&series.title);

    create_missing_folders_on_remote(
        &sftp,
        &remote_series_folder,
        Path::new(&device.download_folder),
    )?;

    for work in &series.works {
        upload_work(
            work,
            device,
            config,
            download_format,
            Some(&sftp),
            Some(series),
        )
        .await?;
    }
    Ok(())
}

fn create_missing_folders_on_remote(
    sftp: &Sftp,
    path_to_create: &Path,
    remote_download_folder: &Path,
) -> Result<()> {
    let remote_download_folder_num_ancestors = remote_download_folder.ancestors().count();
    let remote_file_ancestors = path_to_create.ancestors().collect::<Vec<&Path>>();
    let remote_file_iterator = remote_file_ancestors
        .iter()
        .rev()
        .skip(remote_download_folder_num_ancestors);

    for path in remote_file_iterator {
        if sftp.lstat(path).is_err() {
            // TODO maybe set more restrictive permissions
            sftp.mkdir(path, 0o777).with_context(|| {
                format!(
                    "Failed to make missing folder {} on remote for work",
                    path.display()
                )
            })?;
        }
    }

    Ok(())
}

pub fn create_sftp_connection(device: &Device) -> Result<Sftp> {
    let tcp = TcpStream::connect((device.ip.clone(), device.port)).with_context(|| {
        format!(
            "Failed to connect to device {} at {}:{}",
            device.name, device.ip, device.port
        )
    })?;
    let mut session = Session::new().with_context(|| {
        format!(
            "Failed to setup SSH session for device {} at {}:{}",
            device.name, device.ip, device.port
        )
    })?;
    session.set_tcp_stream(tcp);
    session.handshake().with_context(|| {
        format!(
            "Failed handshake with device {} at {}:{}",
            device.name, device.ip, device.port
        )
    })?;
    session
        .userauth_password(&device.username, &device.password)
        .with_context(|| {
            format!(
                "Failed to authenticate with device {} at {}:{}",
                device.name, device.ip, device.port
            )
        })?;
    session.set_blocking(true);
    session.sftp().with_context(|| {
        format!(
            "Failed to initialise SFTP with device {} at {}:{}",
            device.name, device.ip, device.port
        )
    })
}
