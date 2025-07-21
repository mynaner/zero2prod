/*
 * @Date: 2025-07-15 10:36:14
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-20 22:18:37
 * @FilePath: /zero2prod/src/email_client.rs
 */

use core::str;

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization: SecretString,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization: SecretString,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            base_url,
            sender,
            http_client: Client::builder().timeout(timeout).build().unwrap(),
            authorization,
        }
    }
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let send_email_request = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            html_body: html_content,
            text_body: text_content,
            subject: subject,
        };
        self.http_client
            .post(url)
            .header(
                "x-Postmark-Server-Token",
                self.authorization.expose_secret(),
            )
            .json(&send_email_request)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use claim::{assert_err, assert_ok};
    use fake::{
        Fake,
        faker::{internet::en::SafeEmail, lorem::en::Sentence},
    };
    use secrecy::SecretString;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{any, header, header_exists, method, path},
    };

    use crate::{domain::SubscriberEmail, email_client::EmailClient};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, serde_json::Error> =
                serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                dbg!(&body);
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Sentence(1..10).fake()
    }
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(uri: String) -> EmailClient {
        EmailClient::new(
            uri,
            email(),
            SecretString::new(Sentence(3..10).fake::<String>().into()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_emial_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;

        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new("500"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_tasks_too_long() {
        let mock_server = MockServer::start().await;

        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new("200").set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_err!(outcome);
    }
}
