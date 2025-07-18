/*
 * @Date: 2025-07-17 15:40:30
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-18 10:08:07
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
#[tracing::instrument(name = "confrim opending a subscribe", skip(data))]
pub async fn subscriptions_confirm(
    data: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    HttpResponse::Ok().finish()
}
