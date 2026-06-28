use crate::{
    common::{filter_fandoms, get_page, sanitise_string, DownloadFormat},
    config::Config,
    domain::{series::Series, user::User},
};

use anyhow::{Context, Result};
use derive_builder::Builder;
use scraper::{ElementRef, Selector};
use std::{collections::HashMap, path::Path, str::FromStr};
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Debug, PartialEq, Clone)]
pub struct SeriesLink {
    pub series_id: String,
    pub series_name: String,
    pub part_in_series: u8,
}

#[derive(Builder, Debug, Default, Clone)]
#[builder(default)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub author: String,
    pub download_links: HashMap<DownloadFormat, String>,
    pub fandoms: Vec<String>,
    pub filtered_fandom: String,
    pub relationships: Vec<String>,
    pub characters: Vec<String>,
    pub additional_tags: Vec<String>,
    pub series: HashMap<String, SeriesLink>,
}

impl std::fmt::Display for Work {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "id: {},\ntitle: {},\nauthor: {},\ndownload_links: {:?},\nfandoms: {:?},\nfiltered_fandoms: {:?},\nrelationships: {:?},\ncharacters: {:?},\nadditional_tags: {:?}\nseries: {:?}",
            self.id,
            self.title,
            self.author,
            self.download_links,
            self.fandoms,
            self.filtered_fandom,
            self.relationships,
            self.characters,
            self.additional_tags,
            self.series
        )
    }
}

impl Work {
    pub fn test_work(
        title: String,
        fandom: String,
        series: Option<String>,
        part_in_series: Option<u8>,
    ) -> Self {
        match series {
            None => Self {
                id: "1".to_owned(),
                title,
                author: String::new(),
                download_links: HashMap::default(),
                fandoms: vec![],
                filtered_fandom: fandom,
                relationships: vec![],
                characters: vec![],
                additional_tags: vec![],
                series: HashMap::default(),
            },
            Some(unwrapped_series) => Self {
                id: "1".to_owned(),
                title,
                author: String::new(),
                download_links: HashMap::default(),
                fandoms: vec![],
                filtered_fandom: fandom,
                relationships: vec![],
                characters: vec![],
                additional_tags: vec![],
                series: HashMap::from([(
                    "1".to_owned(),
                    SeriesLink {
                        series_id: "1".to_string(),
                        series_name: unwrapped_series,
                        part_in_series: part_in_series.unwrap(),
                    },
                )]),
            },
        }
    }

    pub fn get_series_link(&self, series_id: &String) -> Option<&SeriesLink> {
        self.series.get(series_id)
    }

    //TODO maybe pass in whole series and get part in series from that
    pub fn get_filename(&self, format: DownloadFormat, series_id: Option<&String>) -> String {
        let non_series_filename = format!("{}.{}", self.title, format.to_string().to_lowercase());

        if let Some(series_id) = series_id {
            match self.get_series_link(series_id) {
                Some(series_link) => format!(
                    "{} - {}.{}",
                    series_link.part_in_series,
                    self.title,
                    format.to_string().to_lowercase()
                ),
                None => non_series_filename,
            }
        } else {
            non_series_filename
        }
    }

