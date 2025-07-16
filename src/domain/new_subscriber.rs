use crate::domain::{SubScriberName, subscriber_email::SubscriberEmail};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubScriberName,
}
