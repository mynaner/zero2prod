use actix_web::{
    HttpRequest, HttpResponse, ResponseError,
    http::{
        StatusCode,
        header::{self, HeaderMap, HeaderValue},
    },
    web,
};

use anyhow::Context;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use base64::Engine;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::subscriptions::error_chain_fmt,
    telemetry::spawn_blocking_with_tractiong,
};

#[derive(Debug, Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Debug, Deserialize)]
pub struct Content {
    text: String,
    html: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publish a newsletters issue",
    skip(body, pool, email_client, request)
    fields(username=tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn publish_newsletters(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &pool).await?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
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
#[derive(Debug)]
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
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            Self::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            Self::UnexpectedError(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
            Self::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ELAH/Jh1Hw$\
        CWOrko070JBQ/iyh7uJ0L02aLEfrHWTWLLSAxT0zRno",
    );

    if let Some((store_user_id, store_password_hash)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(PublishError::UnexpectedError)?
    {
        user_id = Some(store_user_id);
        expected_password_hash = store_password_hash;
    };

    spawn_blocking_with_tractiong(|| {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking taks")
    .map_err(PublishError::UnexpectedError)??;
    user_id.ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unkown username")))
}

async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
         SELECT user_id,password_hash from users WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to preform a query to validate auth credentials.")?
    .map(|row| (row.user_id, SecretString::new(row.password_hash.into())));

    Ok(row)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_passowrd_hash, password_hash)
)]
fn verify_password_hash(
    expected_passowrd_hash: SecretString,
    password_hash: SecretString,
) -> Result<(), PublishError> {
    let expected_password_hash = PasswordHash::new(&expected_passowrd_hash.expose_secret())
        .context("Failed to PasswordHash ")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            password_hash.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Invalid passowrd")
        .map_err(PublishError::AuthError)
}
