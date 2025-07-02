# Podman 替代 Docker 方案指南

本指南将介绍 Podman 作为 Docker 的替代方案，并提供在 Windows 10 环境下使用 Podman 搭建 PostgreSQL 和 DragonflyDB 数据库的步骤。

## 1. 为什么考虑 Podman？

Podman 是一个由 Red Hat 主导开发的开源容器引擎，它与 Docker 兼容，但具有一些独特的优势，使其成为 Docker 的有力替代品：

*   **无守护进程 (Daemonless)**：这是 Podman 最大的特点。Docker 依赖于一个常驻的守护进程 (Docker Daemon) 来管理容器，而 Podman 不需要。这意味着：
    *   **更高的安全性**：没有守护进程意味着没有一个特权进程始终运行在后台，减少了攻击面。
    *   **更少的资源占用**：在没有运行容器时，Podman 不会占用系统资源。
    *   **更好的集成性**：可以直接通过 `systemd` 或其他进程管理器管理容器，无需额外的守护进程。
*   **无根容器 (Rootless Containers)**：Podman 允许用户以非特权用户身份运行容器，这进一步增强了安全性，避免了容器逃逸时对宿主系统的潜在危害。
*   **与 Docker CLI 兼容**：Podman 的命令行接口与 Docker CLI 高度兼容，这意味着你可以继续使用大部分熟悉的 Docker 命令（例如 `podman run` 替代 `docker run`，`podman ps` 替代 `docker ps`）。
*   **Pod (容器组)** 概念：Podman 原生支持 Kubernetes 的 Pod 概念，允许你将多个容器作为一个逻辑单元进行管理，这对于学习和实践 Kubernetes 概念非常有帮助。
*   **Windows 上的 WSL2 集成**：在 Windows 上，Podman 通常通过 WSL2 (Windows Subsystem for Linux 2) 运行，提供了良好的性能和集成度。

## 2. 在 Windows 10 上安装 Podman

在 Windows 10 上安装 Podman 的推荐方式是通过 WSL2。

### 步骤 2.1：确保 WSL2 已安装和启用

