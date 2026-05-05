use crate::ao3::common::{filter_fandoms, get_series_pages, sanitise_string, DownloadFormat};
use crate::ao3::user::User;
use crate::ao3::work::Work;
use crate::config::Config;

use anyhow::Result;
use scraper::Selector;
use std::collections::HashSet;
use std::fs::read_dir;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tokio::fs::create_dir;

pub struct Series {
    pub id: String,
    pub title: String,
    creator: String,
    series_begun: String,   // TODO make some sort of date type
    series_updated: String, // TODO make some sort of date type
    description: String,
    num_words: u32,
    num_works: u32,
    is_completed: bool,
    num_bookmarks: u32,

    //These are gotten from parsing all the works in the series
    pub works: Vec<Work>,
    authors: HashSet<String>,
    fandoms: HashSet<String>,
    pub filtered_fandom: String,
}

impl std::fmt::Display for Series {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "id: {}\ntitle: {}\ncreator: {}\nseries_begun: {}\nseries_updated: {}\ndescription: {}\nnum_words: {}\nnum_works: {}\nis_completed: {}\nnum_bookmarks: {}\nworks: {:?}\nauthors: {:?}\nfandoms: {:?}\nfiltered_fandoms: {:?}",
            self.id,
            self.title,
            self.creator,
            self.series_begun,
            self.series_updated,
            self.description,
            self.num_words,
            self.num_works,
            self.is_completed,
            self.num_bookmarks,
            self.works,
            self.authors,
            self.fandoms,
            self.filtered_fandom
        )
    }
}

impl Series {
    pub fn test_series(title: String, fandom: String, config: &Config) -> Result<Series> {
        let (works, num_works) =
            Self::load_series_works_from_local(title.clone(), fandom.clone(), config)?;

        Ok(Series {
            id: "1".to_owned(),
            title: sanitise_string(&*title),
            creator: "bob".to_owned(),
            series_begun: "at some point".to_owned(),
            series_updated: "sure".to_owned(),
            description: "yes".to_owned(),
            num_words: 1,
            num_works,
            is_completed: true,
            num_bookmarks: 0,
            works,
            authors: HashSet::from(["yes".to_string()]),
            fandoms: HashSet::from(["yes".to_string()]),
            filtered_fandom: fandom,
        })
    }

    fn load_series_works_from_local(
        title: String,
        fandom: String,
        config: &Config,
    ) -> Result<(Vec<Work>, u32)> {
        let series_path = Path::new(&config.download_path).join(title.clone());
        let works = read_dir(series_path.clone())?;
        let mut num_works = 0;

        Ok((
            works
                .map(|work| {
                    num_works += 1;
                    let binding = Into::<PathBuf>::into(work.unwrap().file_name());
                    let (num_in_series, work_title) = binding
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split_once(" - ")
                        .unwrap();
                    Work::test_work(
                        work_title.to_string(),
                        fandom.to_string(),
                        Some(title.to_string()),
                        Some(num_in_series.parse::<u8>().unwrap()),
                    )
                })
                .collect(),
            num_works,
        ))
    }

