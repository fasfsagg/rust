/**
 * JWT 认证管理模块
 * 
 * 提供完整的用户认证功能，包括：
 * - JWT 令牌的安全存储和管理
 * - 自动过期检查和清理
 * - 用户登录、注册、登出功能
 * - 认证状态管理和事件通知
 * - 与后端 API 的完整集成
 * 
 * 安全特性：
 * - localStorage 存储（结合适当的安全措施）
 * - 自动令牌过期检查
 * - 令牌解析和验证
 * - 认证状态变化事件通知
 * 
 * 使用示例：
 * const authManager = new AuthManager();
 * await authManager.login('username', 'password');
 * const isAuth = authManager.isAuthenticated();
 * const user = authManager.getCurrentUser();
 */

class AuthManager {
    /**
     * 构造函数
     * @param {Object} options - 配置选项
     * @param {ApiClient} options.apiClient - API 客户端实例
     * @param {string} options.tokenKey - localStorage 中存储令牌的键名
     * @param {string} options.userKey - localStorage 中存储用户信息的键名
     * @param {number} options.checkInterval - 令牌过期检查间隔（毫秒）
     */
    constructor(options = {}) {
        // 初始化配置
        this._apiClient = options.apiClient || (window.ApiClient ? new window.ApiClient('/api') : null);
        this._tokenKey = options.tokenKey || 'jwt_token';
        this._userKey = options.userKey || 'user_info';
        this._checkInterval = options.checkInterval || 60000; // 1分钟检查一次
        
        // 内部状态
        this._isAuthenticated = false;
        this._currentUser = null;
        this._token = null;
        this._tokenExpiry = null;
        this._checkTimer = null;
        this._eventListeners = new Map();
        
        // 初始化
        this._initialize();
    }

    /**
     * 初始化认证管理器
     * @private
     */
    _initialize() {
        console.log('AUTH_MANAGER: 初始化认证管理器');
        
        // 检查 API 客户端是否可用
        if (!this._apiClient) {
            console.error('AUTH_MANAGER: API 客户端未找到，请确保 ApiClient 已加载');
            return;
        }
        
        // 从本地存储恢复认证状态
        this._restoreAuthState();
        
        // 启动定期检查
        this._startPeriodicCheck();
        
        // 监听页面可见性变化，当页面重新可见时检查令牌
        document.addEventListener('visibilitychange', () => {
            if (!document.hidden) {
                this._checkTokenValidity();
            }
        });
        
        console.log('AUTH_MANAGER: 初始化完成，当前认证状态:', this._isAuthenticated);
    }

    /**
     * 用户登录
     * @param {string} username - 用户名
     * @param {string} password - 密码
     * @returns {Promise<Object>} 登录结果
     */
    async login(username, password) {
        console.log('AUTH_MANAGER: 开始用户登录，用户名:', username);
        
        // 输入验证
        if (!username || !password) {
            throw new Error('用户名和密码不能为空');
        }
        
        if (username.length < 3) {
            throw new Error('用户名至少需要3个字符');
        }
        
        if (password.length < 8) {
            throw new Error('密码至少需要8个字符');
        }
        
        try {
            // 构建登录请求数据
            const loginData = { username, password };
            
            // 发送登录请求
            const response = await this._apiClient.post('/auth/login', loginData);
            
            console.log('AUTH_MANAGER: 登录请求成功，处理响应数据');
            
            // 处理登录响应
            this._handleAuthResponse(response);
            
            // 触发登录成功事件
            this._emitEvent('login', { user: this._currentUser });
            
            console.log('AUTH_MANAGER: 用户登录成功，用户:', this._currentUser.username);
            
            return {
                success: true,
                user: this._currentUser,
                message: '登录成功'
            };
            
        } catch (error) {
            console.error('AUTH_MANAGER: 用户登录失败:', error.message);
            
            // 触发登录失败事件
            this._emitEvent('loginError', { error: error.message });
            
            // 重新抛出错误，让调用者处理
            throw new Error(this._getErrorMessage(error));
        }
    }

