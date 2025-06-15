# JWT 认证管理模块前后端集成测试脚本
# 测试 AuthManager 与后端 Axum API 的正确对接

Write-Host "=== JWT 认证管理模块前后端集成测试 ===" -ForegroundColor Green
Write-Host "测试服务器: http://127.0.0.1:3000" -ForegroundColor Yellow
Write-Host ""

# 测试变量
$baseUrl = "http://127.0.0.1:3000"
$testUsername = "testuser$(Get-Random -Minimum 1000 -Maximum 9999)"
$testPassword = "password123"

# 测试结果统计
$testsPassed = 0
$testsFailed = 0

function Test-ApiEndpoint {
    param(
        [string]$TestName,
        [string]$Method,
        [string]$Url,
        [hashtable]$Headers = @{},
        [string]$Body = $null,
        [int]$ExpectedStatus = 200
    )
    
    Write-Host "测试: $TestName" -ForegroundColor Cyan
    
    try {
        $params = @{
            Uri = $Url
            Method = $Method
            Headers = $Headers
            ContentType = "application/json"
        }
        
        if ($Body) {
            $params.Body = $Body
        }
        
        $response = Invoke-WebRequest @params -ErrorAction Stop
        
        if ($response.StatusCode -eq $ExpectedStatus) {
            Write-Host "  ✓ 通过 - 状态码: $($response.StatusCode)" -ForegroundColor Green
            Write-Host "  响应: $($response.Content)" -ForegroundColor Gray
            $script:testsPassed++
            return $response
        } else {
            Write-Host "  ✗ 失败 - 期望状态码: $ExpectedStatus, 实际: $($response.StatusCode)" -ForegroundColor Red
            $script:testsFailed++
            return $null
        }
    }
    catch {
        Write-Host "  ✗ 失败 - 错误: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.Response) {
            Write-Host "  状态码: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
            try {
                $errorContent = $_.Exception.Response.GetResponseStream()
                $reader = New-Object System.IO.StreamReader($errorContent)
                $errorText = $reader.ReadToEnd()
                Write-Host "  错误响应: $errorText" -ForegroundColor Red
            } catch {}
        }
        $script:testsFailed++
        return $null
    }
    
    Write-Host ""
}

# 1. 测试服务器连接
Write-Host "1. 测试服务器连接" -ForegroundColor Yellow
$serverResponse = Test-ApiEndpoint -TestName "服务器健康检查" -Method "GET" -Url "$baseUrl/static/index.html" -ExpectedStatus 200

if (-not $serverResponse) {
    Write-Host "服务器连接失败，终止测试" -ForegroundColor Red
    exit 1
}

# 2. 测试用户注册 API
Write-Host "2. 测试用户注册 API" -ForegroundColor Yellow
$registerBody = @{
    username = $testUsername
    password = $testPassword
    confirmPassword = $testPassword
} | ConvertTo-Json

$registerResponse = Test-ApiEndpoint -TestName "用户注册" -Method "POST" -Url "$baseUrl/api/auth/register" -Body $registerBody -ExpectedStatus 201

if ($registerResponse) {
    $registerData = $registerResponse.Content | ConvertFrom-Json
    Write-Host "  注册成功 - 用户ID: $($registerData.user.id), 用户名: $($registerData.user.username)" -ForegroundColor Green
}

# 3. 测试用户登录 API
Write-Host "3. 测试用户登录 API" -ForegroundColor Yellow
$loginBody = @{
    username = $testUsername
    password = $testPassword
} | ConvertTo-Json

$loginResponse = Test-ApiEndpoint -TestName "用户登录" -Method "POST" -Url "$baseUrl/api/auth/login" -Body $loginBody -ExpectedStatus 200

$jwtToken = $null
if ($loginResponse) {
    $loginData = $loginResponse.Content | ConvertFrom-Json
    $jwtToken = $loginData.access_token
    Write-Host "  登录成功 - 用户: $($loginData.user.username)" -ForegroundColor Green
    Write-Host "  JWT 令牌: $($jwtToken.Substring(0, 50))..." -ForegroundColor Green
    Write-Host "  令牌类型: $($loginData.token_type)" -ForegroundColor Green
    Write-Host "  过期时间: $($loginData.expires_in) 秒" -ForegroundColor Green
}

