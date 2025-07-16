 
-- migrations/{timestamp}_create_subscriptions_table.sql
-- 创建 subscriptions 表
create table subscriptions(
    id uuid not null, 
    primary key (id), 
    email text not null unique,
    name text not null,
    subscribed_at timestamptz not null
)

-- uuid  
-- unique 唯一
-- not null 不为空