use actix_web::{
    HttpRequest, HttpResponse, ResponseError,
    http::{StatusCode, header::HeaderMap},
    web,
};

use anyhow::Context;
use base64::Engine;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::subscriptions::error_chain_fmt,
};

#[derive(Debug, Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug, Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

pub async fn publish_newsletters(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    requset: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(requset.headers());

    let subscribers = get_confirmed_subscriber(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to send newsletter issue to {}",
                            subscriber.email.as_ref()
                        )
                    })?;
            }
            Err(err) => {
                tracing::warn!(error=?err,"Skipping a confirmed subscriber \
                hteir stored contact details are invalid");
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

struct Credentials {
    username: String,
    password: SecretString,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes: Vec<u8> = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decode_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credentials tring is not valid utf8.")?;

    let mut credentials = decode_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'basic' auth."))?
        .to_string();

    let passowrd = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'basic' auth."))?
        .to_string();

    Ok(Credentials {
        username,
        password: secrecy::SecretString::new(passowrd.into()),
    })
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscriber(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    // struct Row {
    //     email: String,
    // }

    let confirmed_subscribers =
        sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#,)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|e| match SubscriberEmail::parse(e.email) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(e) => Err(anyhow::anyhow!(e)),
            })
            .collect();

    // let mut list = vec![];

    // for data in rows {
    //     let email = SubscriberEmail::parse(data.email).unwrap();
    //     list.push(ConfirmedSubscriber { email: email });
    // }

    // Ok(list)

    // let confirmed_subscribers = rows
    //     .into_iter()
    //     .map(|e| match SubscriberEmail::parse(e.email) {
    //         Ok(email) => Ok(ConfirmedSubscriber { email }),
    //         Err(err) => Err(anyhow::anyhow!(err)),
    //     })
    //     .collect();
    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