# 4. 测试 JWT 令牌验证（访问受保护的资源）
if ($jwtToken) {
    Write-Host "4. 测试 JWT 令牌验证" -ForegroundColor Yellow
    $authHeaders = @{
        "Authorization" = "Bearer $jwtToken"
    }
    
    $tasksResponse = Test-ApiEndpoint -TestName "访问受保护的任务列表" -Method "GET" -Url "$baseUrl/api/tasks" -Headers $authHeaders -ExpectedStatus 200
    
    if ($tasksResponse) {
        $tasksData = $tasksResponse.Content | ConvertFrom-Json
        Write-Host "  成功访问任务列表 - 任务数量: $($tasksData.Count)" -ForegroundColor Green
    }
}

# 5. 测试输入验证
Write-Host "5. 测试输入验证" -ForegroundColor Yellow

# 测试短用户名
$shortUsernameBody = @{
    username = "ab"
    password = $testPassword
    confirmPassword = $testPassword
} | ConvertTo-Json

Test-ApiEndpoint -TestName "短用户名验证" -Method "POST" -Url "$baseUrl/api/auth/register" -Body $shortUsernameBody -ExpectedStatus 400

# 测试短密码
$shortPasswordBody = @{
    username = "validusername"
    password = "123"
    confirmPassword = "123"
} | ConvertTo-Json

Test-ApiEndpoint -TestName "短密码验证" -Method "POST" -Url "$baseUrl/api/auth/register" -Body $shortPasswordBody -ExpectedStatus 400

# 测试密码不匹配
$mismatchPasswordBody = @{
    username = "validusername"
    password = "password123"
    confirmPassword = "password456"
} | ConvertTo-Json

Test-ApiEndpoint -TestName "密码不匹配验证" -Method "POST" -Url "$baseUrl/api/auth/register" -Body $mismatchPasswordBody -ExpectedStatus 400

# 6. 测试重复用户名
Write-Host "6. 测试重复用户名" -ForegroundColor Yellow
$duplicateUserBody = @{
    username = $testUsername
    password = $testPassword
    confirmPassword = $testPassword
} | ConvertTo-Json

Test-ApiEndpoint -TestName "重复用户名" -Method "POST" -Url "$baseUrl/api/auth/register" -Body $duplicateUserBody -ExpectedStatus 409

# 7. 测试无效登录凭据
Write-Host "7. 测试无效登录凭据" -ForegroundColor Yellow
$invalidLoginBody = @{
    username = $testUsername
    password = "wrongpassword"
} | ConvertTo-Json

Test-ApiEndpoint -TestName "无效密码登录" -Method "POST" -Url "$baseUrl/api/auth/login" -Body $invalidLoginBody -ExpectedStatus 401

# 8. 测试无 JWT 令牌访问受保护资源
Write-Host "8. 测试无 JWT 令牌访问受保护资源" -ForegroundColor Yellow
Test-ApiEndpoint -TestName "无令牌访问任务列表" -Method "GET" -Url "$baseUrl/api/tasks" -ExpectedStatus 401

# 9. 测试无效 JWT 令牌
Write-Host "9. 测试无效 JWT 令牌" -ForegroundColor Yellow
$invalidAuthHeaders = @{
    "Authorization" = "Bearer invalid.jwt.token"
}

Test-ApiEndpoint -TestName "无效令牌访问任务列表" -Method "GET" -Url "$baseUrl/api/tasks" -Headers $invalidAuthHeaders -ExpectedStatus 401

# 测试结果总结
Write-Host ""
Write-Host "=== 测试结果总结 ===" -ForegroundColor Green
Write-Host "通过: $testsPassed" -ForegroundColor Green
Write-Host "失败: $testsFailed" -ForegroundColor Red
Write-Host "总计: $($testsPassed + $testsFailed)" -ForegroundColor Yellow

if ($testsFailed -eq 0) {
    Write-Host ""
    Write-Host "🎉 所有测试通过！JWT 认证管理模块与后端 API 集成正常。" -ForegroundColor Green
    exit 0
} else {
    Write-Host ""
    Write-Host "❌ 有 $testsFailed 个测试失败，请检查实现。" -ForegroundColor Red
    exit 1
}
