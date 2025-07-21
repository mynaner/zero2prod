use core::str;
/*
 * @Date: 2025-07-12 16:14:33
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-20 22:19:39
 * @FilePath: /zero2prod/src/routes/subscriptions.rs
 */
use actix_web::{
    HttpResponse, ResponseError,
    http::StatusCode,
    web::{self},
};
use anyhow::Context;
use rand::{Rng, distributions::Alphanumeric, thread_rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubScriberName, SubscriberEmail},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
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
    skip(form, pool,email_client,base_url),
    fields(
        subscriber_email=%form.email,
        subscriber_name=%form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    base_url: web::Data<ApplicationBaseUrl>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber: NewSubscriber =
        form.0.try_into().map_err(SubscribeError::ValidationError)?;
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from the pool")?;
    // .map_err(|e| {
    //     SubscribeError::UnexpectedError(
    //         Box::new(e),
    //         "Failed to acquire a postgres connection from the pool".to_owned(),
    //     )
    // })?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database")?;
    let subscriber_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &subscriber_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to stroe a new subscriber.")?;

    send_confirmation_email(&email_client, new_subscriber, &base_url, &subscriber_token)
        .await
        .context("Failed to send a confirmation email")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &ApplicationBaseUrl,
    token: &str,
) -> Result<(), reqwest::Error> {
    let link = format!(
        "{}/subscriptions/confirm?subscription_token={token}",
        base_url.0
    );
    let html_body = format!(
        "Welcomme to newsletter!<br/>Click <a herf=\"{}\">here</a>to confirm you subscription",
        link
    );
    let text_body = format!(
        "Welcomme to newsletter! Vlist {} to confirm you subscription",
        link
    );

    email_client
        .send_email(&new_subscriber.email, "Welcomme !", &html_body, &text_body)
        .await
}

pub async fn insert_subscriber(
    transation: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = uuid::Uuid::new_v4();

    sqlx::query!(
        r#"INSERT INTO subscriptions (id,email,name,subscribed_at,status)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        chrono::Utc::now(),
        "pending_confirmation"
    )
    .execute(&mut **transation)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query:{:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    // repeat_with 会不停的调用这个笔包生成器
    std::iter::repeat_with(|| {
        // 生成 0-9a-zA-Z 随机单个值 返回的是 u8类型
        rng.sample(Alphanumeric)
    })
    //  u8 转换为cart 类型
    .map(char::from)
    // 取前25个字符
    .take(25)
    // 转换为接收值
    .collect()
}

#[tracing::instrument(
    name = "Store subscription token in the database.",
    skip(transation, subscriber_id, subscriber_token)
)]
async fn store_token(
    transation: &mut Transaction<'_, Postgres>,
    subscriber_id: uuid::Uuid,
    subscriber_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"
            INSERT INTO subscription_tokens ( subscription_token,subscription_id )
            VALUES ($1,$2)
        "#,
        subscriber_token,
        subscriber_id
    )
    .execute(&mut **transation)
    .await
    .map_err(StoreTokenError)?;

    Ok(())
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a database error was encountered while trying to store a subscription token."
        )
    }
}
impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

// impl ResponseError for StoreTokenError {}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(f, "{}\n", e)?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    // #[error("Failed to insert new subscriber in the database")]
    // InsertSubscriberError(#[source] sqlx::Error),
    #[error("{0}")]
    ValidationError(String),
    // #[error("{1}")]
    // UnexpectedError(#[source] Box<dyn std::error::Error>, String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    // #[error("failed to store the confirmation token for a new subscriber .")]
    // StoreTokenError(StoreTokenError),
    // #[error("Failed to send a confirmation email.")]
    // SendEmailError(reqwest::Error),
    // // DatabaseError(sqlx::Error),
    // #[error("Failed to acquire a postgres connection from the pool")]
    // PoolError(sqlx::Error),
    // #[error("Failed to insert new subscriber in the database")]
    // InsertSubscriberError(sqlx::Error),
    // #[error("Failed to commit SQL transaction to stroe a new subscriber.")]
    // TransationCommitError(sqlx::Error),
}

// impl std::fmt::Display for SubscribeError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::SendEmailError(e) => write!(f, "Failed to send a confirmation email"),
//             Self::StoreTokenError(e) => write!(
//                 f,
//                 "Failed to store the confirmation token for a new subscriber"
//             ),
//             Self::ValidationError(e) => write!(f, "{}", e),

//             Self::PoolError(e) => {
//                 write!(f, "Failed to acquire a Postgres connection from the pool")
//             }
//             Self::InsertSubscriberError(e) => {
//                 write!(f, "Failed to insert new subscriber in the database")
//             }
//             Self::TransationCommitError(e) => write!(
//                 f,
//                 "Failed to commit SQL transaction to store a new subscriber."
//             ),
//         }
//     }
// }

// impl std::error::Error for SubscribeError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             Self::ValidationError(_) => None,

//             Self::SendEmailError(e) => Some(e),
//             Self::StoreTokenError(e) => Some(e),
//             Self::PoolError(e) => Some(e),
//             Self::TransationCommitError(e) => Some(e),
//             Self::InsertSubscriberError(e) => Some(e),
//         }
//     }
// }

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            // Self::UnexpectedError(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
            // Self::InsertSubscriberError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
