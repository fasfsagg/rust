# 本地数据库环境配置指南：PostgreSQL & DragonflyDB

本指南将帮助你在 Windows 10 本地开发环境中，使用 Docker 快速搭建 PostgreSQL 和 DragonflyDB 数据库，为你的 Axum 项目提供数据库支持。

## 1. 前提条件

在开始之前，请确保你的 Windows 10 系统已安装以下软件：

*   **Docker Desktop**：Docker 是一个开源的应用容器引擎，可以让你轻松地部署和运行应用程序。访问 [Docker 官网](https://www.docker.com/products/docker-desktop/) 下载并安装 Docker Desktop。安装完成后，请确保 Docker Desktop 正在运行。
*   **Git Bash 或 PowerShell**：用于执行命令行操作。

## 2. 启动 PostgreSQL 和 DragonflyDB

我们将使用 Docker Compose 来一键启动 PostgreSQL 和 DragonflyDB。

### 步骤 2.1：创建 `docker-compose.yml` 文件

在你的项目根目录 `D:\ceshi\ceshi\axum-tutorial\` 下，创建一个名为 `docker-compose.yml` 的文件，并复制以下内容：

```yaml
version: '3.8'
services:
  postgres:
    image: postgres:16-alpine # 使用轻量级的 PostgreSQL 16 版本
    restart: always
    environment:
      POSTGRES_USER: axum_user # 数据库用户名
      POSTGRES_PASSWORD: axum_password # 数据库密码
      POSTGRES_DB: axum_tutorial # 数据库名称
    ports:
      - "5432:5432" # 将容器的 5432 端口映射到主机的 5432 端口
    volumes:
      - postgres_data:/var/lib/postgresql/data # 数据持久化卷，防止容器删除后数据丢失

  dragonflydb:
    image: docker.dragonflydb.io/dragonflydb/dragonfly:latest # 使用最新版 DragonflyDB
    restart: always
    ports:
      - "6379:6379" # 将容器的 6379 端口映射到主机的 6379 端口
    command: ["dragonfly", "--maxmemory", "1gb"] # 启动命令，限制 DragonflyDB 最大内存为 1GB

volumes:
  postgres_data: # 定义数据卷
```

**说明**：
*   `postgres` 服务：
    *   `image: postgres:16-alpine`：指定使用 PostgreSQL 16 的 Alpine Linux 版本，它更小巧。
    *   `environment`：设置 PostgreSQL 的用户名、密码和数据库名称。**请记住这些信息，你的 Axum 项目将使用它们来连接数据库。**
    *   `ports: - "5432:5432"`：将容器内部的 PostgreSQL 默认端口 5432 映射到你本地机器的 5432 端口。
    *   `volumes: - postgres_data:/var/lib/postgresql/data`：确保 PostgreSQL 的数据在容器重启或删除后不会丢失。
*   `dragonflydb` 服务：
    *   `image: docker.dragonflydb.io/dragonflydb/dragonfly:latest`：指定使用最新版的 DragonflyDB 镜像。
    *   `ports: - "6379:6379"`：将容器内部的 DragonflyDB 默认端口 6379 映射到你本地机器的 6379 端口。
    *   `command: ["dragonfly", "--maxmemory", "1gb"]`：启动 DragonflyDB 并限制其最大内存使用量为 1GB。

### 步骤 2.2：启动 Docker 容器

打开你的 Git Bash 或 PowerShell，导航到项目根目录 `D:\ceshi\ceshi\axum-tutorial\`，然后执行以下命令：

```bash
docker-compose up -d
```

*   `docker-compose up`：根据 `docker-compose.yml` 文件启动所有服务。
*   `-d`：表示在后台运行（detached mode），这样你就可以继续使用命令行。

首次运行此命令时，Docker 会下载 PostgreSQL 和 DragonflyDB 的镜像，这可能需要一些时间，具体取决于你的网络速度。

### 步骤 2.3：验证容器是否成功启动

执行以下命令，检查容器的运行状态：

```bash
docker-compose ps
```

你应该会看到 `postgres` 和 `dragonflydb` 两个服务的状态显示为 `Up`。

## 3. 配置 Axum 项目的 `.env` 文件

你的 Axum 项目需要知道如何连接到这些数据库。你需要在项目根目录下的 `.env` 文件中配置数据库连接字符串。

### 步骤 3.1：编辑 `.env` 文件

打开你项目根目录 `D:\ceshi\ceshi\axum-tutorial\` 下的 `.env` 文件（如果不存在，请创建一个）。添加或修改以下行：

```dotenv
# PostgreSQL 数据库连接字符串
# 请确保这里的用户名、密码、数据库名与 docker-compose.yml 中设置的一致
DATABASE_URL_POSTGRES="postgresql://axum_user:axum_password@localhost:5432/axum_tutorial"

# DragonflyDB 连接字符串 (兼容 Redis 协议)
DRAGONFLYDB_URL="redis://localhost:6379/"
```

**重要提示**：
*   `DATABASE_URL_POSTGRES`：这是你的 Axum 项目连接 PostgreSQL 的 URL。
    *   `axum_user` 和 `axum_password` 应该与你在 `docker-compose.yml` 中为 PostgreSQL 设置的 `POSTGRES_USER` 和 `POSTGRES_PASSWORD` 保持一致。
    *   `localhost:5432`：表示 PostgreSQL 运行在你本地机器的 5432 端口。
    *   `axum_tutorial`：表示连接到名为 `axum_tutorial` 的数据库。
*   `DRAGONFLYDB_URL`：这是你的 Axum 项目连接 DragonflyDB 的 URL。
    *   `localhost:6379`：表示 DragonflyDB 运行在你本地机器的 6379 端口。

## 4. 验证数据库连接（可选，但推荐）

### 4.1 验证 PostgreSQL 连接

你可以使用 `psql` 命令行工具（如果已安装）或任何图形化数据库客户端（如 DBeaver, pgAdmin）来连接 PostgreSQL。

使用 `psql`：

```bash
psql -h localhost -p 5432 -U axum_user -d axum_tutorial
```

输入密码 `axum_password` 后，如果成功连接，你将看到 `axum_tutorial=#` 的提示符。输入 `\q` 退出。

### 4.2 验证 DragonflyDB 连接

你可以使用 `redis-cli` 命令行工具（如果已安装）来连接 DragonflyDB。

```bash
redis-cli -h localhost -p 6379
```

如果成功连接，你将看到 `localhost:6379>` 的提示符。你可以尝试一些 Redis 命令，例如 `PING`，它应该返回 `PONG`。输入 `QUIT` 退出。

## 5. 后续步骤

完成以上步骤后，你的本地开发环境就已经准备就绪，可以开始进行 Axum 项目的代码修改，以实现从 SQLite 到 PostgreSQL + DragonflyDB 的数据库迁移了。

在进行代码修改时，你的 Axum 项目将能够通过 `.env` 文件中配置的 URL 连接到这些本地运行的数据库。
