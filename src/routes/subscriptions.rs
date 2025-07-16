/*
 * @Date: 2025-07-12 16:14:33
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-15 10:26:02
 * @FilePath: /zero2prod/src/routes/subscriptions.rs
 */
use actix_web::{
    HttpResponse,
    web::{self},
};
use sqlx::PgPool;

use crate::domain::{NewSubscriber, SubScriberName, SubscriberEmail};
#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

// 实现 Display
impl std::fmt::Display for FormData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, email: {}", self.name, self.email)
    }
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let email = SubscriberEmail::parse(value.email)?;
        let name = SubScriberName::parse(value.name)?;
        Ok(NewSubscriber { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber", 
    skip(form, pool),
    fields(
        subscriber_email=%form.email,
        subscriber_name=%form.name
    )
)]
pub async fn subscribe(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(e) => e,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscriptions (id,email,name,subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        uuid::Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        chrono::Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query:{:?}", e);
        e
    })?;

    Ok(())
}
