use core::str;
/*
 * @Date: 2025-07-16 10:45:18
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-18 15:58:49
 * @FilePath: /zero2prod/src/startup.rs
 */
use std::net::TcpListener;

use actix_web::{App, HttpServer, dev::Server, web};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing_actix_web::TracingLogger;

use crate::{
    configuration::{self, DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{
        health_check::health_check, subscriptions::subscribe, subscriptions_confirm::confirm,
    },
};

pub struct Application {
    port: u16,
    server: Server,
}

pub struct ApplicationBaseUrl(pub String);

impl Application {
    pub async fn build(config: &Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&config.database);

        let sender = config
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let timeout = config.email_client.timeout();
        let email_client = EmailClient::new(
            config.email_client.base_url.clone(),
            sender,
            config.email_client.authorization_token.clone(),
            timeout,
        );

        let address = format!("{}:{}", config.application.host, config.application.port);
        let listener = TcpListener::bind(address)?;

        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            config.application.base_url.clone(),
        )?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stoppend(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(database_config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(database_config.with_db())
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
