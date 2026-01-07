pub mod urls {
    use sqlx::{PgPool, postgres::PgQueryResult};

    pub async fn find_url_by_code(
        pool: &PgPool,
        code: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT url from urls where code = $1")
            .bind(code)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_code_by_url(pool: &PgPool, url: &str) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT code FROM urls WHERE url = $1")
            .bind(url)
            .fetch_optional(pool)
            .await
    }

    pub async fn insert(
        pool: &PgPool,
        code: &str,
        url: &str,
    ) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("INSERT INTO urls (code, url) VALUES ($1, $2)")
            .bind(code)
            .bind(url)
            .execute(pool)
            .await
    }

    pub async fn list_all(pool: &PgPool) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as("SELECT code, url FROM urls")
            .fetch_all(pool)
            .await
    }

    pub async fn delete_all(pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM urls").execute(pool).await
    }

    pub async fn delete_code(pool: &PgPool, code: &str) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("DELETE FROM urls where code = $1 RETURNING url")
            .bind(code)
            .fetch_optional(pool)
            .await
    }
}

pub mod clicks {
    use sqlx::{PgPool, postgres::PgQueryResult};

    pub async fn insert(pool: &PgPool, code: &str) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::query("INSERT INTO clicks (code) VALUES ($1)")
            .bind(code)
            .execute(pool)
            .await
    }
}
