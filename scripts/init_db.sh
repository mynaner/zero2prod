#! /usr/bin/env bash 
###
 # @Date: 2025-07-12 10:53:02
 # @LastEditors: myclooe 994386508@qq.com
 # @LastEditTime: 2025-07-14 14:49:36
 # @FilePath: /zero2prod/scripts/init_db.sh
### 

set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
    echo >&2 "Error:psql is not installed"
    exit 1
fi


if ! [ -x "$(command -v sqlx)" ]; then
    echo >&2 "Error sqlx is not installed"
    echo >&2 "Use:"
    echo >&2 "cargo install --version=0.6.0 sqlx-cli --no-default-features --features postgres"
    echo >&2 "to install it."
    exit 1
fi 


# 检测是否设置自定义用户名,如未设置,则默认是 “postgres”
DB_USER=${POSTGRES_USER:=postgres}
# 检测是否已设置自定义密码.如果未设置,这默认是 password
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"


DB_NAME="${POSTGRES_DB:=newsletter}"

DB_PORT="${POSTGRES_PORT:=5432}"

if [[ -z "${SKIP_DOCKER}" ]]; then
    docker run \
        -e POSTGRES_USER=${DB_USER} \
        -e POSTGRES_PASSWORD=${DB_PASSWORD} \
        -e POSTGRES_DB=${DB_NAME} \
        -p "${DB_PORT}":5432 \
        -d postgres \
        postgres -N 1000
fi 



export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c "\q"; do  
    >&2 echo "Postgres is still unavailable - sleeping" 
    sleep 1
done

>&2 echo "Postgres is up and runing on port ${DB_PORT}! - running migrations now!"

# export DATABASE_URL=postgres://postgres:password@localhost:5432/newsletter
export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@localhost:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run


>&2 echo "Postgres has been migrated, ready to go!"