use argon2::Algorithm;
use argon2::Argon2;
use argon2::Params;
use argon2::PasswordHasher;
use argon2::Version;
use argon2::password_hash::SaltString;
/*
 * @Date: 2025-07-15 22:34:51
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-23 11:46:32
 * @FilePath: /zero2prod/tests/api/helpers.rs
 */
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::DatabaseSettings;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::{Application, get_connection_pool};
use zero2prod::telemetry::{get_subscriber, init_subscriber};

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    // 启动一个模拟服务器,替代PostMark API
    pub email_server: MockServer,
    pub prot: u16,
    // pub database_name: String,
    pub test_user: TestUser,
}

#[derive(Debug)]
pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }
    /**
     * 表操作-插入用户数据
     */
    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(1500, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (user_id,username,password_hash) VALUES ($1,$2,$3)
        "#,
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to create test users");
    }
}

pub struct ConfirmationLink {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLink {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| linkify::LinkKind::Url == *l.kind())
                .collect();
            assert_eq!(links.len(), 1);
            let row_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&row_link).unwrap();
            confirmation_link.set_port(Some(self.prot)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLink { html, plain_text }
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/newsletters", self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("failed to execute request.")
    }
}

// 使用 once_cell 确保 tracing 栈堆中只被初始化一次
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info";
    let subscriber_name = "test";

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

/// 启动一个新的应用程序,并运行在空的数据库之上
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;
    let configuration = {
        let mut config = get_configuration().expect("failed to read configuration");
        config.database.database_name = Uuid::new_v4().to_string();
        config.application.port = 0;
        config.email_client.base_url = email_server.uri();
        config
    };

    configure_database(&configuration.database).await;

    let application = Application::build(&configuration)
        .await
        .expect("failed to build application.");

    let address = format!("http://127.0.0.1:{}", application.port());
    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stoppend());

    let test_app = TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        prot: application_port,
        test_user: TestUser::generate(),
        // database_name: configuration.database.database_name,
    };
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("failed to create database");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("failed to run database migrations");
    connection_pool
}
