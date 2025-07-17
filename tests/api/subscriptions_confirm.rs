/*
 * @Date: 2025-07-17 15:18:38
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-17 15:39:47
 * @FilePath: /zero2prod/tests/api/subscriptions_confirm.rs
 */

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(format!("http://{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400)
}
