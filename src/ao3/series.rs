use crate::ao3::common::{filter_fandoms, get_page, DownloadFormat};
use crate::ao3::user::User;
use crate::ao3::work::Work;
use crate::config::Config;

use anyhow::Result;
use scraper::Selector;
use std::collections::HashSet;
use std::fs::create_dir;
use std::path::Path;

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
    pub fn parse_series(id: &str, user: Option<&User>, config: &Config) -> Result<Series> {
        println!("Loading series {}", id);
        let mut document = get_page(id, Some(1), user).expect("Failed to get the requested page");

        let pagination_selector = Selector::parse("ol.pagination.actions>li")
            .expect("Failed to parse pagination buttons");
        let pagination_elements = document.select(&pagination_selector).count() as u8;
        let num_series_pages = if pagination_elements > 0 {
            pagination_elements / 2 - 2
        } else {
            1
        };

        let title_selector = Selector::parse("h2.heading").expect("Failed to parse title");
        let creator_selector =
            Selector::parse("dl.series.meta.group>dd>a").expect("Failed to parse creator");
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
            .unwrap()
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
            .unwrap_or_else(|_| panic!("Failed to convert {} to u32", raw_num_words));
        let num_works: u32 = raw_num_works
            .replace(&[',', '.'][..], "")
            .parse()
            .unwrap_or_else(|_| panic!("Failed to convert {} to u32", raw_num_works));
        let is_completed: bool = match raw_is_completed.as_str() {
            "Yes" => true,
            "No" => false,
            _ => false,
        };
        let num_bookmarks: u32 = raw_num_bookmarks
            .replace(&[',', '.'][..], "")
            .parse()
            .unwrap_or_else(|_| panic!("Failed to convert {} to u32", raw_num_bookmarks));

        let mut works = Vec::new();
        let mut authors = HashSet::new();
        let mut fandoms = HashSet::new();

        for page in 1..=num_series_pages {
            if page > 1 {
                document = get_page(id, Some(page), user)?;
            };
            for work in document.select(&work_selector) {
                let work_id = work
                    .value()
                    .attr("id")
                    .unwrap()
                    .chars()
                    .skip(5)
                    .collect::<String>();
                println!("  Found work {}", work_id);
                let parsed_work = Work::parse_work_from_blurb(work, &title, config)?;
                fandoms.extend(parsed_work.fandoms.clone());
                authors.insert(parsed_work.author.clone());
                works.push(parsed_work);
            }
        }

        Ok(Series {
            id: id.to_owned(),
            title,
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

    pub fn download(&self, path: &Path, format: DownloadFormat) -> std::io::Result<()> {
        let series_path = path.join(&self.title);
        create_dir(&series_path)?;
        for work in &self.works {
            let _ = work.download(&series_path, format, Some(&self.id));
            println!()
        };
        Ok(())
    }
}
