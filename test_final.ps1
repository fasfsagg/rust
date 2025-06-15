Write-Host "JWT 认证管理模块前后端集成验证测试" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green

$baseUrl = "http://127.0.0.1:3000"
$testUsername = "finaltest$(Get-Random -Minimum 1000 -Maximum 9999)"
$testPassword = "password123"

Write-Host "测试用户: $testUsername" -ForegroundColor Yellow
Write-Host ""

# 测试计数器
$passed = 0
$failed = 0

function Test-Result {
    param($condition, $testName)
    if ($condition) {
        Write-Host "✓ $testName" -ForegroundColor Green
        $script:passed++
    } else {
        Write-Host "✗ $testName" -ForegroundColor Red
        $script:failed++
    }
}

# 1. 用户注册测试
Write-Host "1. 用户注册测试" -ForegroundColor Cyan
$registerBody = @{
    username = $testUsername
    password = $testPassword
    confirmPassword = $testPassword
} | ConvertTo-Json

try {
    $registerResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method POST -Body $registerBody -ContentType "application/json"
    Test-Result $true "用户注册成功"
    Write-Host "   用户ID: $($registerResponse.user.id)" -ForegroundColor Gray
} catch {
    Test-Result $false "用户注册失败: $($_.Exception.Message)"
}

# 2. 用户登录测试
Write-Host "`n2. 用户登录测试" -ForegroundColor Cyan
$loginBody = @{
    username = $testUsername
    password = $testPassword
} | ConvertTo-Json

$jwtToken = $null
try {
    $loginResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/login" -Method POST -Body $loginBody -ContentType "application/json"
    $jwtToken = $loginResponse.access_token
    Test-Result $true "用户登录成功"
    Write-Host "   令牌类型: $($loginResponse.token_type)" -ForegroundColor Gray
    Write-Host "   过期时间: $($loginResponse.expires_in) 秒" -ForegroundColor Gray
    
    # 验证 JWT 格式
    $tokenParts = $jwtToken.Split('.')
    Test-Result ($tokenParts.Length -eq 3) "JWT 令牌格式正确"
    
} catch {
    Test-Result $false "用户登录失败: $($_.Exception.Message)"
}

# 3. JWT 令牌验证测试
if ($jwtToken) {
    Write-Host "`n3. JWT 令牌验证测试" -ForegroundColor Cyan
    $authHeaders = @{
        "Authorization" = "Bearer $jwtToken"
    }
    
    try {
        $tasksResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET -Headers $authHeaders
        Test-Result $true "使用 JWT 访问受保护资源成功"
        Write-Host "   任务数量: $($tasksResponse.Count)" -ForegroundColor Gray
    } catch {
        Test-Result $false "使用 JWT 访问受保护资源失败"
    }
    
    # 测试创建任务
    $taskBody = @{
        title = "JWT 测试任务"
        description = "通过 JWT 认证创建的任务"
        completed = $false
    } | ConvertTo-Json
    
    try {
        $createResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method POST -Body $taskBody -Headers $authHeaders -ContentType "application/json"
        Test-Result $true "使用 JWT 创建任务成功"
        Write-Host "   任务ID: $($createResponse.id)" -ForegroundColor Gray
    } catch {
        Test-Result $false "使用 JWT 创建任务失败"
    }
}

# 4. 安全性测试
Write-Host "`n4. 安全性测试" -ForegroundColor Cyan

# 测试无效令牌
try {
    $invalidHeaders = @{ "Authorization" = "Bearer invalid.token" }
    Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET -Headers $invalidHeaders
    Test-Result $false "无效令牌应该被拒绝"
} catch {
    Test-Result $true "无效令牌正确被拒绝"
}

# 测试缺少认证头
try {
    Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method GET
    Test-Result $false "缺少认证头应该被拒绝"
} catch {
    Test-Result $true "缺少认证头正确被拒绝"
}

# 5. 输入验证测试
Write-Host "`n5. 输入验证测试" -ForegroundColor Cyan

# 测试短用户名
try {
    $shortUserBody = @{
        username = "ab"
        password = $testPassword
        confirmPassword = $testPassword
    } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method POST -Body $shortUserBody -ContentType "application/json"
    Test-Result $false "短用户名应该被拒绝"
} catch {
    Test-Result $true "短用户名正确被拒绝"
}

# 测试短密码
try {
    $shortPassBody = @{
        username = "validuser"
        password = "123"
        confirmPassword = "123"
    } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method POST -Body $shortPassBody -ContentType "application/json"
    Test-Result $false "短密码应该被拒绝"
} catch {
    Test-Result $true "短密码正确被拒绝"
}

# 测试结果总结
Write-Host "`n========================================" -ForegroundColor Green
Write-Host "测试结果总结:" -ForegroundColor Green
Write-Host "通过: $passed" -ForegroundColor Green
Write-Host "失败: $failed" -ForegroundColor Red
Write-Host "总计: $($passed + $failed)" -ForegroundColor Yellow

if ($failed -eq 0) {
    Write-Host "`n🎉 所有测试通过！JWT 认证管理模块与后端集成完美！" -ForegroundColor Green
} else {
    Write-Host "`n❌ 有 $failed 个测试失败" -ForegroundColor Red
}
