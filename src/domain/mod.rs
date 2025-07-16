/*
 * @Date: 2025-07-14 23:26:40
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-14 23:35:32
 * @FilePath: /zero2prod/src/domain/mod.rs
 */
mod new_subscriber;
mod subscriber_email;
mod subscriber_name;

pub use new_subscriber::NewSubscriber;
pub use subscriber_email::SubscriberEmail;
pub use subscriber_name::SubScriberName;
