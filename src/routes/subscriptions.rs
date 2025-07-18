/*
 * @Date: 2025-07-12 16:14:33
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-18 16:28:50
 * @FilePath: /zero2prod/src/routes/subscriptions.rs
 */
use actix_web::{
    HttpResponse,
    web::{self},
};
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
) -> HttpResponse {
    let new_subscriber: NewSubscriber = match form.0.try_into() {
        Ok(e) => e,
        Err(e) => return HttpResponse::BadRequest().body(e),
    };
    let mut transaction: Transaction<'_, Postgres> = match pool.begin().await {
        Ok(e) => e,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(e) => e,
        Err(e) => {
            println!("insert_subscriber err");
            return HttpResponse::InternalServerError().body(e.to_string());
        }
    };
    let subscriber_token = generate_subscription_token();

    if store_token(&mut transaction, subscriber_id, &subscriber_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    if send_confirmation_email(&email_client, new_subscriber, &base_url, &subscriber_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    return HttpResponse::Ok().finish();
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
        .send_email(new_subscriber.email, "Welcomme !", &html_body, &text_body)
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
) -> Result<(), sqlx::Error> {
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
    .map_err(|e| {
        tracing::error!("Failed to execute query:{:?}", e);
        e
    })?;

    Ok(())
}