    pub async fn parse_work(
        id: &str,
        user: &User,
        config: &Config,
        fandom_override: Option<String>,
    ) -> Result<Work> {
        println!("loading work {id}");
        let document = get_page(id, None, user).await?;
        println!("Got AO3 response");

        let title_selector = Selector::parse("h2.title.heading").expect("Error parsing title");
        let author_selector = Selector::parse("h3.byline.heading>a").expect("Error parsing author");
        let anonymous_author_selector =
            Selector::parse("h3.byline.heading").expect("Error parsing author");
        let downloads_selector =
            Selector::parse("li.download>ul>li>a").expect("Error parsing download links");
        let fandoms_selector =
            Selector::parse("dd.fandom.tags>ul>li>a").expect("Error parsing fandom tags");
        let relationships_selector = Selector::parse("dd.relationship.tags>ul>li>a")
            .expect("Error parsing relationship tags");
        let characters_selector =
            Selector::parse("dd.character.tags>ul>li>a").expect("Error parsing character tags");
        let additional_tags_selector =
            Selector::parse("dd.freeform.tags>ul>li>a").expect("Error parsing additional tags");
        let part_in_series_selector = Selector::parse("dd.series>span.series>span.position")
            .expect("Error parsing part in series");

        let test = document.html();
        println!("{test}");

        //TODO check for ssl error
        let title: String = document
            .select(&title_selector)
            .next()
            .with_context(|| format!("Could not find title for work {id}"))?
            .text()
            .collect();
        let author: String = document
            .select(&author_selector)
            .next()
            .unwrap_or(document.select(&anonymous_author_selector).next().unwrap())
            .text()
            .collect();
        let downloads_popup = document.select(&downloads_selector);
        let download_links: HashMap<DownloadFormat, String> = downloads_popup
            .map(|link| {
                (
                    DownloadFormat::from_str(&link.text().collect::<String>())
                        .expect("Failed to parse download format enum"),
                    format!(
                        "https://archiveofourown.org{}",
                        link.value().attr("href").unwrap()
                    ),
                )
            })
            .collect();
        let fandoms: Vec<String> = document
            .select(&fandoms_selector)
            .map(|x| x.text().collect())
            .collect();
        let relationships: Vec<String> = document
            .select(&relationships_selector)
            .map(|x| x.text().collect())
            .collect();
        let characters: Vec<String> = document
            .select(&characters_selector)
            .map(|x| x.text().collect())
            .collect();
        let additional_tags: Vec<String> = document
            .select(&additional_tags_selector)
            .map(|x| x.text().collect())
            .collect();
        let series_element = document.select(&part_in_series_selector);
        let series_links: HashMap<String, SeriesLink> = series_element
            .map(|series| {
                let series_name_element = series.child_elements().next().unwrap();
                let series_id = series_name_element
                    .value()
                    .attr("href")
                    .unwrap()
                    .split_terminator('/')
                    .nth(2)
                    .unwrap()
                    .to_owned();

                (
                    series_id.clone(),
                    SeriesLink {
                        series_name: sanitise_string(
                            &series_name_element
                                .text()
                                .collect::<String>()
                                .split_whitespace()
                                .filter(|chunk| *chunk != "series")
                                .collect::<Vec<&str>>()
                                .join(" "),
                        ),
                        series_id,
                        part_in_series: series
                            .text()
                            .collect::<String>()
                            .split_whitespace()
                            .nth(1)
                            .unwrap()
                            .parse::<u8>()
                            .unwrap(),
                    },
                )
            })
            .collect();

        println!("Work loaded");

        Ok(Work {
            id: id.to_owned(),
            title: sanitise_string(&title),
            author,
            download_links,
            fandoms: fandoms.clone(),
            filtered_fandom: match fandom_override {
                Some(fandom) => fandom,
                None => filter_fandoms(&fandoms, config),
            },
            relationships,
            characters,
            additional_tags,
            series: series_links,
        })
    }

