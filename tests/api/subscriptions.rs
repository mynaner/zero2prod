use serde_json::Value;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::spawn_app;

// 替换
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let test_app = spawn_app().await;
    let body = "name=deng%20xin&email=994386502%40qq.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    let response = test_app.post_subscriptions(body.into()).await;
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email,name FROM subscriptions ")
        .fetch_one(&test_app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "994386502@qq.com");
    assert_eq!(saved.name, "deng xin");
}

#[tokio::test]
async fn subscribe_returns_a_400_for_valid_form_data() {
    let test_app = spawn_app().await;
    let test_cases = vec![
        ("name=deng%20xin", "missing the email"),
        ("email=994386502%40qq.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (body, message) in test_cases {
        let response = test_app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The api did not fial with 400 Bad request when the payload was {}.",
            message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_for_email() {
    let test_app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    let response = test_app
        .post_subscriptions("name=deng&email=994386502%40qq.com".into())
        .await;
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let test_app = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, msg) in test_cases {
        let response = test_app.post_subscriptions(body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The Api did not return a 200 ok when the payload was {}.",
            msg
        )
    }
}

#[tokio::test]
async fn subscribe_send_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;

    let body = "name=deng%20xin&email=994386508%40qq.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_send_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = "name=deng%20xin&email=994386508%40qq.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    // 获取第一个被截取的请求
    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    let body: Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
    let content_link = get_link(&body["TextBody"].as_str().unwrap());
    assert_eq!(html_link, content_link);
}