    /**
     * 用户注册
     * @param {string} username - 用户名
     * @param {string} password - 密码
     * @param {string} confirmPassword - 确认密码
     * @returns {Promise<Object>} 注册结果
     */
    async register(username, password, confirmPassword) {
        console.log('AUTH_MANAGER: 开始用户注册，用户名:', username);
        
        // 输入验证
        if (!username || !password || !confirmPassword) {
            throw new Error('所有字段都不能为空');
        }
        
        if (username.length < 3) {
            throw new Error('用户名至少需要3个字符');
        }
        
        if (password.length < 8) {
            throw new Error('密码至少需要8个字符');
        }
        
        if (password !== confirmPassword) {
            throw new Error('密码和确认密码不匹配');
        }
        
        try {
            // 构建注册请求数据
            const registerData = { username, password, confirmPassword };
            
            // 发送注册请求
            const response = await this._apiClient.post('/auth/register', registerData);
            
            console.log('AUTH_MANAGER: 注册请求成功');
            
            // 触发注册成功事件
            this._emitEvent('register', { user: response.user });
            
            console.log('AUTH_MANAGER: 用户注册成功，用户:', response.user.username);
            
            return {
                success: true,
                user: response.user,
                message: response.message || '注册成功'
            };
            
        } catch (error) {
            console.error('AUTH_MANAGER: 用户注册失败:', error.message);
            
            // 触发注册失败事件
            this._emitEvent('registerError', { error: error.message });
            
            // 重新抛出错误，让调用者处理
            throw new Error(this._getErrorMessage(error));
        }
    }

    /**
     * 用户登出
     * @returns {Promise<void>}
     */
    async logout() {
        console.log('AUTH_MANAGER: 开始用户登出');
        
        try {
            // 清除本地认证状态
            this._clearAuthState();
            
            // 触发登出事件
            this._emitEvent('logout', {});
            
            console.log('AUTH_MANAGER: 用户登出成功');
            
        } catch (error) {
            console.error('AUTH_MANAGER: 登出过程中发生错误:', error.message);
            // 即使发生错误，也要清除本地状态
            this._clearAuthState();
        }
    }

    /**
     * 检查用户是否已认证
     * @returns {boolean} 是否已认证
     */
    isAuthenticated() {
        // 首先检查令牌是否有效
        this._checkTokenValidity();
        return this._isAuthenticated;
    }

    /**
     * 获取当前用户信息
     * @returns {Object|null} 当前用户信息
     */
    getCurrentUser() {
        if (!this.isAuthenticated()) {
            return null;
        }
        return this._currentUser;
    }

    /**
     * 获取有效的 JWT 令牌
     * @returns {string|null} JWT 令牌
     */
    getToken() {
        if (!this.isAuthenticated()) {
            return null;
        }
        return this._token;
    }

    /**
     * 添加事件监听器
     * @param {string} event - 事件名称 (login, logout, register, loginError, registerError, tokenExpired)
     * @param {Function} callback - 回调函数
     */
    addEventListener(event, callback) {
        if (!this._eventListeners.has(event)) {
            this._eventListeners.set(event, []);
        }
        this._eventListeners.get(event).push(callback);
    }

    /**
     * 移除事件监听器
     * @param {string} event - 事件名称
     * @param {Function} callback - 回调函数
     */
    removeEventListener(event, callback) {
        if (this._eventListeners.has(event)) {
            const listeners = this._eventListeners.get(event);
            const index = listeners.indexOf(callback);
            if (index > -1) {
                listeners.splice(index, 1);
            }
        }
    }

    /**
     * 销毁认证管理器
     */
    destroy() {
        console.log('AUTH_MANAGER: 销毁认证管理器');
        
        // 停止定期检查
        if (this._checkTimer) {
            clearInterval(this._checkTimer);
            this._checkTimer = null;
        }
        
        // 清除事件监听器
        this._eventListeners.clear();
        
        // 清除认证状态
        this._clearAuthState();
    }

    // ===== 私有方法 =====

    /**
     * 处理认证响应
     * @param {Object} response - 认证响应数据
     * @private
     */
    _handleAuthResponse(response) {
        if (!response || !response.access_token || !response.user) {
            throw new Error('无效的认证响应数据');
        }
        
        // 存储令牌和用户信息
        this._token = response.access_token;
        this._currentUser = response.user;
        this._isAuthenticated = true;
        
        // 解析令牌过期时间
        this._parseTokenExpiry();
        
        // 保存到本地存储
        this._saveAuthState();
        
        // 更新 API 客户端的认证令牌
        if (this._apiClient) {
            this._apiClient.setAuthToken(this._token);
        }
    }

    /**
     * 从本地存储恢复认证状态
     * @private
     */
    _restoreAuthState() {
        try {
            const token = localStorage.getItem(this._tokenKey);
            const userInfo = localStorage.getItem(this._userKey);
            
            if (token && userInfo) {
                this._token = token;
                this._currentUser = JSON.parse(userInfo);
                this._parseTokenExpiry();
                
                // 检查令牌是否仍然有效
                if (this._isTokenValid()) {
                    this._isAuthenticated = true;
                    
                    // 更新 API 客户端的认证令牌
                    if (this._apiClient) {
                        this._apiClient.setAuthToken(this._token);
                    }
                    
                    console.log('AUTH_MANAGER: 从本地存储恢复认证状态成功');
                } else {
                    console.log('AUTH_MANAGER: 本地存储的令牌已过期，清除认证状态');
                    this._clearAuthState();
                }
            }
        } catch (error) {
            console.error('AUTH_MANAGER: 恢复认证状态失败:', error.message);
            this._clearAuthState();
        }
    }

