// @ts-check
const { test, expect } = require('@playwright/test');

/**
 * 简化版用户场景端到端测试
 * 
 * 测试目标：验证基本的用户流程功能
 * 
 * 测试场景：
 * 1. 用户注册和登录
 * 2. 创建任务
 * 3. WebSocket连接和消息发送
 * 4. 用户登出
 */

// 测试配置
const BASE_URL = 'http://127.0.0.1:3000';

// 测试用户数据 - 使用时间戳确保唯一性
const createTestUser = (prefix) => ({
  username: `${prefix}_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
  password: 'TestPassword123!',
  confirmPassword: 'TestPassword123!'
});

// 测试任务数据
const createTestTask = (title) => ({
  title: `${title} - ${Date.now()}`,
  description: `测试任务描述 - ${new Date().toISOString()}`
});

test.describe('简化版用户场景端到端测试', () => {
  
  test.beforeEach(async ({ page }) => {
    // 导航到主页
    await page.goto('/');
    
    // 等待页面加载完成
    await page.waitForLoadState('networkidle');
  });

  test('基本用户流程：注册 → 登录 → 创建任务 → WebSocket → 登出', async ({ page }) => {
    console.log('🚀 开始基本用户流程测试...');
    
    const testUser = createTestUser('simple_user');
    const testTask = createTestTask('简单测试任务');
    
    // 1. 用户注册
    console.log(`📝 注册用户: ${testUser.username}`);
    
    // 点击注册标签
    await page.click('#registerTab');
    await page.waitForSelector('#registerForm', { state: 'visible' });
    
    // 填写注册表单
    await page.fill('#registerUsername', testUser.username);
    await page.fill('#registerPassword', testUser.password);
    await page.fill('#confirmPassword', testUser.confirmPassword);
    
    // 提交注册
    await page.click('#registerForm button[type="submit"]');
    await page.waitForTimeout(2000);
    
    console.log(`✅ 用户注册完成: ${testUser.username}`);
    
    // 2. 用户登录
    console.log(`🔐 登录用户: ${testUser.username}`);
    
    // 切换到登录标签
    await page.click('#loginTab');
    await page.waitForSelector('#loginForm', { state: 'visible' });
    
    // 填写登录表单
    await page.fill('#loginUsername', testUser.username);
    await page.fill('#loginPassword', testUser.password);
    
    // 提交登录
    await page.click('#loginForm button[type="submit"]');
    await page.waitForTimeout(2000);
    
    // 验证登录状态
    const authStatus = page.locator('#authStatus');
    await expect(authStatus).toHaveText('已认证', { timeout: 10000 });
    
    console.log(`✅ 用户登录成功: ${testUser.username}`);
    
    // 3. 创建任务
    console.log(`📋 创建任务: ${testTask.title}`);
    
    // 填写任务表单
    await page.fill('#title', testTask.title);
    await page.fill('#description', testTask.description);
    
    // 提交任务
    await page.click('#createTaskForm button[type="submit"]');
    await page.waitForTimeout(2000);
    
    // 验证任务出现在列表中
    const taskList = page.locator('#tasksList');
    await expect(taskList).toContainText(testTask.title, { timeout: 10000 });
    
    console.log(`✅ 任务创建成功: ${testTask.title}`);
    
    // 4. WebSocket连接和消息发送
    console.log(`🔌 建立WebSocket连接: ${testUser.username}`);
    
    // 点击连接按钮
    const wsConnectBtn = page.locator('#wsConnectBtn');
    await expect(wsConnectBtn).toBeEnabled();
    await wsConnectBtn.click();
    
    // 等待连接建立
    await page.waitForTimeout(3000);
    
    // 验证连接状态
    const wsStatus = page.locator('#wsStatus');
    await expect(wsStatus).toHaveText('已连接', { timeout: 15000 });
    
    console.log(`✅ WebSocket连接成功: ${testUser.username}`);
    
    // 发送测试消息
    console.log('💬 发送测试消息...');
    const messageInput = page.locator('#wsMessage');
    const sendBtn = page.locator('#wsSendForm button[type="submit"]');
    
    await messageInput.fill('Hello from simple test!');
    await sendBtn.click();
    
    // 验证消息发送
    const wsMessages = page.locator('#webSocketMessages');
    await expect(wsMessages).toContainText('发送: Hello from simple test!', { timeout: 10000 });
    
    console.log('✅ 消息发送成功');
    
    // 5. 断开WebSocket连接
    console.log(`🔌 断开WebSocket连接: ${testUser.username}`);
    
    const wsDisconnectBtn = page.locator('#wsDisconnectBtn');
    await expect(wsDisconnectBtn).toBeEnabled();
    await wsDisconnectBtn.click();
    
    // 等待断开完成
    await page.waitForTimeout(3000);
    
    // 验证断开状态
    await expect(wsStatus).toHaveText('未连接', { timeout: 15000 });
    
    console.log(`✅ WebSocket断开成功: ${testUser.username}`);
    
    // 6. 用户登出
    console.log(`🚪 用户登出: ${testUser.username}`);
    
    const logoutBtn = page.locator('#logoutBtn');
    await expect(logoutBtn).toBeVisible();
    await logoutBtn.click();
    
    // 等待登出完成
    await page.waitForTimeout(2000);
    
    // 验证登出状态
    await expect(authStatus).toHaveText('未认证', { timeout: 10000 });
    
    console.log(`✅ 用户登出成功: ${testUser.username}`);
    
    console.log('✅ 基本用户流程测试完成');
  });

  test('双用户WebSocket聊天测试', async ({ browser }) => {
    console.log('🚀 开始双用户WebSocket聊天测试...');
    
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
      
      const user1 = createTestUser('chat_user1');
      const user2 = createTestUser('chat_user2');
      
      // 注册和登录用户1
      console.log(`📝 注册用户1: ${user1.username}`);
      await page1.click('#registerTab');
      await page1.waitForSelector('#registerForm', { state: 'visible' });
      await page1.fill('#registerUsername', user1.username);
      await page1.fill('#registerPassword', user1.password);
      await page1.fill('#confirmPassword', user1.confirmPassword);
      await page1.click('#registerForm button[type="submit"]');
      await page1.waitForTimeout(2000);
      
      await page1.click('#loginTab');
      await page1.waitForSelector('#loginForm', { state: 'visible' });
      await page1.fill('#loginUsername', user1.username);
      await page1.fill('#loginPassword', user1.password);
      await page1.click('#loginForm button[type="submit"]');
      await page1.waitForTimeout(2000);
      
      // 注册和登录用户2
      console.log(`📝 注册用户2: ${user2.username}`);
      await page2.click('#registerTab');
      await page2.waitForSelector('#registerForm', { state: 'visible' });
      await page2.fill('#registerUsername', user2.username);
      await page2.fill('#registerPassword', user2.password);
      await page2.fill('#confirmPassword', user2.confirmPassword);
      await page2.click('#registerForm button[type="submit"]');
      await page2.waitForTimeout(2000);
      
      await page2.click('#loginTab');
      await page2.waitForSelector('#loginForm', { state: 'visible' });
      await page2.fill('#loginUsername', user2.username);
      await page2.fill('#loginPassword', user2.password);
      await page2.click('#loginForm button[type="submit"]');
      await page2.waitForTimeout(2000);
      
      // 验证登录状态
      await expect(page1.locator('#authStatus')).toHaveText('已认证', { timeout: 10000 });
      await expect(page2.locator('#authStatus')).toHaveText('已认证', { timeout: 10000 });
      
      // 建立WebSocket连接
      console.log('🔌 建立WebSocket连接...');
      await page1.click('#wsConnectBtn');
      await page2.click('#wsConnectBtn');
      
      await page1.waitForTimeout(3000);
      await page2.waitForTimeout(3000);
      
      // 验证连接状态
      await expect(page1.locator('#wsStatus')).toHaveText('已连接', { timeout: 15000 });
      await expect(page2.locator('#wsStatus')).toHaveText('已连接', { timeout: 15000 });
      
      // 用户1发送消息
      console.log('💬 用户1发送消息...');
      await page1.fill('#wsMessage', `Hello from ${user1.username}!`);
      await page1.click('#wsSendForm button[type="submit"]');
      await page1.waitForTimeout(1000);
      
      // 用户2发送消息
      console.log('💬 用户2发送消息...');
      await page2.fill('#wsMessage', `Hello from ${user2.username}!`);
      await page2.click('#wsSendForm button[type="submit"]');
      await page2.waitForTimeout(1000);
      
      // 验证消息发送
      await expect(page1.locator('#webSocketMessages')).toContainText(`Hello from ${user1.username}!`, { timeout: 10000 });
      await expect(page2.locator('#webSocketMessages')).toContainText(`Hello from ${user2.username}!`, { timeout: 10000 });
      
      console.log('✅ 双用户WebSocket聊天测试完成');
      
    } finally {
      // 清理资源
      await page1.close();
      await page2.close();
      await context1.close();
      await context2.close();
    }
  });
});
