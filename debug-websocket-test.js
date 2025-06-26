const { test, expect } = require('@playwright/test');

test('调试WebSocket断开连接问题', async ({ page }) => {
  // 监听控制台消息
  page.on('console', msg => {
    console.log(`浏览器控制台: ${msg.text()}`);
  });

  // 导航到页面
  await page.goto('http://127.0.0.1:3080');

  // 注册用户
  await page.getByRole('button', { name: '注册' }).click();
  await page.getByRole('textbox', { name: '用户名' }).fill('debug_disconnect');
  await page.locator('#registerForm').getByRole('textbox', { name: '密码', exact: true }).fill('testpass123');
  await page.getByRole('textbox', { name: '确认密码' }).fill('testpass123');
  await page.locator('#registerForm').getByRole('button', { name: '注册' }).click();

  // 等待注册完成
  await page.waitForTimeout(1000);

  // 登录
  await page.getByRole('button', { name: '登录' }).click();
  await page.getByRole('textbox', { name: '用户名' }).fill('debug_disconnect');
  await page.getByRole('textbox', { name: '密码' }).fill('testpass123');
  await page.locator('#loginForm').getByRole('button', { name: '登录' }).click();

  // 等待登录完成
  await page.waitForTimeout(1000);

  // 验证登录状态
  const authStatus = page.locator('#authStatus');
  await expect(authStatus).toHaveText('已认证');

  const wsStatus = page.locator('#wsStatus');
  const wsConnectBtn = page.locator('#wsConnectBtn');
  const wsDisconnectBtn = page.locator('#wsDisconnectBtn');

  console.log('=== 步骤1: 建立WebSocket连接 ===');
  await wsConnectBtn.click();
  await page.waitForTimeout(2000);

  console.log('连接后状态:', await wsStatus.textContent());
  await expect(wsStatus).toHaveText('已连接');

  console.log('=== 步骤2: 测试断开连接 ===');
  console.log('断开按钮是否启用:', await wsDisconnectBtn.isEnabled());

  // 点击断开按钮
  console.log('点击断开按钮...');
  await wsDisconnectBtn.click();

  console.log('等待断开操作完成...');
  await page.waitForTimeout(3000);

  console.log('断开后状态:', await wsStatus.textContent());
  console.log('断开后CSS类:', await wsStatus.getAttribute('class'));

  // 检查是否真的断开了
  const finalStatus = await wsStatus.textContent();
  if (finalStatus === '未连接') {
    console.log('✅ WebSocket成功断开');
  } else {
    console.log('❌ WebSocket断开失败，当前状态:', finalStatus);
  }
});
