// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * 完整用户场景端到端测试
 * 
 * 测试目标：验证完整的用户流程，从注册到聊天的全过程
 * 
 * 测试场景：
 * 1. 用户注册 → 登录 → 任务操作 → WebSocket聊天 → 登出
 * 2. 多用户并发场景测试（模拟2-3个用户同时操作）
 * 3. WebSocket实时通信端到端功能验证
 * 4. 系统故障恢复场景（网络中断、重连等）
 * 5. 性能和稳定性验证
 */

// 测试配置
const BASE_URL = 'http://127.0.0.1:3000';
const API_BASE = `${BASE_URL}/api`;
const WS_URL = 'ws://127.0.0.1:3000/ws';

// 测试用户数据 - 使用高精度时间戳和更强随机性确保唯一性
const createTestUser = (prefix) => {
  const timestamp = Date.now();
  const microseconds = performance.now().toString().replace('.', '');
  const randomStr = Math.random().toString(36).substr(2, 12);
  const extraRandom = Math.floor(Math.random() * 10000);

  return {
    username: `${prefix}_${timestamp}_${microseconds}_${randomStr}_${extraRandom}`,
    password: 'TestPassword123!',
    confirmPassword: 'TestPassword123!'
  };
};

// 测试任务数据
const createTestTask = (title) => ({
  title: `${title} - ${Date.now()}`,
  description: `测试任务描述 - ${new Date().toISOString()}`
});

// 辅助函数：等待元素可见
async function waitForElement(page, selector, timeout = 10000) {
  await page.waitForSelector(selector, { state: 'visible', timeout });
}

// 辅助函数：用户注册
async function registerUser(page, user) {
  console.log(`📝 注册用户: ${user.username}`);

  // 点击注册标签
  await page.click('#registerTab');
  await waitForElement(page, '#registerForm');

  // 确保表单字段为空
  await page.fill('#registerUsername', '');
  await page.fill('#registerPassword', '');
  await page.fill('#confirmPassword', '');
  await page.waitForTimeout(200);

  // 填写注册表单
  await page.fill('#registerUsername', user.username);
  await page.fill('#registerPassword', user.password);
  await page.fill('#confirmPassword', user.confirmPassword);

  // 提交注册
  await page.click('#registerForm button[type="submit"]');

  // 等待注册结果
  await page.waitForTimeout(3000);

  // 检查注册是否成功
  const registerMessage = page.locator('#registerMessage');
  const registerMessageText = await registerMessage.textContent();
  if (registerMessageText && registerMessageText.includes('失败')) {
    console.error(`❌ 注册失败: ${registerMessageText}`);
    throw new Error(`注册失败: ${registerMessageText}`);
  }

  console.log(`✅ 用户注册完成: ${user.username}`);
}

// 辅助函数：用户登录
async function loginUser(page, user) {
  console.log(`🔐 登录用户: ${user.username}`);

  // 确保在登录表单
  const loginForm = page.locator('#loginForm');
  if (!(await loginForm.isVisible())) {
    await page.click('#loginTab');
    await waitForElement(page, '#loginForm');
  }

  // 确保表单字段为空
  await page.fill('#loginUsername', '');
  await page.fill('#loginPassword', '');
  await page.waitForTimeout(200);

  // 填写登录表单
  await page.fill('#loginUsername', user.username);
  await page.fill('#loginPassword', user.password);

  // 提交登录
  await page.click('#loginForm button[type="submit"]');

  // 等待登录完成
  await page.waitForTimeout(3000);

  // 检查是否有登录错误消息
  const loginMessage = page.locator('#loginMessage');
  const loginMessageText = await loginMessage.textContent();
  if (loginMessageText && loginMessageText.includes('失败')) {
    console.error(`❌ 登录失败: ${loginMessageText}`);
    throw new Error(`登录失败: ${loginMessageText}`);
  }

  // 验证登录状态
  const authStatus = page.locator('#authStatus');
  await expect(authStatus).toHaveText('已认证', { timeout: 15000 });

  console.log(`✅ 用户登录成功: ${user.username}`);
}

// 辅助函数：创建任务
async function createTask(page, task) {
  console.log(`📋 创建任务: ${task.title}`);

  // 确保表单字段为空，避免残留数据
  await page.fill('#title', '');
  await page.fill('#description', '');
  await page.waitForTimeout(100);

  // 填写任务表单
  await page.fill('#title', task.title);
  await page.fill('#description', task.description);

  // 提交任务
  await page.click('#createTaskForm button[type="submit"]');

  // 等待任务创建完成 - 增加等待时间
  await page.waitForTimeout(1500);

  // 验证任务出现在列表中
  const taskList = page.locator('#taskList');
  await expect(taskList).toContainText(task.title, { timeout: 8000 });

  console.log(`✅ 任务创建成功: ${task.title}`);
}

