
# 使用最新的稳定版本作为基础镜像
# 构建阶段
FROM lukemathwalker/cargo-chef:latest-rust-1.85.1 AS chef

# 把工作目录切换到 app (相当于cd app)
WORKDIR /app

# 为连接配置安装所需的系统依赖
RUN apt update && apt install lld clang -y


FROM chef as planner
# 将工作环境中的文件复制到Docker 镜像中
COPY . .

# 为项目计算出一个类似于锁的文件
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# 构建项目依赖关系,而不是我们的应用程序
RUN cargo chef cook --release --recipe-path recipe.json

# 至此,如果依赖树保持不变,那么所有的分层都应该被缓存
COPY . .

ENV SQLX_OFFLINE true


# 开始构建二进制文件
# 使用release 参数诱惑以提高sud
RUN cargo build --release --bin zero2prod

FROM debian:bookworm-slim AS runtime

WORKDIR /app


# 安装 OpenSSL -- 通过一些依赖动态连接
# 安装 ca-certificates -- 在建立HTTPS连接时,需要验证 TLS 证书
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # 清理
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# 从构建环境中复制已编译的二进制文件到运行时环境中
COPY --from=builder /app/target/release/zero2prod zero2prod
# 运行时需要的配置文件
COPY configuration configuration

ENV APP_ENVIRONMENT production

# 当执行 “docker run” 时启动二进制文件
ENTRYPOINT ["./zero2prod"]

