/*
 * @Date: 2025-07-13 11:27:17
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-23 09:47:24
 * @FilePath: /zero2prod/src/telemetry.rs
 */
use tokio::task::JoinHandle;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter, layer::SubscriberExt};

/// 将多个层次组合成 tracing 的订阅器
/// 将  impl Subscriber 作为返回值的类型,以避免写出繁琐的真实类型
///
pub fn get_subscriber<Sink>(
    name: &str,
    env_filter: &str,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    // 如果没有设置 RUST_LOG 环境变量,则输出所有 env_filter 及以上级别的跨度
    let evn_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    let formatting = BunyanFormattingLayer::new(
        name.into(),
        // 将日志输出到 stdout
        sink,
    );
    // with 方法由 SubscriberExt 提供,可以扩展 tracing_subscriber 的 Subscriber
    Registry::default()
        .with(evn_filter)
        .with(JsonStorageLayer)
        .with(formatting)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("failed to set Logger");
    // 可以用于指定处理跨度订阅器
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn spawn_blocking_with_tractiong<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
