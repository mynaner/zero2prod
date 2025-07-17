/*
 * @Date: 2025-07-12 16:14:33
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-17 15:17:20
 * @FilePath: /zero2prod/src/routes/subscriptions.rs
 */
use actix_web::{
    HttpResponse,
    web::{self},
};
use sqlx::PgPool;

use crate::{
    domain::{NewSubscriber, SubScriberName, SubscriberEmail},
    email_client::EmailClient,
};
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
    skip(form, pool,email_client),
    fields(
        subscriber_email=%form.email,
        subscriber_name=%form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let new_subscriber: NewSubscriber = match form.0.try_into() {
        Ok(e) => e,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };

    if insert_subscriber(&pool, &new_subscriber).await.is_err() {
        println!("insert_subscriber err");
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(&email_client, new_subscriber)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    return HttpResponse::Ok().finish();
}

#[tracing::instrument(
    name = "send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
) -> Result<(), reqwest::Error> {
    let link = "https://my-api.com/subscriptions/confirm";
    let html_body = format!(
        "Welcomme to newsletter!<br/>Click <a herf=\"{}\">here</a>to confirm you subscription",
        link
    );
    let text_body = format!(
        "Welcomme to newsletter! Vlist {} to confirm you subscription",
        link
    );

    email_client
        .send_email(new_subscriber.email, "Welcomme !", &html_body, &text_body)
        .await
}

pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query! {
        r#"INSERT INTO subscriptions (id,email,name,subscribed_at,status)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        uuid::Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        chrono::Utc::now(),
        "pending_confirmation"
    }
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query:{:?}", e);
        e
    })?;

    Ok(())
}
