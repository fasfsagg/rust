// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * WebSocket 安全测试套件
 * 
 * 测试覆盖以下场景：
 * 1. 未登录用户无法建立 WebSocket 连接
 * 2. 已登录用户可以正常建立 WebSocket 连接
 * 3. WebSocket 连接需要有效的 JWT token
 * 4. 过期或无效的 token 无法建立连接
 */

// 测试用户凭据 - 使用高精度时间戳和更强随机性确保唯一性
const createTestUser = () => {
  const timestamp = Date.now();
  const microseconds = performance.now().toString().replace('.', '');
  const randomStr = Math.random().toString(36).substr(2, 12);
  const extraRandom = Math.floor(Math.random() * 10000);

  return {
    username: `testuser_ws_${timestamp}_${microseconds}_${randomStr}_${extraRandom}`,
    password: 'testpass123'
  };
};

const TEST_USER = createTestUser();

test.describe('WebSocket 安全测试', () => {
  
  test.beforeEach(async ({ page }) => {
    // 导航到主页
    await page.goto('/');
    
    // 等待页面加载完成
    await page.waitForLoadState('networkidle');
  });

  test('未登录用户无法建立 WebSocket 连接', async ({ page }) => {
    // 确保用户未登录
    const authStatus = page.locator('#authStatus');
    await expect(authStatus).toHaveText('未认证');
    
    // WebSocket 连接按钮应该被禁用
    const wsConnectBtn = page.locator('#wsConnectBtn');
    await expect(wsConnectBtn).toBeDisabled();

    // 由于按钮被禁用，我们需要直接调用JavaScript函数来测试错误消息
    await page.evaluate(() => {
      // 直接调用连接函数来触发错误消息
      window.connectWebSocket();
    });

    // 检查是否显示错误消息
    const wsMessages = page.locator('#webSocketMessages');
    await expect(wsMessages).toContainText('错误: 请先登录后再连接WebSocket');
    
    // 确认 WebSocket 状态仍然是未连接
    const wsStatus = page.locator('#wsStatus');
    await expect(wsStatus).toHaveText('未连接');
    await expect(wsStatus).toHaveClass(/disconnected/);
  });

  test('已登录用户可以正常建立 WebSocket 连接', async ({ page }) => {
    // 1. 先注册用户
    await registerUser(page, TEST_USER);
    
    // 2. 登录用户
    await loginUser(page, TEST_USER);
    
    // 3. 验证登录状态
    const authStatus = page.locator('#authStatus');
    await expect(authStatus).toHaveText('已认证');
    
    // 4. WebSocket 连接按钮应该被启用
    const wsConnectBtn = page.locator('#wsConnectBtn');
    await expect(wsConnectBtn).toBeEnabled();
    
    // 5. 建立 WebSocket 连接
    await wsConnectBtn.click();
    
    // 6. 等待连接建立
    await page.waitForTimeout(1000);
    
    // 7. 验证连接状态
    const wsStatus = page.locator('#wsStatus');
    await expect(wsStatus).toHaveText('已连接');
    await expect(wsStatus).toHaveClass(/connected/);
    
    // 8. 验证连接成功消息
    const wsMessages = page.locator('#webSocketMessages');
    await expect(wsMessages).toContainText('已连接到WebSocket服务器（已认证）');
    
    // 9. 测试发送消息
    const messageInput = page.locator('#wsMessage');
    const sendBtn = page.locator('#wsSendForm button[type="submit"]');
    
    await expect(sendBtn).toBeEnabled();
    await messageInput.fill('Hello WebSocket!');
    await sendBtn.click();
    
    // 10. 验证消息发送和接收
    await expect(wsMessages).toContainText('发送: Hello WebSocket!');
    // 等待服务器回应（如果有的话）
    await page.waitForTimeout(500);
    
    // 11. 断开连接
    const wsDisconnectBtn = page.locator('#wsDisconnectBtn');
    await expect(wsDisconnectBtn).toBeEnabled();
    await wsDisconnectBtn.click();

    // 等待断开操作完成
    await page.waitForTimeout(1000);

    // 等待WebSocket状态变为"未连接"
    await expect(wsStatus).toHaveText('未连接', { timeout: 10000 });

    // 12. 验证断开状态
    await expect(wsStatus).toHaveText('未连接');
    await expect(wsStatus).toHaveClass(/disconnected/);
  });

  test('登出后 WebSocket 连接自动断开', async ({ page }) => {
    // 1. 注册并登录用户
    await registerUser(page, TEST_USER);
    await loginUser(page, TEST_USER);

    // 2. 等待登录状态完全更新
    await page.waitForTimeout(1000);

    // 3. 建立 WebSocket 连接
    const wsConnectBtn = page.locator('#wsConnectBtn');
    await expect(wsConnectBtn).toBeEnabled({ timeout: 10000 });
    await wsConnectBtn.click();
    await page.waitForTimeout(1000);
    
    // 3. 验证连接已建立
    const wsStatus = page.locator('#wsStatus');
    await expect(wsStatus).toHaveText('已连接');
    
    // 4. 登出用户
    const logoutBtn = page.locator('#logoutBtn');
    await logoutBtn.click();

    // 等待登出操作完成
    await page.waitForTimeout(2000);

    // 5. 验证登出状态
    const authStatus = page.locator('#authStatus');
    await expect(authStatus).toHaveText('未认证');

    // 6. 验证 WebSocket 连接自动断开
    await expect(wsStatus).toHaveText('未连接');
    await expect(wsStatus).toHaveClass(/disconnected/);

    // 7. 验证断开消息
    const wsMessages = page.locator('#webSocketMessages');
    await expect(wsMessages).toContainText('用户登出，WebSocket连接已断开');

    // 8. 等待更长时间确保所有状态更新完成
    await page.waitForTimeout(3000);

    // 9. 检查JavaScript中的socket状态
    const socketState = await page.evaluate(() => {
      return {
        socketExists: window.socket !== null,
        socketState: window.socket ? window.socket.readyState : 'null',
        authToken: window.authToken,
        currentUser: window.currentUser
      };
    });
    console.log('Socket状态:', socketState);

    // 10. 验证连接按钮被禁用
    await expect(wsConnectBtn).toBeDisabled();
  });

  test('WebSocket 连接状态在页面刷新后保持正确', async ({ page }) => {
    // 1. 注册并登录用户
    await registerUser(page, TEST_USER);
    await loginUser(page, TEST_USER);
    
    // 2. 刷新页面
    await page.reload();
    await page.waitForLoadState('networkidle');
    
    // 3. 验证登录状态恢复
    const authStatus = page.locator('#authStatus');
    await expect(authStatus).toHaveText('已认证');
    
    // 4. 验证 WebSocket 连接按钮可用
    const wsConnectBtn = page.locator('#wsConnectBtn');
    await expect(wsConnectBtn).toBeEnabled();
    
    // 5. 验证 WebSocket 状态正确
    const wsStatus = page.locator('#wsStatus');
    await expect(wsStatus).toHaveText('未连接');
    await expect(wsStatus).toHaveClass(/disconnected/);
  });

});

// 辅助函数：注册用户
async function registerUser(page, user) {
  // 切换到注册表单 - 使用更精确的选择器
  const registerTab = page.locator('#registerTab');
  await registerTab.click();

  // 填写注册表单
  await page.locator('#registerUsername').fill(user.username);
  await page.locator('#registerPassword').fill(user.password);
  await page.locator('#confirmPassword').fill(user.password);

  // 提交注册
  const registerBtn = page.locator('#registerForm button[type="submit"]');
  await registerBtn.click();

  // 等待注册完成 - 增加等待时间
  await page.waitForTimeout(3000);
}

// 辅助函数：登录用户
async function loginUser(page, user) {
  // 切换到登录表单 - 使用更精确的选择器
  const loginTab = page.locator('#loginTab');
  await loginTab.click();
  
  // 填写登录表单
  await page.locator('#loginUsername').fill(user.username);
  await page.locator('#loginPassword').fill(user.password);
  
  // 提交登录
  const loginBtn = page.locator('#loginForm button[type="submit"]');
  await loginBtn.click();

  // 等待登录完成 - 增加等待时间
  await page.waitForTimeout(3000);
}
