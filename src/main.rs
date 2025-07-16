/*
 * @Date: 2025-07-11 21:15:33
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-15 23:08:49
 * @FilePath: /zero2prod/src/main.rs
 */
use zero2prod::{
    configuration::get_configuration,
    startup::Application,
    telemetry::{get_subscriber, init_subscriber},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod", "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let config = get_configuration().expect("Failed to read configuration .");
    let application = Application::build(&config).await?;
    application.run_until_stoppend().await?;
    Ok(())
}
