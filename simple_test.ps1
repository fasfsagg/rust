# Simple API Test

$baseUrl = "http://127.0.0.1:3000"

Write-Host "Testing user registration..." -ForegroundColor Green

$registerBody = @{
    username = "testuser"
    password = "password123"
    confirmPassword = "password123"
} | ConvertTo-Json

try {
    $registerResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/register" -Method Post -Body $registerBody -ContentType "application/json"
    Write-Host "Registration successful:" -ForegroundColor Green
    $registerResponse | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Registration failed:" -ForegroundColor Red
    Write-Host $_.Exception.Message
}

Write-Host "Testing user login..." -ForegroundColor Green

$loginBody = @{
    username = "testuser"
    password = "password123"
} | ConvertTo-Json

try {
    $loginResponse = Invoke-RestMethod -Uri "$baseUrl/api/auth/login" -Method Post -Body $loginBody -ContentType "application/json"
    Write-Host "Login successful:" -ForegroundColor Green
    $loginResponse | ConvertTo-Json -Depth 3
} catch {
    Write-Host "Login failed:" -ForegroundColor Red
    Write-Host $_.Exception.Message
}

Write-Host "API test completed" -ForegroundColor Cyan