// 辅助函数：建立WebSocket连接
async function connectWebSocket(page, user) {
  console.log(`🔌 建立WebSocket连接: ${user.username}`);
  
  // 点击连接按钮
  const wsConnectBtn = page.locator('#wsConnectBtn');
  await expect(wsConnectBtn).toBeEnabled();
  await wsConnectBtn.click();
  
  // 等待连接建立
  await page.waitForTimeout(2000);
  
  // 验证连接状态
  const wsStatus = page.locator('#wsStatus');
  await expect(wsStatus).toHaveText('已连接', { timeout: 10000 });
  await expect(wsStatus).toHaveClass(/connected/);
  
  // 验证连接成功消息
  const wsMessages = page.locator('#rawMessages');
  await expect(wsMessages).toContainText('已连接到WebSocket服务器（已认证）');
  
  console.log(`✅ WebSocket连接成功: ${user.username}`);
}

// 辅助函数：发送WebSocket消息
async function sendWebSocketMessage(page, message, user) {
  console.log(`💬 发送消息: ${message} (${user.username})`);
  
  const messageInput = page.locator('#messageInput');
  const sendBtn = page.locator('#sendBtn');
  
  await messageInput.fill(message);
  await sendBtn.click();
  
  // 验证消息发送
  const wsMessages = page.locator('#rawMessages');
  await expect(wsMessages).toContainText(`发送: ${message}`, { timeout: 5000 });
  
  console.log(`✅ 消息发送成功: ${message}`);
}

// 辅助函数：断开WebSocket连接
async function disconnectWebSocket(page, user) {
  console.log(`🔌 断开WebSocket连接: ${user.username}`);
  
  const wsDisconnectBtn = page.locator('#wsDisconnectBtn');
  await expect(wsDisconnectBtn).toBeEnabled();
  await wsDisconnectBtn.click();
  
  // 等待断开完成
  await page.waitForTimeout(2000);
  
  // 验证断开状态
  const wsStatus = page.locator('#wsStatus');
  await expect(wsStatus).toHaveText('未连接', { timeout: 10000 });
  await expect(wsStatus).toHaveClass(/disconnected/);
  
  console.log(`✅ WebSocket断开成功: ${user.username}`);
}

// 辅助函数：用户登出
async function logoutUser(page, user) {
  console.log(`🚪 用户登出: ${user.username}`);
  
  const logoutBtn = page.locator('#logoutBtn');
  await expect(logoutBtn).toBeVisible();
  await logoutBtn.click();
  
  // 等待登出完成
  await page.waitForTimeout(1000);
  
  // 验证登出状态
  const authStatus = page.locator('#authStatus');
  await expect(authStatus).toHaveText('未认证', { timeout: 5000 });
  
  console.log(`✅ 用户登出成功: ${user.username}`);
}

