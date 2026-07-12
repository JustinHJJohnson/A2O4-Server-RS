use crate::A2O4Db;

use rocket_db_pools::Connection;

pub async fn test_connection(
    mut db: Connection<A2O4Db>,
) -> Result<String, rocket::response::Debug<sqlx::Error>> {
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&mut **db) // Dereference to access the underlying SqliteConnection
        .await?;

    Ok(format!("Database is online! Test result: {}", row.0))
}