    pub fn parse_work_from_blurb(
        blurb: ElementRef,
        series_name: &str,
        config: &Config,
    ) -> Result<Work> {
        let heading_selector = Selector::parse("h4.heading>a").expect("Error parsing heading");
        let fandoms_selector =
            Selector::parse("h5.fandoms.heading>a.tag").expect("Error parsing fandom tags");
        let relationships_selector =
            Selector::parse("li.relationships>a.tag").expect("Error parsing relationship tags");
        let characters_selector =
            Selector::parse("li.characters>a.tag").expect("Error parsing character tags");
        let additional_tags_selector =
            Selector::parse("li.freeforms>a.tag").expect("Error parsing additional tags");
        let series_selector = Selector::parse("ul.series>li").expect("Error parsing series");

        let mut heading = blurb.select(&heading_selector);
        let title_element = heading.next().context("Could not find title for work")?;
        let id: String = title_element
            .attr("href")
            .context("Could not find id for work in blurb")?
            .split_terminator('/')
            .nth(2)
            .context("Could not find id for work in blurb")?
            .to_owned();
        let title: String = title_element.text().collect();

        println!("  Parsing work {id} - {title}");

        let author: String = if let Some(element) = heading.next() {
            element.text().collect()
        } else {
            "Anonymous".to_owned() //TODO use a proper selector for this
        };
        let download_links: HashMap<DownloadFormat, String> =
            enum_iterator::all::<DownloadFormat>()
                .map(|download_format| {
                    (
                        download_format,
                        format!(
                            "https://download.archiveofourown.org/downloads/{}/work.{}",
                            id,
                            download_format.to_string().to_lowercase()
                        ),
                    )
                })
                .collect();
        let fandoms: Vec<String> = blurb
            .select(&fandoms_selector)
            .map(|fandom| fandom.text().collect())
            .collect();
        let relationships: Vec<String> = blurb
            .select(&relationships_selector)
            .map(|relationship| relationship.text().collect())
            .collect();
        let characters: Vec<String> = blurb
            .select(&characters_selector)
            .map(|character| character.text().collect())
            .collect();
        let additional_tags: Vec<String> = blurb
            .select(&additional_tags_selector)
            .map(|tag| tag.text().collect())
            .collect();
        let series_element = blurb.select(&series_selector);
        let series_links: HashMap<String, SeriesLink> = series_element
            .map(|series| {
                let mut elements = series.child_elements();
                let part_in_series = elements
                    .next()
                    .unwrap()
                    .text()
                    .collect::<String>()
                    .parse::<u8>()
                    .unwrap();
                let series_id = elements
                    .next()
                    .unwrap()
                    .value()
                    .attr("href")
                    .unwrap()
                    .split_terminator('/')
                    .nth(2)
                    .unwrap()
                    .to_owned();

                (
                    series_id.clone(),
                    SeriesLink {
                        series_name: sanitise_string(series_name),
                        series_id,
                        part_in_series,
                    },
                )
            })
            .collect();

        println!("  Work parsed\n");

        Ok(Work {
            id: id.clone(),
            title: sanitise_string(&title),
            author,
            download_links,
            fandoms: fandoms.clone(),
            filtered_fandom: filter_fandoms(&fandoms, config),
            relationships,
            characters,
            additional_tags,
            series: series_links,
        })
    }

    pub async fn download(
        &self,
        download_folder: &Path,
        format: DownloadFormat,
        series: Option<&Series>,
        user: &User,
    ) -> Result<()> {
        let download_link = self.download_links[&format].clone();
        println!("Download link: {download_link}");

        let work_response = user
            .client
            .get(&download_link)
            .send()
            .await
            .with_context(|| {
                format!("Error downloading work {} from {download_link}", self.title)
            })?;

        if work_response.status() == 525 {
            return Err(anyhow::anyhow!(
                "SSL error trying to download work {}: {work_response:?}",
                self.title
            ));
        } else if !work_response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Unknown error trying to download work {}: {work_response:?}",
                self.title
            ));
        }

        let work = work_response
            .bytes()
            .await
            .with_context(|| format!("Error converting work {} to bytes", self.title))?;
        let download_path = download_folder.join(self.get_filename(format, series.map(|x| &x.id)));

        println!("Downloading to: {}", download_folder.to_str().unwrap());

        let mut work_file = File::create(&download_path).await.with_context(|| {
            format!(
                "Error creating file for work {} at {}",
                self.title,
                download_path.display()
            )
        })?;
        work_file.write_all(&work).await.with_context(|| {
            format!(
                "Error writing file for work {} at {}",
                self.title,
                download_path.display()
            )
        })?;
        work_file.flush().await.with_context(|| {
            format!(
                "Error writing file during flush for work {} at {}",
                self.title,
                download_path.display()
            )
        })?;
        Ok(())
    }
}
