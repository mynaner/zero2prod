use wiremock::{
    Mock, ResponseTemplate,
    matchers::{any, method, path},
};

use crate::helpers::{ConfirmationLink, TestApp, spawn_app};

/*
 * @Date: 2025-07-20 17:12:01
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-23 10:42:01
 * @FilePath: /zero2prod/tests/api/newsletter.rs
 */

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let newsletter_request_body = serde_json::json!({
        "title":"newsLetter title",
        "content":{
            "text":"NewsLetter body as plain text",
            "html":"<p>NewsLetter body as html</p>"
        }
    });

    let response = app.post_newsletters(newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200)
}
#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;
    let newsletter_request_body = serde_json::json!({
        "title":"newsLetter title",
        "content":{
            "text":"NewsLetter body as plain text",
            "html":"<p>NewsLetter body as html</p>"
        }
    });

    let response = app.post_newsletters(newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200)
}

/// 使用测试程序的公共api 创建一个未确定的订阅者
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLink {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _moke_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        // mount_as_scoped 确保两个mock不会重叠
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    let test_case = vec![
        (
            serde_json::json!({
                "content":{
                    "text":"",
                    "html":"",
                },

            }),
            "messing title",
        ),
        (
            serde_json::json!({"title":"Newsletter!"}),
            "message content",
        ),
    ];

    for (invalid_body, error_message) in test_case {
        let response = app.post_newsletters(invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "the API did not fail with 400 Bad request when the payload was {}",
            error_message
        )
    }
}
#[tokio::test]
async fn request_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", app.address))
        .json(&serde_json::json!({
            "title":"Newsletters title",
            "content":{
                "text":"Newsletter body as plain text",
                "html":"<p>Newsletter body as html</p>"
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["www-Authenticate"]
    )
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;
    let username = uuid::Uuid::new_v4();
    let password = uuid::Uuid::new_v4();
    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title":"NewsLetter title",
            "content":{
                "text":"Newsletter body as plain text",
                "html":"<p>newsletter body as HTML</p>"
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let app = spawn_app().await;

    let username = &app.test_user.username;
    let password = uuid::Uuid::new_v4().to_string();
    assert_ne!(app.test_user.password, password);

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title":"NewsLetter title",
            "content":{
                "text":"Newsletter body as plain text",
                "html":"<p>newsletter body as HTML</p>"
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}
