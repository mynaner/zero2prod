-- Add migration script here

-- 将整个迁移过程放入事物中,以确保原子化的成功或失败
-- sqlx 不会自动帮我们进行原子化处理
BEGIN;
    -- 为历史记录回填 status
    update subscriptions
        set status = 'confirmed'
        WHERE status IS NULL;
    -- 让 status 不为空
    ALTER TABLE subscriptions ALTER COLUMN status set not null;
COMMIT;