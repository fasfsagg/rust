# JWT 令牌详细验证测试
Write-Host "=== JWT 令牌详细验证测试 ===" -ForegroundColor Green

$baseUrl = "http://127.0.0.1:3000"
$testUsername = "jwttest$(Get-Random -Minimum 1000 -Maximum 9999)"
$testPassword = "password123"

# 1. 注册并登录获取 JWT
Write-Host "1. 注册并登录获取 JWT 令牌..." -ForegroundColor Cyan

$registerBody = @{
    username = $testUsername
    password = $testPassword
    confirmPassword = $testPassword
} | ConvertTo-Json

try {
    $registerResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method POST -Body $registerBody -ContentType "application/json"
    Write-Host "✓ 注册成功: $($registerResponse.user.username)" -ForegroundColor Green
} catch {
    Write-Host "✗ 注册失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

$loginBody = @{
    username = $testUsername
    password = $testPassword
} | ConvertTo-Json

try {
    $loginResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/login" -Method POST -Body $loginBody -ContentType "application/json"
    Write-Host "✓ 登录成功" -ForegroundColor Green
    
    $jwtToken = $loginResponse.access_token
    Write-Host "JWT 令牌长度: $($jwtToken.Length)" -ForegroundColor Yellow
    Write-Host "令牌类型: $($loginResponse.token_type)" -ForegroundColor Yellow
    Write-Host "过期时间: $($loginResponse.expires_in) 秒" -ForegroundColor Yellow
    
    # 验证 JWT 令牌格式（应该有3个部分，用.分隔）
    $tokenParts = $jwtToken.Split('.')
    if ($tokenParts.Length -eq 3) {
        Write-Host "✓ JWT 令牌格式正确 (3个部分)" -ForegroundColor Green
        Write-Host "  Header: $($tokenParts[0].Substring(0, [Math]::Min(20, $tokenParts[0].Length)))..." -ForegroundColor Gray
        Write-Host "  Payload: $($tokenParts[1].Substring(0, [Math]::Min(20, $tokenParts[1].Length)))..." -ForegroundColor Gray
        Write-Host "  Signature: $($tokenParts[2].Substring(0, [Math]::Min(20, $tokenParts[2].Length)))..." -ForegroundColor Gray
    } else {
        Write-Host "✗ JWT 令牌格式错误" -ForegroundColor Red
    }
    
} catch {
    Write-Host "✗ 登录失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 2. 测试使用 JWT 访问受保护资源
Write-Host "`n2. 测试使用 JWT 访问受保护资源..." -ForegroundColor Cyan

$authHeaders = @{
    "Authorization" = "Bearer $jwtToken"
}

try {
    $tasksResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET -Headers $authHeaders
    Write-Host "✓ 成功访问任务列表" -ForegroundColor Green
    Write-Host "  任务数量: $($tasksResponse.Count)" -ForegroundColor Yellow
} catch {
    Write-Host "✗ 访问任务列表失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 3. 测试创建任务（需要认证）
Write-Host "`n3. 测试创建任务（需要认证）..." -ForegroundColor Cyan

$taskBody = @{
    title = "JWT 测试任务"
    description = "这是一个通过 JWT 认证创建的测试任务"
    completed = $false
} | ConvertTo-Json

try {
    $createTaskResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method POST -Body $taskBody -Headers $authHeaders -ContentType "application/json"
    Write-Host "✓ 成功创建任务" -ForegroundColor Green
    Write-Host "  任务ID: $($createTaskResponse.id)" -ForegroundColor Yellow
    Write-Host "  任务标题: $($createTaskResponse.title)" -ForegroundColor Yellow
    
    $taskId = $createTaskResponse.id
} catch {
    Write-Host "✗ 创建任务失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 4. 测试无效 JWT 令牌
Write-Host "`n4. 测试无效 JWT 令牌..." -ForegroundColor Cyan

$invalidHeaders = @{
    "Authorization" = "Bearer invalid.jwt.token"
}

try {
    Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET -Headers $invalidHeaders
    Write-Host "✗ 无效令牌应该被拒绝" -ForegroundColor Red
} catch {
    Write-Host "✓ 无效令牌正确被拒绝" -ForegroundColor Green
}

# 5. 测试缺少 Authorization 头
Write-Host "`n5. 测试缺少 Authorization 头..." -ForegroundColor Cyan

try {
    Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET
    Write-Host "✗ 缺少认证头应该被拒绝" -ForegroundColor Red
} catch {
    Write-Host "✓ 缺少认证头正确被拒绝" -ForegroundColor Green
}

# 6. 测试错误的 Authorization 格式
Write-Host "`n6. 测试错误的 Authorization 格式..." -ForegroundColor Cyan

$wrongFormatHeaders = @{
    "Authorization" = "Basic $jwtToken"  # 应该是 Bearer，不是 Basic
}

try {
    Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET -Headers $wrongFormatHeaders
    Write-Host "✗ 错误格式应该被拒绝" -ForegroundColor Red
} catch {
    Write-Host "✓ 错误格式正确被拒绝" -ForegroundColor Green
}

# 7. 验证 JWT 令牌包含正确的用户信息
Write-Host "`n7. 验证 JWT 令牌包含正确的用户信息..." -ForegroundColor Cyan

# 解码 JWT payload（Base64URL 解码）
try {
    $payload = $tokenParts[1]
    # 添加必要的填充
    while ($payload.Length % 4 -ne 0) {
        $payload += "="
    }
    # 替换 URL-safe 字符
    $payload = $payload.Replace('-', '+').Replace('_', '/')

    $decodedBytes = [System.Convert]::FromBase64String($payload)
    $decodedJson = [System.Text.Encoding]::UTF8.GetString($decodedBytes)
    $claims = $decodedJson | ConvertFrom-Json

    Write-Host "✓ JWT 令牌解码成功" -ForegroundColor Green
    Write-Host "  用户ID (sub): $($claims.sub)" -ForegroundColor Yellow
    Write-Host "  用户名: $($claims.username)" -ForegroundColor Yellow
    Write-Host "  签发时间 (iat): $($claims.iat)" -ForegroundColor Yellow
    Write-Host "  过期时间 (exp): $($claims.exp)" -ForegroundColor Yellow

    # 验证用户名是否匹配
    if ($claims.username -eq $testUsername) {
        Write-Host "✓ JWT 中的用户名匹配" -ForegroundColor Green
    } else {
        Write-Host "✗ JWT 中的用户名不匹配" -ForegroundColor Red
    }

    # 验证过期时间是否合理（应该是未来的时间）
    $currentTime = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    if ($claims.exp -gt $currentTime) {
        Write-Host "✓ JWT 过期时间设置正确" -ForegroundColor Green
        $timeToExpire = $claims.exp - $currentTime
        Write-Host "  距离过期还有: $timeToExpire 秒" -ForegroundColor Yellow
    } else {
        Write-Host "✗ JWT 过期时间设置错误" -ForegroundColor Red
    }

} catch {
    Write-Host "✗ JWT 令牌解码失败: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "`n=== JWT 令牌验证测试完成 ===" -ForegroundColor Green
