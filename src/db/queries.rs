pub mod urls {
    use crate::sql_query;
    use sqlx::{PgPool, postgres::PgQueryResult};

    pub async fn find_url_by_code(
        pool: &PgPool,
        code: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let stmt = sql_query!("urls", "find_url_by_code");
        sqlx::query_scalar(stmt)
            .bind(code)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_code_by_url(pool: &PgPool, url: &str) -> Result<Option<String>, sqlx::Error> {
        let stmt = sql_query!("urls", "find_code_by_url");
        sqlx::query_scalar(stmt)
            .bind(url)
            .fetch_optional(pool)
            .await
    }

    pub async fn insert(
        pool: &PgPool,
        code: &str,
        url: &str,
    ) -> Result<PgQueryResult, sqlx::Error> {
        let stmt = sql_query!("urls", "insert");
        sqlx::query(stmt).bind(code).bind(url).execute(pool).await
    }

    pub async fn list_all(pool: &PgPool) -> Result<Vec<(String, String)>, sqlx::Error> {
        let stmt = sql_query!("urls", "list_all");
        sqlx::query_as(stmt).fetch_all(pool).await
    }

    pub async fn delete_all(pool: &PgPool) -> Result<PgQueryResult, sqlx::Error> {
        let stmt = sql_query!("urls", "delete_all");
        sqlx::query(stmt).execute(pool).await
    }

    pub async fn delete_code(pool: &PgPool, code: &str) -> Result<Option<String>, sqlx::Error> {
        let stmt = sql_query!("urls", "delete_code");
        sqlx::query_scalar(stmt)
            .bind(code)
            .fetch_optional(pool)
            .await
    }
}

pub mod clicks {
    use crate::sql_query;
    use serde::Serialize;
    use sqlx::{PgPool, postgres::PgQueryResult, types::chrono::NaiveDate};
    use utoipa::ToSchema;

    pub async fn insert(pool: &PgPool, code: &str) -> Result<PgQueryResult, sqlx::Error> {
        let stmt = sql_query!("clicks", "insert");
        sqlx::query(stmt).bind(code).execute(pool).await
    }

    pub async fn get_code_total_clicks(pool: &PgPool, code: &str) -> Result<i64, sqlx::Error> {
        let stmt = sql_query!("clicks", "get_code_total_clicks");
        sqlx::query_scalar(stmt).bind(code).fetch_one(pool).await
    }

    #[derive(Serialize, Debug, sqlx::FromRow, ToSchema)]
    pub struct DailyClick {
        date: NaiveDate,
        count: i64,
    }

    pub async fn get_code_daily_clicks(
        pool: &PgPool,
        code: &str,
    ) -> Result<Vec<DailyClick>, sqlx::Error> {
        let stmt = sql_query!("clicks", "get_code_daily_clicks");
        sqlx::query_as(stmt).bind(code).fetch_all(pool).await
    }
}
