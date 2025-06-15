Write-Host "JWT 认证管理模块集成测试" -ForegroundColor Green

# 测试变量
$baseUrl = "http://127.0.0.1:3000"
$testUsername = "testuser$(Get-Random -Minimum 1000 -Maximum 9999)"
$testPassword = "password123"

Write-Host "测试用户名: $testUsername" -ForegroundColor Yellow

# 1. 测试用户注册
Write-Host "1. 测试用户注册..." -ForegroundColor Cyan
$registerBody = @{
    username = $testUsername
    password = $testPassword
    confirmPassword = $testPassword
} | ConvertTo-Json

try {
    $registerResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method POST -Body $registerBody -ContentType "application/json"
    Write-Host "注册成功: $($registerResponse.message)" -ForegroundColor Green
    Write-Host "用户ID: $($registerResponse.user.id)" -ForegroundColor Green
} catch {
    Write-Host "注册失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 2. 测试用户登录
Write-Host "2. 测试用户登录..." -ForegroundColor Cyan
$loginBody = @{
    username = $testUsername
    password = $testPassword
} | ConvertTo-Json

try {
    $loginResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/login" -Method POST -Body $loginBody -ContentType "application/json"
    Write-Host "登录成功!" -ForegroundColor Green
    Write-Host "用户: $($loginResponse.user.username)" -ForegroundColor Green
    Write-Host "令牌类型: $($loginResponse.token_type)" -ForegroundColor Green
    Write-Host "过期时间: $($loginResponse.expires_in) 秒" -ForegroundColor Green
    Write-Host "JWT 令牌: $($loginResponse.access_token.Substring(0, 50))..." -ForegroundColor Green
    
    # 3. 测试使用 JWT 访问受保护资源
    Write-Host "3. 测试 JWT 令牌访问受保护资源..." -ForegroundColor Cyan
    $headers = @{
        "Authorization" = "Bearer $($loginResponse.access_token)"
    }
    
    $tasksResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET -Headers $headers
    Write-Host "成功访问任务列表，任务数量: $($tasksResponse.Count)" -ForegroundColor Green
    
} catch {
    Write-Host "登录失败: $($_.Exception.Message)" -ForegroundColor Red
}

# 4. 测试输入验证
Write-Host "4. 测试输入验证..." -ForegroundColor Cyan

# 测试短用户名
try {
    $shortUserBody = @{
        username = "ab"
        password = $testPassword
        confirmPassword = $testPassword
    } | ConvertTo-Json
    
    Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method POST -Body $shortUserBody -ContentType "application/json"
    Write-Host "短用户名验证失败 - 应该被拒绝" -ForegroundColor Red
} catch {
    Write-Host "短用户名正确被拒绝: $($_.Exception.Message)" -ForegroundColor Green
}

Write-Host "测试完成!" -ForegroundColor Green
