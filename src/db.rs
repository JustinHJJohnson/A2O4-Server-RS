use crate::{
    domain::{series::Series, work::Work},
    A2O4Db,
};

use rocket_db_pools::{sqlx, Connection};
use sqlx::{Acquire, Execute, QueryBuilder, Sqlite, SqliteConnection};
use strum_macros::{Display, EnumString};

#[derive(Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TagType {
    Fandom,
    Characters,
    Relationships,
    Additional,
}

pub async fn insert_work<'a, A>(
    //Allows this function to be called with either a raw connection or a transaction
    attachable: A,
    work: &Work,
) -> Result<(), sqlx::Error>
where
    A: Acquire<'a, Database = Sqlite>,
{
    let mut tx = attachable.begin().await?;

    println!("inserting work");
    sqlx::query("INSERT INTO work (id, title, filtered_fandom) VALUES ($1, $2, $3)")
        .bind(&work.id)
        .bind(&work.title)
        .bind(&work.filtered_fandom)
        .execute(&mut *tx)
        .await?;

    println!("inserting authors");
    let author_ids = insert_authors(&mut tx, &work.authors).await?;
    println!("inserting work author links");
    insert_work_author_link(&mut tx, &author_ids, &work.id).await?;

    println!("inserting tags");
    let tag_ids = insert_tags(
        &mut tx,
        &work.fandoms,
        &work.characters,
        &work.relationships,
        &work.additional_tags,
    )
    .await?;
    println!("inserting work tag links");
    insert_work_tags_links(&mut tx, tag_ids, &work.id).await?;

    tx.commit().await?;

    Ok(())
}

pub async fn insert_series(mut db: Connection<A2O4Db>, series: &Series) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;

    sqlx::query("
        INSERT INTO series (id, title, begun, updated, description, num_words, num_works, is_completed, num_bookmarks)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    ")
        .bind(&series.id)
        .bind(&series.title)
        .bind(&series.begun)
        .bind(&series.updated)
        .bind(&series.description)
        .bind(series.num_words)
        .bind(series.num_works)
        .bind(series.is_completed)
        .bind(series.num_bookmarks)
        .execute(&mut *tx)
        .await?;

    for work in &series.works {
        insert_work(&mut tx, work).await?;
    }

    insert_work_series_link(&mut tx, &series.works, &series.id).await?;

    tx.commit().await?;

    Ok(())
}

async fn insert_work_series_link(
    tx: &mut SqliteConnection,
    works: &Vec<Work>,
    series_id: &str,
) -> Result<(), sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("INSERT OR IGNORE INTO work_series_link (work, series) ");

    query_builder.push_values(works, |mut query, work| {
        query.push_bind(&work.title).push_bind(series_id);
    });

    let test = query_builder.build();

    println!("{}", test.sql());

    test.execute(tx).await?;

    Ok(())
}

async fn insert_authors(
    tx: &mut SqliteConnection,
    authors: &Vec<String>,
) -> Result<Vec<i64>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("INSERT INTO author (name) ");

    query_builder.push_values(authors, |mut query, author| {
        query.push_bind(author);
    });

    query_builder
        //Do a dummy update so sqlite will still return the id for an existing author
        .push(" ON CONFLICT(name) DO UPDATE SET name=excluded.name")
        .push(" RETURNING id");

    let test = query_builder.build_query_scalar();

    println!("{}", test.sql());

    let ids: Vec<i64> = test.fetch_all(tx).await?;

    Ok(ids)
}

async fn insert_work_author_link(
    tx: &mut SqliteConnection,
    author_ids: &Vec<i64>,
    work_id: &str,
) -> Result<(), sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("INSERT OR IGNORE INTO work_author_link (work, author) ");

    query_builder.push_values(author_ids, |mut query, author| {
        query.push_bind(work_id).push_bind(author);
    });

    let test = query_builder.build();

    println!("{}", test.sql());

    test.execute(tx).await?;

    Ok(())
}

async fn insert_tags(
    tx: &mut SqliteConnection,
    fandoms: &Vec<String>,
    characters: &Vec<String>,
    relationships: &Vec<String>,
    additional: &Vec<String>,
) -> Result<Vec<i64>, sqlx::Error> {
    let mut all_tags = Vec::new();

    for tag in fandoms {
        all_tags.push((TagType::Fandom.to_string(), tag));
    }
    for tag in characters {
        all_tags.push((TagType::Characters.to_string(), tag));
    }
    for tag in relationships {
        all_tags.push((TagType::Relationships.to_string(), tag));
    }
    for tag in additional {
        all_tags.push((TagType::Additional.to_string(), tag));
    }

    if all_tags.is_empty() {
        return Ok(Vec::new());
    }

    let mut query_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("INSERT INTO tag (type, name) ");

    query_builder.push_values(all_tags, |mut query, (tag_type, tag_name)| {
        query.push_bind(tag_type).push_bind(tag_name);
    });

    query_builder
        .push(" ON CONFLICT(name) DO UPDATE SET name=excluded.name")
        .push(" RETURNING id");

    let ids: Vec<i64> = query_builder.build_query_scalar().fetch_all(tx).await?;

    Ok(ids)
}

async fn insert_work_tags_links(
    tx: &mut SqliteConnection,
    tag_ids: Vec<i64>,
    work_id: &str,
) -> Result<(), sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> =
        QueryBuilder::new("INSERT OR IGNORE INTO work_tag_link (work, tag) ");

    query_builder.push_values(tag_ids, |mut query, tag_id| {
        query.push_bind(work_id).push_bind(tag_id);
    });

    query_builder.build().execute(tx).await?;

    Ok(())
}
