# 测试 Axum API 端点

$baseUrl = "http://127.0.0.1:3000"

Write-Host "=== 测试用户注册 ===" -ForegroundColor Green

$registerBody = @{
    username = "testuser"
    password = "password123"
    confirmPassword = "password123"
} | ConvertTo-Json

try {
    $registerResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method Post -Body $registerBody -ContentType "application/json"
    Write-Host "注册成功:" -ForegroundColor Green
    $registerResponse | ConvertTo-Json -Depth 3
} catch {
    Write-Host "注册失败:" -ForegroundColor Red
    Write-Host $_.Exception.Message
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "响应内容: $responseBody"
    }
}

Write-Host "`n=== 测试用户登录 ===" -ForegroundColor Green

$loginBody = @{
    username = "testuser"
    password = "password123"
} | ConvertTo-Json

try {
    $loginResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/login" -Method Post -Body $loginBody -ContentType "application/json"
    Write-Host "登录成功:" -ForegroundColor Green
    $loginResponse | ConvertTo-Json -Depth 3
    
    $token = $loginResponse.access_token
    Write-Host "获取到令牌: $token" -ForegroundColor Yellow
    
    # 测试任务 API
    Write-Host "`n=== 测试获取任务列表 ===" -ForegroundColor Green
    
    $headers = @{
        "Authorization" = "Bearer $token"
        "Content-Type" = "application/json"
    }
    
    try {
        $tasksResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method Get -Headers $headers
        Write-Host "获取任务列表成功:" -ForegroundColor Green
        $tasksResponse | ConvertTo-Json -Depth 3
    } catch {
        Write-Host "获取任务列表失败:" -ForegroundColor Red
        Write-Host $_.Exception.Message
    }
    
    # 测试创建任务
    Write-Host "`n=== 测试创建任务 ===" -ForegroundColor Green
    
    $createTaskBody = @{
        title = "测试任务"
        description = "这是一个测试任务"
        completed = $false
    } | ConvertTo-Json
    
    try {
        $createTaskResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks" -Method Post -Body $createTaskBody -Headers $headers
        Write-Host "创建任务成功:" -ForegroundColor Green
        $createTaskResponse | ConvertTo-Json -Depth 3
        
        $taskId = $createTaskResponse.id
        Write-Host "创建的任务ID: $taskId" -ForegroundColor Yellow
        
        # 测试获取单个任务
        Write-Host "`n=== 测试获取单个任务 ===" -ForegroundColor Green
        
        try {
            $getTaskResponse = Invoke-RestMethod -Uri "$baseUrl/api/tasks/$taskId" -Method Get -Headers $headers
            Write-Host "获取单个任务成功:" -ForegroundColor Green
            $getTaskResponse | ConvertTo-Json -Depth 3
        } catch {
            Write-Host "获取单个任务失败:" -ForegroundColor Red
            Write-Host $_.Exception.Message
        }
        
    } catch {
        Write-Host "创建任务失败:" -ForegroundColor Red
        Write-Host $_.Exception.Message
    }
    
} catch {
    Write-Host "登录失败:" -ForegroundColor Red
    Write-Host $_.Exception.Message
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())
        $responseBody = $reader.ReadToEnd()
        Write-Host "响应内容: $responseBody"
    }
}

Write-Host "`n=== API 测试完成 ===" -ForegroundColor Cyan
