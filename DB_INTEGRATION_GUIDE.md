# Rust Axum 项目数据库集成指南：PostgreSQL 与 DragonflyDB

你好！作为一名资深工程师，我很高兴能引导你为 `axum-tutorial` 项目集成生产级的数据库和缓存系统。本指南将带你一步步完成从环境配置到代码集成的全过程，并提供详尽的解释和最佳实践。

**目标**:
1.  **主数据库**: 将项目的数据库从 `SQLite` 升级为功能更强大的 `PostgreSQL`。
2.  **内存缓存**: 引入 `DragonflyDB` 作为高性能内存数据库（Redis 的直接替代品），用于缓存、会话管理等场景。

---

## Part 1: 技术选型与调研总结

在动手之前，我们先明确技术选型和关键信息：

*   **PostgreSQL**:
    *   **目标版本**: **PostgreSQL 17** (最新稳定版)。
    *   **Rust Crate**: 继续使用项目已有的 `sea-orm`，只需将其后端从 `sqlx-sqlite` 切换到 `sqlx-postgres`。
*   **DragonflyDB**:
    *   **目标版本**: **v1.30.0+** (最新稳定版)。
    *   **核心优势**: 与 Redis API 100% 兼容，但性能更高（利用多核优势）。
    *   **Rust Crate**: 我们将选用 `fred` 这个现代的异步 Redis 客户端。相比于传统的 `redis-rs`，`fred` 在 Axum 的并发环境（共享 `State`）中更容易使用，无需处理复杂的可变借用问题，非常适合初学者。

---

## Part 2: 开发环境配置 (Windows 10)

1.  **环境准备**:

    *   已经配置 WSL2 + Podman Desktop，
    *   已经安装 `podman-compose` 工具。

1.  **创建 `docker-compose.yml` 文件**:
    *   在你的项目根目录 `D:\ceshi\ceshi\axum-tutorial\` 下创建一个名为 `docker-compose.yml` 的文件。
    *   将以下内容复制进去：

    ```yaml
    # docker-compose.yml
    version: '3.8'

    services:
      # PostgreSQL 服务
      postgres:
        image: postgres:17-alpine  # 使用最新的PostgreSQL 17 Alpine镜像，体积更小
        container_name: axum_postgres
        environment:
          # 设置数据库的超级用户和密码，请务必修改为强密码
          POSTGRES_USER: myuser
          POSTGRES_PASSWORD: mysecretpassword
          POSTGRES_DB: axum_chat_db # 创建一个默认的数据库
        ports:
          # 将容器的5432端口映射到你主机的5432端口
          - "5432:5432"
        volumes:
          # 将数据库数据持久化到本地，防止容器删除后数据丢失
          - postgres_data:/var/lib/postgresql/data
        restart: unless-stopped

      # DragonflyDB 服务
      dragonfly:
        image: docker.dragonflydb.io/dragonflydb/dragonfly:latest # 使用官方最新镜像
        container_name: axum_dragonfly
        ports:
          # 将容器的6379端口映射到你主机的6379端口 (Redis协议)
          - "6379:6379"
        volumes:
          # 将DragonflyDB数据持久化
          - dragonfly_data:/data
        restart: unless-stopped
        # DragonflyDB 推荐的内核设置，可提升性能
        ulimits:
          memlock: -1
          nofile:
            soft: 65535
            hard: 65535

    volumes:
      # 定义数据卷
      postgres_data:
      dragonfly_data:
    ```

2.  **启动服务**:
    *   打开你的终端 (PowerShell 或 CMD)。
    *   进入项目根目录 `D:\ceshi\ceshi\axum-tutorial\`。
    *   运行命令：`podman-compose up -d`
    *   这个命令会以后台模式（`-d`）下载并启动 PostgreSQL 和 DragonflyDB 容器。你可以在 Podman Desktop 的界面中看到它们的状态。

3.  **连接信息**:
    *   **PostgreSQL 连接字符串**:
        ```
        postgres://myuser:mysecretpassword@localhost:5432/axum_chat_db
        ```
    *   **DragonflyDB/Redis 连接字符串**:
        ```
        redis://localhost:6379
        ```



## Part 3: 项目集成实现

现在，我们开始修改 Rust 代码，将数据库集成到你的 Axum 应用中。

### 1. 依赖配置 (`Cargo.toml`)

打开 `Cargo.toml` 文件，进行以下修改：

```toml
# D:\ceshi\ceshi\axum-tutorial\Cargo.toml

[dependencies]
# ... 其他依赖保持不变 ...

# SeaORM - 将特性从 sqlite 修改为 postgres
# 注意：我们移除了 "sqlx-sqlite" 并添加了 "sqlx-postgres"
sea-orm = { version = "1.1.12", features = ["sqlx-postgres", "runtime-tokio-native-tls", "mock", "macros"] }

# 新增 DragonflyDB/Redis 客户端
fred = "6.3.0" # 推荐使用较新版本

# ... 其他依赖保持不变 ...
```

修改后，在终端运行 `cargo build` 或 `cargo check` 来下载新的依赖。

### 2. 数据库连接模块

集中管理数据库连接，

**重要**: 你需要在项目根目录创建一个 `.env` 文件来存放你的连接字符串，**不要将密码硬编码在代码中**。

```.env
# D:\ceshi\ceshi\axum-tutorial\.env

# 使用方案A (Podman) 的连接字符串
DATABASE_URL=postgres://myuser:mysecretpassword@localhost:5432/axum_chat_db
REDIS_URL=redis://localhost:6379
```

### 4. 在处理器中使用数据库

现在你可以在任何 Axum 处理器中通过 `State` 提取器来访问数据库了。