test.describe('完整用户场景端到端测试', () => {

  test.beforeEach(async ({ page }) => {
    // 导航到主页
    await page.goto('/');

    // 等待页面加载完成
    await page.waitForLoadState('networkidle');
  });

  test('单用户完整流程：注册 → 登录 → 任务操作 → WebSocket聊天 → 登出', async ({ page }) => {
    console.log('🚀 开始单用户完整流程测试...');

    const testUser = createTestUser('complete_user');
    const testTask = createTestTask('完整流程测试任务');

    // 1. 用户注册
    await registerUser(page, testUser);

    // 2. 用户登录
    await loginUser(page, testUser);

    // 3. 创建任务
    await createTask(page, testTask);

    // 4. 建立WebSocket连接
    await connectWebSocket(page, testUser);

    // 5. 发送WebSocket消息
    await sendWebSocketMessage(page, 'Hello from complete user journey!', testUser);

    // 6. 断开WebSocket连接
    await disconnectWebSocket(page, testUser);

    // 7. 用户登出
    await logoutUser(page, testUser);

    console.log('✅ 单用户完整流程测试完成');
  });

  test('多用户并发场景：2个用户同时操作和聊天', async ({ browser }) => {
    console.log('🚀 开始多用户并发场景测试...');

    // 创建两个独立的浏览器上下文
    const context1 = await browser.newContext();
    const context2 = await browser.newContext();

    const page1 = await context1.newPage();
    const page2 = await context2.newPage();

    try {
      // 导航到主页
      await Promise.all([
        page1.goto('/'),
        page2.goto('/')
      ]);

      // 等待页面加载
      await Promise.all([
        page1.waitForLoadState('networkidle'),
        page2.waitForLoadState('networkidle')
      ]);

      const user1 = createTestUser('concurrent_user1');
      const user2 = createTestUser('concurrent_user2');

      // 1. 并发注册两个用户
      console.log('📝 并发注册两个用户...');
      await Promise.all([
        registerUser(page1, user1),
        registerUser(page2, user2)
      ]);

      // 2. 并发登录两个用户
      console.log('🔐 并发登录两个用户...');
      await Promise.all([
        loginUser(page1, user1),
        loginUser(page2, user2)
      ]);

      // 3. 顺序创建任务（避免并发冲突）
      console.log('📋 顺序创建任务...');
      const task1 = createTestTask('用户1的任务');
      const task2 = createTestTask('用户2的任务');

      // 用户1创建任务
      await createTask(page1, task1);
      await page1.waitForTimeout(500);

      // 用户2创建任务
      await createTask(page2, task2);
      await page2.waitForTimeout(500);

      // 4. 并发建立WebSocket连接
      console.log('🔌 并发建立WebSocket连接...');
      await Promise.all([
        connectWebSocket(page1, user1),
        connectWebSocket(page2, user2)
      ]);

      // 5. 交替发送消息测试实时通信
      console.log('💬 测试实时通信...');
      await sendWebSocketMessage(page1, `Hello from ${user1.username}!`, user1);
      await page1.waitForTimeout(500);

      await sendWebSocketMessage(page2, `Hello from ${user2.username}!`, user2);
      await page2.waitForTimeout(500);

      await sendWebSocketMessage(page1, `How are you, ${user2.username}?`, user1);
      await page1.waitForTimeout(500);

      await sendWebSocketMessage(page2, `I'm fine, ${user1.username}!`, user2);
      await page2.waitForTimeout(500);

      // 6. 并发断开WebSocket连接
      console.log('🔌 并发断开WebSocket连接...');
      await Promise.all([
        disconnectWebSocket(page1, user1),
        disconnectWebSocket(page2, user2)
      ]);

      // 7. 并发登出
      console.log('🚪 并发用户登出...');
      await Promise.all([
        logoutUser(page1, user1),
        logoutUser(page2, user2)
      ]);

      console.log('✅ 多用户并发场景测试完成');

    } finally {
      // 清理资源
      await page1.close();
      await page2.close();
      await context1.close();
      await context2.close();
    }
  });

  test('WebSocket连接故障恢复：网络中断和重连', async ({ page }) => {
    console.log('🚀 开始WebSocket故障恢复测试...');

    const testUser = createTestUser('recovery_user');

    // 1. 用户注册和登录
    await registerUser(page, testUser);
    await loginUser(page, testUser);

    // 2. 建立WebSocket连接
    await connectWebSocket(page, testUser);

    // 3. 发送初始消息
    await sendWebSocketMessage(page, 'Initial message before disconnect', testUser);

    // 4. 模拟网络中断（手动断开连接）
    console.log('🔌 模拟网络中断...');
    await disconnectWebSocket(page, testUser);

    // 5. 等待一段时间模拟网络恢复
    console.log('⏳ 等待网络恢复...');
    await page.waitForTimeout(2000);

    // 6. 重新建立连接
    console.log('🔌 重新建立WebSocket连接...');
    await connectWebSocket(page, testUser);

    // 7. 验证重连后功能正常
    await sendWebSocketMessage(page, 'Message after reconnection', testUser);

    // 8. 清理
    await disconnectWebSocket(page, testUser);
    await logoutUser(page, testUser);

    console.log('✅ WebSocket故障恢复测试完成');
  });

  test('性能和稳定性验证：快速操作和消息发送', async ({ page }) => {
    console.log('🚀 开始性能和稳定性测试...');

    const testUser = createTestUser('performance_user');

    // 1. 用户注册和登录
    await registerUser(page, testUser);
    await loginUser(page, testUser);

    // 2. 建立WebSocket连接
    await connectWebSocket(page, testUser);

    // 3. 顺序创建多个任务（避免表单竞态条件）
    console.log('📋 顺序创建多个任务...');
    for (let i = 1; i <= 5; i++) {
      const task = createTestTask(`性能测试任务${i}`);
      await createTask(page, task);
      // 添加短暂延迟确保任务创建完成
      await page.waitForTimeout(200);
    }

    // 4. 快速发送多条WebSocket消息
    console.log('💬 快速发送多条消息...');
    const messagePromises = [];
    for (let i = 1; i <= 10; i++) {
      messagePromises.push(
        sendWebSocketMessage(page, `Performance test message ${i}`, testUser)
      );
      // 添加小延迟避免过快发送
      await page.waitForTimeout(100);
    }

    // 5. 验证所有消息都已发送
    console.log('✅ 验证消息发送完成...');
    const wsMessages = page.locator('#rawMessages');
    await expect(wsMessages).toContainText('Performance test message 10', { timeout: 10000 });

    // 6. 测试页面刷新后的状态恢复
    console.log('🔄 测试页面刷新后状态恢复...');
    await page.reload();
    await page.waitForLoadState('networkidle');

    // 验证登录状态恢复
    const authStatus = page.locator('#authStatus');
    await expect(authStatus).toHaveText('已认证', { timeout: 5000 });

    // 验证任务列表恢复
    const taskList = page.locator('#taskList');
    await expect(taskList).toContainText('性能测试任务1', { timeout: 5000 });

    // 7. 重新建立WebSocket连接
    await connectWebSocket(page, testUser);

    // 8. 验证连接后功能正常
    await sendWebSocketMessage(page, 'Message after page refresh', testUser);

    // 9. 清理
    await disconnectWebSocket(page, testUser);
    await logoutUser(page, testUser);

    console.log('✅ 性能和稳定性测试完成');
  });

  test('三用户并发聊天场景：模拟聊天室环境', async ({ browser }) => {
    console.log('🚀 开始三用户并发聊天测试...');

    // 创建三个独立的浏览器上下文
    const contexts = await Promise.all([
      browser.newContext(),
      browser.newContext(),
      browser.newContext()
    ]);

    const pages = await Promise.all([
      contexts[0].newPage(),
      contexts[1].newPage(),
      contexts[2].newPage()
    ]);

    try {
      // 导航到主页
      await Promise.all(pages.map(page => page.goto('/')));

      // 等待页面加载
      await Promise.all(pages.map(page => page.waitForLoadState('networkidle')));

      const users = [
        createTestUser('chat_user1'),
        createTestUser('chat_user2'),
        createTestUser('chat_user3')
      ];

      // 1. 并发注册三个用户
      console.log('📝 并发注册三个用户...');
      await Promise.all(users.map((user, index) => registerUser(pages[index], user)));

      // 2. 并发登录三个用户
      console.log('🔐 并发登录三个用户...');
      await Promise.all(users.map((user, index) => loginUser(pages[index], user)));

      // 3. 并发建立WebSocket连接
      console.log('🔌 并发建立WebSocket连接...');
      await Promise.all(users.map((user, index) => connectWebSocket(pages[index], user)));

      // 4. 模拟聊天室对话
      console.log('💬 模拟聊天室对话...');

      // 用户1发起对话
      await sendWebSocketMessage(pages[0], 'Hello everyone! 👋', users[0]);
      await pages[0].waitForTimeout(500);

      // 用户2回应
      await sendWebSocketMessage(pages[1], 'Hi there! How is everyone doing?', users[1]);
      await pages[1].waitForTimeout(500);

      // 用户3加入对话
      await sendWebSocketMessage(pages[2], 'Great to see you all here! 😊', users[2]);
      await pages[2].waitForTimeout(500);

      // 用户1继续对话
      await sendWebSocketMessage(pages[0], 'This chat system is working perfectly!', users[0]);
      await pages[0].waitForTimeout(500);

      // 用户2和用户3同时发送消息
      await Promise.all([
        sendWebSocketMessage(pages[1], 'Absolutely! The real-time sync is amazing.', users[1]),
        sendWebSocketMessage(pages[2], 'I agree! Very smooth experience.', users[2])
      ]);

      // 5. 验证消息延迟（应该在1秒内）
      console.log('⏱️ 验证消息延迟...');
      const startTime = Date.now();
      await sendWebSocketMessage(pages[0], 'Testing message delay...', users[0]);
      const endTime = Date.now();
      const delay = endTime - startTime;

      console.log(`📊 消息发送延迟: ${delay}ms`);
      expect(delay).toBeLessThan(1000); // 要求延迟小于1秒

      // 6. 并发断开连接
      console.log('🔌 并发断开WebSocket连接...');
      await Promise.all(users.map((user, index) => disconnectWebSocket(pages[index], user)));

      // 7. 并发登出
      console.log('🚪 并发用户登出...');
      await Promise.all(users.map((user, index) => logoutUser(pages[index], user)));

      console.log('✅ 三用户并发聊天测试完成');

    } finally {
      // 清理资源
      await Promise.all(pages.map(page => page.close()));
      await Promise.all(contexts.map(context => context.close()));
    }
  });
});