如果你还没有安装 WSL2，请按照微软官方文档进行安装：
[https://learn.microsoft.com/zh-cn/windows/wsl/install](https://learn.microsoft.com/zh-cn/windows/wsl/install)

安装完成后，确保你已安装一个 Linux 发行版（如 Ubuntu），并将其设置为 WSL2 模式。

### 步骤 2.2：在 WSL2 中安装 Podman

打开你的 WSL2 Linux 发行版终端（例如 Ubuntu），然后执行以下命令来安装 Podman。

**对于 Ubuntu/Debian 系统：**

```bash
sudo apt update
sudo apt install podman
```

**对于 Fedora/CentOS/RHEL 系统：**

```bash
sudo dnf install podman
```

### 步骤 2.3：初始化 Podman 虚拟机 (可选，但推荐)

在 WSL2 中，Podman 可以直接运行，但为了更好的兼容性和网络隔离，可以初始化一个 Podman 虚拟机。

```bash
podman machine init
podman machine start
```

### 步骤 2.4：安装 `podman-compose`

`podman-compose` 是一个 Python 脚本，它允许你使用与 `docker-compose` 兼容的 `docker-compose.yml` 文件来管理 Podman 容器。

在你的 WSL2 Linux 终端中安装 `podman-compose`：

```bash
sudo apt install python3-pip # 如果没有安装 pip
pip3 install podman-compose
```

## 3. 使用 Podman 启动 PostgreSQL 和 DragonflyDB

现在，你可以使用之前为 Docker Compose 创建的 `docker-compose.yml` 文件来启动数据库服务。

### 步骤 3.1：确保 `docker-compose.yml` 文件存在

请确保你的项目根目录 `D:\ceshi\ceshi\axum-tutorial\` 下存在之前创建的 `docker-compose.yml` 文件。

### 步骤 3.2：在 WSL2 中启动容器

打开你的 WSL2 Linux 终端，导航到你的项目目录（例如，如果你的项目在 Wind
ows 的 `D:\ceshi\ceshi\axum-tutorial`，在 WSL2 中它可能映射为 `/mnt/d/ceshi/ceshi/axum-tutorial`）。

然后执行以下命令：

```bash
podman-compose up -d
```

*   `podman-compose up`：根据 `docker-compose.yml` 文件启动所有服务。
*   `-d`：表示在后台运行。

首次运行此命令时，Podman 会下载 PostgreSQL 和 DragonflyDB 的镜像，这可能需要一些时间。

### 步骤 3.3：验证容器是否成功启动

执行以下命令，检查容器的运行状态：

```bash
podman-compose ps
```

你应该会看到 `postgres` 和 `dragonflydb` 两个服务的状态显示为 `running`。

你也可以使用 `podman ps` 来查看所有正在运行的 Podman 容器。

## 4. 配置 Axum 项目的 `.env` 文件

无论你使用 Docker 还是 Podman，你的 Axum 项目连接数据库的方式是相同的。你只需要确保 `.env` 文件中的数据库连接字符串指向正确的地址和端口。

### 步骤 4.1：编辑 `.env` 文件

打开你项目根目录 `D:\ceshi\ceshi\axum-tutorial\` 下的 `.env` 文件。添加或修改以下行：

```dotenv
# PostgreSQL 数据库连接字符串
# 请确保这里的用户名、密码、数据库名与 docker-compose.yml 中设置的一致
DATABASE_URL_POSTGRES="postgresql://axum_user:axum_password@localhost:5432/axum_tutorial"

# DragonflyDB 连接字符串 (兼容 Redis 协议)
DRAGONFLYDB_URL="redis://localhost:6379/"
```

**重要提示**：
*   `localhost:5432` 和 `localhost:6379`：在 Windows 上通过 WSL2 运行 Podman 时，通常可以直接通过 `localhost` 访问容器映射的端口。

## 5. 验证数据库连接（可选，但推荐）

验证方法与 Docker 类似。

### 5.1 验证 PostgreSQL 连接

在 WSL2 终端中，你可以使用 `psql` 命令行工具：

```bash
psql -h localhost -p 5432 -U axum_user -d axum_tutorial
```

输入密码 `axum_password` 后，如果成功连接，你将看到 `axum_tutorial=#` 的提示符。输入 `\q` 退出。

### 5.2 验证 DragonflyDB 连接

在 WSL2 终端中，你可以使用 `redis-cli` 命令行工具：

```bash
redis-cli -h localhost -p 6379
```

如果成功连接，你将看到 `localhost:6379>` 的提示符。你可以尝试一些 Redis 命令，例如 `PING`，它应该返回 `PONG`。输入 `QUIT` 退出。

## 6. Podman 与 Docker 的主要区别和注意事项

*   **守护进程**：Podman 无守护进程，Docker 有。这意味着 Podman 容器的生命周期与启动它的终端会话相关联（除非你使用 `podman generate systemd` 来创建 `systemd` 服务）。
*   **Rootless**：Podman 默认支持无根容器，而 Docker 需要额外配置。
*   **`docker.sock`**：Docker 依赖 `/var/run/docker.sock` 进行通信，Podman 使用不同的机制（通常是 REST API 或直接调用）。
*   **`podman-compose` vs `docker-compose`**：虽然 `podman-compose` 旨在兼容 `docker-compose.yml` 文件，但它是一个独立的工具，可能存在一些细微的行为差异或不支持的特性。对于大多数常见用例，它应该能正常工作。
*   **资源管理**：由于无守护进程，Podman 在不运行容器时资源占用更少。

## 7. 后续步骤

完成以上步骤后，你的本地开发环境就已经准备就绪，可以开始进行 Axum 项目的代码修改，以实现从 SQLite 到 PostgreSQL + DragonflyDB 的数据库迁移了。
  