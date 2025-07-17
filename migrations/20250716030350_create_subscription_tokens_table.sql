-- Add migration script here

CREATE TABLE subscription_tokens(
    subscription_token text not null,
    subscription_id uuid not null REFERENCES subscriptions (id), -- subscription_id 是 subscription_tokens 表的外键
    PRIMARY KEY (subscription_token)
)