    pub async fn parse_series(id: &str, user: &User, config: &Config) -> Result<Series> {
        println!("Loading series {id}");
        let all_pages = get_series_pages(id, user).await?;
        println!("Got AO3 response");
        let document = all_pages.first().unwrap();

        let title_selector = Selector::parse("h2.heading").expect("Failed to parse title");
        let creator_selector =
            Selector::parse("dl.series.meta.group>dd>a").expect("Failed to parse creator");
        let anonymous_creator_selector =
            Selector::parse("dl.series.meta.group>dd").expect("Failed to parse creator");
        let series_date_selector =
            Selector::parse("dl.series.meta.group>dd").expect("Failed to parse series dates");
        let description_selector =
            Selector::parse("blockquote.userstuff>p").expect("Failed to parse description");
        let words_selector = Selector::parse("dd.words").expect("Failed to parse number of words");
        let works_selector = Selector::parse("dd.works").expect("Failed to parse number of works");
        let completed_selector =
            Selector::parse("dl.stats>dd").expect("Failed to parse if series is completed");
        let bookmarks_selector =
            Selector::parse("dd.bookmarks>a").expect("Failed to parse number of bookmarks");
        let work_selector = Selector::parse("li.work.blurb").expect("Failed to parse work blurbs");

        let mut series_date_select = document.select(&series_date_selector);
        series_date_select.next(); //Skip creator field to be picked up by different selector

        let test = document.html();
        println!("{test}");

        let title: String = document
            .select(&title_selector)
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .split_whitespace()
            .filter(|chunk| *chunk != "series")
            .collect::<Vec<&str>>()
            .join(" ");
        let creator: String = document
            .select(&creator_selector)
            .next()
            .unwrap_or(document.select(&anonymous_creator_selector).next().unwrap())
            .text()
            .collect();
        let series_begun: String = series_date_select.next().unwrap().text().collect();
        let series_updated: String = series_date_select.next().unwrap().text().collect();
        let description: String = document
            .select(&description_selector)
            .next()
            .unwrap()
            .text()
            .collect();
        let raw_num_words: String = document
            .select(&words_selector)
            .next()
            .unwrap()
            .text()
            .collect();
        let raw_num_works: String = document
            .select(&works_selector)
            .next()
            .unwrap()
            .text()
            .collect();
        let raw_is_completed: String = document
            .select(&completed_selector)
            .nth(2)
            .unwrap()
            .text()
            .collect();
        let raw_num_bookmarks: String = document
            .select(&bookmarks_selector)
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim()
            .parse()
            .expect("Failed to parse number of bookmarks after selecting");

        let num_words: u32 = raw_num_words
            .replace(&[',', '.'][..], "")
            .parse()
            .unwrap_or_else(|_| panic!("Failed to convert {raw_num_words} to u32"));
        let num_works: u32 = raw_num_works
            .replace(&[',', '.'][..], "")
            .parse()
            .unwrap_or_else(|_| panic!("Failed to convert {raw_num_works} to u32"));
        let is_completed: bool = match raw_is_completed.as_str() {
            "Yes" => true,
            "No" => false,
            _ => false,
        };
        let num_bookmarks: u32 = raw_num_bookmarks
            .replace(&[',', '.'][..], "")
            .parse()
            .unwrap_or_else(|_| panic!("Failed to convert {raw_num_bookmarks} to u32"));

        let mut works = Vec::new();
        let mut authors = HashSet::new();
        let mut fandoms = HashSet::new();

        for page in all_pages {
            for work in page.select(&work_selector) {
                let work_id = work
                    .value()
                    .attr("id")
                    .unwrap()
                    .chars()
                    .skip(5)
                    .collect::<String>();
                println!("  Found work {work_id}");
                let parsed_work = Work::parse_work_from_blurb(work, &title, config)?;
                fandoms.extend(parsed_work.fandoms.clone());
                authors.insert(parsed_work.author.clone());
                works.push(parsed_work);
            }
        }

        println!("Finished parsing series");

        Ok(Series {
            id: id.to_owned(),
            title: sanitise_string(&title),
            creator,
            series_begun,
            series_updated,
            description,
            num_words,
            num_works,
            is_completed,
            num_bookmarks,
            works,
            authors,
            fandoms: fandoms.clone(),
            filtered_fandom: filter_fandoms(&Vec::from_iter(fandoms), config),
        })
    }

    pub async fn download(
        &self,
        path: &Path,
        format: DownloadFormat,
        user: &User,
    ) -> std::io::Result<()> {
        let series_path = path.join(&self.title);
        match create_dir(&series_path).await {
            Ok(_) => {}
            Err(error) => match error.kind() {
                ErrorKind::AlreadyExists => {}
                _ => return Err(error),
            },
        };
        for work in &self.works {
            let _ = work.download(&series_path, format, Some(&self), user).await;
            println!();
        }
        Ok(())
    }
}
