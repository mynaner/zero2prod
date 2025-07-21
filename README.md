<!--
 * @Date: 2025-07-16 09:51:52
 * @LastEditors: myclooe 994386508@qq.com
 * @LastEditTime: 2025-07-18 22:38:27
 * @FilePath: /zero2prod/README.md
-->




# 生成新的迁移脚本

```sh
sqlx migrate add make_status_no_null_in_subscriptions
```



```
export RUST_LOG="sqlx=error,info"
export TEST_LOG=enabled
cargo t subscribe_fails_if_there_is_a_fatal_database_error
```