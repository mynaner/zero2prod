/*
 * @Date: 2025-07-17 15:18:38
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-18 10:47:25
 * @FilePath: /zero2prod/tests/api/subscriptions_confirm.rs
 */

use reqwest::Url;
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{method, path},
};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;
    let response = reqwest::get(format!("http://{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400)
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;

    let body = "name=deng%20xin&email=994386508%40qq.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;

    let emial_request = &app.email_server.received_requests().await.unwrap()[0];

    let confirmation: crate::helpers::ConfirmationLink = app.get_confirmation_links(&emial_request);

    assert_eq!(confirmation.html.host_str().unwrap(), "127.0.0.1");

    let response = reqwest::get(confirmation.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}
