# HTTP/3 证书设置指南

## 使用 mkcert 生成本地开发证书

### 1. 安装 mkcert

**Windows (使用 Chocolatey):**
```bash
choco install mkcert
```

**Windows (使用 Scoop):**
```bash
scoop bucket add extras
scoop install mkcert
```

**macOS (使用 Homebrew):**
```bash
brew install mkcert
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install libnss3-tools
wget -O mkcert https://github.com/FiloSottile/mkcert/releases/latest/download/mkcert-v*-linux-amd64
chmod +x mkcert
sudo mv mkcert /usr/local/bin/

# 或者使用包管理器
sudo apt install mkcert  # 如果可用
```

### 2. 初始化 mkcert

```bash
# 安装本地 CA
mkcert -install
```

### 3. 生成证书

在项目根目录运行：

```bash
# 生成支持 localhost、127.0.0.1 和 IPv6 的证书
mkcert localhost 127.0.0.1 ::1
```

这将生成两个文件：
- `localhost+2.pem` (证书文件)
- `localhost+2-key.pem` (私钥文件)

### 4. 配置环境变量

创建 `.env` 文件或设置环境变量：

```bash
# HTTP/3 配置
ENABLE_HTTP3=true
HTTP3_ADDR=127.0.0.1:4433
CERT_PATH=localhost+2.pem
KEY_PATH=localhost+2-key.pem
USE_SELF_SIGNED=false

# 现有配置保持不变
HTTP_ADDR=127.0.0.1:3000
DATABASE_URL=sqlite:task_manager.db?mode=rwc
JWT_SECRET=your-secret-key-change-in-production
```

### 5. 验证配置

运行应用程序：
```bash
cargo run
```

您应该看到类似以下的输出：
```
CONFIG: 正在从环境变量加载配置...
  - HTTP 地址: 127.0.0.1:3000
  - 数据库 URL: sqlite:task_manager.db?mode=rwc
  - JWT 密钥: [已设置]
  - HTTP/3 地址: 127.0.0.1:4433
  - 启用 HTTP/3: true
  - 证书路径: localhost+2.pem
  - 私钥路径: localhost+2-key.pem
  - 使用自签名证书: false
CONFIG: 配置加载完成。
CONFIG: 验证 HTTP/3 配置...
  ✓ 证书和私钥文件验证通过
CONFIG: HTTP/3 配置验证完成。
```

## 替代方案：使用自签名证书

如果您不想使用 mkcert，可以启用自签名证书：

```bash
# 环境变量配置
ENABLE_HTTP3=true
HTTP3_ADDR=127.0.0.1:4433
USE_SELF_SIGNED=true
# 不需要设置 CERT_PATH 和 KEY_PATH
```

## 测试 HTTP/3 连接

生成证书后，您可以使用支持 HTTP/3 的客户端测试连接：

```bash
# 使用 curl (如果支持 HTTP/3)
curl --http3 https://localhost:4433/api/tasks

# 使用 Chrome 浏览器
# 在地址栏输入: chrome://flags/#enable-quic
# 启用 QUIC 协议支持
```

## 注意事项

1. **开发环境专用**: mkcert 生成的证书仅适用于本地开发
2. **生产环境**: 生产环境应使用来自受信任 CA 的证书
3. **文件权限**: 确保私钥文件权限设置正确（600）
4. **版本控制**: 不要将私钥文件提交到版本控制系统

## 故障排除

如果遇到证书相关错误：

1. 检查文件路径是否正确
2. 确认文件权限
3. 验证证书有效期
4. 检查环境变量设置

```bash
# 检查证书信息
openssl x509 -in localhost+2.pem -text -noout
```
