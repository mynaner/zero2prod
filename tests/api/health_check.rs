use crate::helpers::spawn_app;

/*
 * @Date: 2025-07-15 22:34:32
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-16 10:38:55
 * @FilePath: /zero2prod/tests/api/health_check.rs
 */
#[tokio::test]
async fn health_check_works() {
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http:/{}/health_check", test_app.address))
        .send()
        .await
        .expect("Failed to execute request");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