    /**
     * 保存认证状态到本地存储
     * @private
     */
    _saveAuthState() {
        try {
            localStorage.setItem(this._tokenKey, this._token);
            localStorage.setItem(this._userKey, JSON.stringify(this._currentUser));
        } catch (error) {
            console.error('AUTH_MANAGER: 保存认证状态失败:', error.message);
        }
    }

    /**
     * 清除认证状态
     * @private
     */
    _clearAuthState() {
        // 清除内存状态
        this._isAuthenticated = false;
        this._currentUser = null;
        this._token = null;
        this._tokenExpiry = null;
        
        // 清除本地存储
        localStorage.removeItem(this._tokenKey);
        localStorage.removeItem(this._userKey);
        
        // 清除 API 客户端的认证令牌
        if (this._apiClient) {
            this._apiClient.setAuthToken(null);
        }
    }

    /**
     * 解析令牌过期时间
     * @private
     */
    _parseTokenExpiry() {
        if (!this._token) {
            return;
        }
        
        try {
            // JWT 令牌格式：header.payload.signature
            const parts = this._token.split('.');
            if (parts.length !== 3) {
                throw new Error('无效的 JWT 令牌格式');
            }
            
            // 解码 payload（Base64URL）
            const payload = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/')));
            
            if (payload.exp) {
                // exp 是 Unix 时间戳（秒），转换为毫秒
                this._tokenExpiry = new Date(payload.exp * 1000);
            }
        } catch (error) {
            console.error('AUTH_MANAGER: 解析令牌过期时间失败:', error.message);
            this._tokenExpiry = null;
        }
    }

    /**
     * 检查令牌是否有效
     * @returns {boolean} 令牌是否有效
     * @private
     */
    _isTokenValid() {
        if (!this._token || !this._tokenExpiry) {
            return false;
        }
        
        // 检查是否过期（提前5分钟判断为过期，避免边界情况）
        const now = new Date();
        const bufferTime = 5 * 60 * 1000; // 5分钟缓冲时间
        
        return now.getTime() < (this._tokenExpiry.getTime() - bufferTime);
    }

    /**
     * 检查令牌有效性并更新认证状态
     * @private
     */
    _checkTokenValidity() {
        if (this._isAuthenticated && !this._isTokenValid()) {
            console.log('AUTH_MANAGER: 令牌已过期，清除认证状态');
            
            // 触发令牌过期事件
            this._emitEvent('tokenExpired', {});
            
            // 清除认证状态
            this._clearAuthState();
        }
    }

    /**
     * 启动定期检查
     * @private
     */
    _startPeriodicCheck() {
        if (this._checkTimer) {
            clearInterval(this._checkTimer);
        }
        
        this._checkTimer = setInterval(() => {
            this._checkTokenValidity();
        }, this._checkInterval);
    }

    /**
     * 触发事件
     * @param {string} event - 事件名称
     * @param {Object} data - 事件数据
     * @private
     */
    _emitEvent(event, data) {
        if (this._eventListeners.has(event)) {
            this._eventListeners.get(event).forEach(callback => {
                try {
                    callback(data);
                } catch (error) {
                    console.error(`AUTH_MANAGER: 事件回调执行失败 [${event}]:`, error);
                }
            });
        }
    }

    /**
     * 获取错误消息
     * @param {Error} error - 错误对象
     * @returns {string} 用户友好的错误消息
     * @private
     */
    _getErrorMessage(error) {
        // 根据错误类型返回用户友好的消息
        const message = error.message || '未知错误';
        
        if (message.includes('401')) {
            return '用户名或密码错误';
        }
        if (message.includes('409')) {
            return '用户名已存在';
        }
        if (message.includes('422')) {
            return '输入数据格式错误';
        }
        if (message.includes('500')) {
            return '服务器内部错误，请稍后重试';
        }
        if (message.includes('fetch')) {
            return '网络连接错误，请检查网络连接';
        }
        
        return message;
    }
}

// 导出 AuthManager 类，支持 ES6 模块和全局使用
if (typeof module !== 'undefined' && module.exports) {
    module.exports = AuthManager;
} else {
    window.AuthManager = AuthManager;
}
