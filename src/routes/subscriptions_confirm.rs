/*
 * @Date: 2025-07-17 15:40:30
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-17 15:50:47
 * @FilePath: /zero2prod/src/routes/subscriptions_confirm.rs
 */
use actix_web::{HttpResponse, web};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Parameters {
    subscriptions_token: String,
}

// 确认一个打开的订阅
#[tracing::instrument(name = "confrim opending a subscribe", skip(data))]
pub async fn subscriptions_confirm(data: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
