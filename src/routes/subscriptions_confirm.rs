/*
 * @Date: 2025-07-17 15:40:30
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-18 15:58:16
 * @FilePath: /zero2prod/src/routes/subscriptions_confirm.rs
 */
use actix_web::{HttpResponse, web};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

// 确认一个打开的订阅
#[tracing::instrument(name = "confrim opending a subscribe", skip(parameters, pool))]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
        Ok(e) => e,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        Some(id) => {
            if confirm_subscriber(&pool, id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
        None => return HttpResponse::Unauthorized().finish(),
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(
    pool: &PgPool,
    subscriber_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, subscription_token))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscription_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execture query:{:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscription_id))
}
