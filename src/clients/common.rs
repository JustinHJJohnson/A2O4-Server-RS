use crate::ao3::{series::Series, work::Work};

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use anyhow::{Context, Error};

pub fn get_file_with_size(
    work: &Work,
    series: Option<&Series>,
    filename: &str,
    download_path: &str,
) -> Result<(Vec<u8>, u64), Error> {
    let file_path = if let Some(unwrapped_series) = series {
        Path::new(&download_path)
            .join(unwrapped_series.title.clone())
            .join(filename)
    } else {
        Path::new(&download_path).join(filename)
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
    Ok((file_contents, file.metadata()?.len()))
}

pub fn generate_remote_path(
    work: Option<&Work>,
    series: Option<&Series>,
    filename: Option<String>,
    remote_download_folder: &str,
) -> PathBuf {
    let mut remote_file_path = PathBuf::from(remote_download_folder);

    if let Some(unwrapped_series) = series {
        remote_file_path.push(unwrapped_series.filtered_fandom.clone());
        if unwrapped_series.filtered_fandom == "Original Work" {
            remote_file_path.push(unwrapped_series.creator.clone());
        }
        remote_file_path.push(unwrapped_series.title.clone());
    } else {
        let unwrapped_work = work.unwrap();
        remote_file_path.push(unwrapped_work.filtered_fandom.clone());
        if unwrapped_work.filtered_fandom == "Original Work" {
            remote_file_path.push(unwrapped_work.author.clone());
        }
    }
    if let Some(unwrapped_filename) = filename {
        remote_file_path.push(unwrapped_filename);
    }

    remote_file_path
}

#[cfg(test)]
mod tests {
    use crate::ao3::series::SeriesBuilder;
    use crate::ao3::work::WorkBuilder;
    use super::*;

    #[test]
    fn generate_remote_path_for_series_no_work() {
        let series = SeriesBuilder::default()
            .title("Series 1".into())
            .creator("Bobbert".into())
            .filtered_fandom("Bob the Builder".into())
            .build()
            .unwrap();
        assert_eq!(
            generate_remote_path(None, Some(&series), None, "/Download"),
            PathBuf::from("/Download/Bob the Builder/Series 1")
        );
    }
    #[test]
    fn generate_remote_path_for_series_no_work_original_work() {
        let series = SeriesBuilder::default()
            .title("Series 1".into())
            .creator("Bobbert".into())
            .filtered_fandom("Original Work".into())
            .build()
            .unwrap();
        assert_eq!(
            generate_remote_path(None, Some(&series), None, "/Download"),
            PathBuf::from("/Download/Original Work/Bobbert/Series 1")
        );
    }

    #[test]
    fn generate_remote_path_for_series_work() {
        let series = SeriesBuilder::default()
            .title("Series 1".into())
            .creator("Bobbert".into())
            .filtered_fandom("Bob the Builder".into())
            .build()
            .unwrap();
        let work = WorkBuilder::default()
            .filtered_fandom("Not Original Work".into())
            .build()
            .unwrap();

        assert_eq!(
            generate_remote_path(Some(&work), Some(&series), None, "/Download"),
            PathBuf::from("/Download/Bob the Builder/Series 1")
        );
    }

    #[test]
    fn generate_remote_path_for_series_work_original_work() {
        let series = SeriesBuilder::default()
            .title("Series 1".into())
            .creator("Bobbert".into())
            .filtered_fandom("Original Work".into())
            .build()
            .unwrap();
        let work = WorkBuilder::default()
            .filtered_fandom("Not Original Work".into())
            .build()
            .unwrap();

        assert_eq!(
            generate_remote_path(Some(&work), Some(&series), None, "/Download"),
            PathBuf::from("/Download/Original Work/Bobbert/Series 1")
        );
    }

    #[test]
    fn generate_remote_path_for_work_no_series() {
        let work = WorkBuilder::default()
            .filtered_fandom("Bob the Builder".into())
            .build()
            .unwrap();

        assert_eq!(
            generate_remote_path(Some(&work), None, Some("title.epub".into()), "/Download"),
            PathBuf::from("/Download/Bob the Builder/title.epub")
        );
    }

    #[test]
    fn generate_remote_path_for_work_no_series_original_work() {
        let work = WorkBuilder::default()
            .filtered_fandom("Original Work".into())
            .author("Bobbert".into())
            .build()
            .unwrap();

        assert_eq!(
            generate_remote_path(Some(&work), None, Some("title.epub".into()), "/Download"),
            PathBuf::from("/Download/Original Work/Bobbert/title.epub")
        );
    }
}
