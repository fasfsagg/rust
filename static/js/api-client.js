/**
 * API 客户端类
 * 
 * 提供统一的 HTTP API 调用接口，支持：
 * - 自动 JWT 认证头注入
 * - 统一的错误处理
 * - JSON 数据序列化/反序列化
 * - RESTful API 方法封装
 * 
 * 使用示例：
 * const apiClient = new ApiClient('/api');
 * apiClient.setAuthToken('your-jwt-token');
 * const data = await apiClient.get('/tasks');
 */

class ApiClient {
    /**
     * 构造函数
     * @param {string} baseURL - API 基础 URL
     */
    constructor(baseURL = '') {
        this.baseURL = baseURL;
        this.authToken = null;
        this.defaultHeaders = {
            'Content-Type': 'application/json'
        };
    }

    /**
     * 设置认证令牌
     * @param {string|null} token - JWT 令牌
     * @returns {ApiClient} 返回自身以支持链式调用
     */
    setAuthToken(token) {
        this.authToken = token;
        return this;
    }

    /**
     * 获取当前认证令牌
     * @returns {string|null} 当前的 JWT 令牌
     */
    getAuthToken() {
        return this.authToken;
    }

    /**
     * 构建请求头
     * @param {Object} additionalHeaders - 额外的请求头
     * @returns {Object} 完整的请求头对象
     * @private
     */
    _buildHeaders(additionalHeaders = {}) {
        const headers = { ...this.defaultHeaders, ...additionalHeaders };
        
        // 如果有认证令牌，添加 Authorization 头
        if (this.authToken) {
            headers['Authorization'] = `Bearer ${this.authToken}`;
        }
        
        return headers;
    }

    /**
     * 构建完整的 URL
     * @param {string} endpoint - API 端点
     * @returns {string} 完整的 URL
     * @private
     */
    _buildURL(endpoint) {
        // 确保端点以 / 开头
        if (!endpoint.startsWith('/')) {
            endpoint = '/' + endpoint;
        }
        
        return this.baseURL + endpoint;
    }

    /**
     * 处理响应
     * @param {Response} response - Fetch API 响应对象
     * @returns {Promise<any>} 解析后的响应数据
     * @private
     */
    async _handleResponse(response) {
        // 检查响应状态
        if (!response.ok) {
            let errorMessage = `HTTP error! status: ${response.status}`;
            
            try {
                // 尝试解析错误响应的 JSON 内容
                const errorData = await response.json();
                if (errorData.message) {
                    errorMessage = errorData.message;
                } else if (errorData.error) {
                    errorMessage = errorData.error;
                }
            } catch (parseError) {
                // 如果无法解析 JSON，使用默认错误消息
                console.warn('无法解析错误响应:', parseError);
            }
            
            throw new Error(errorMessage);
        }

        // 处理不同的响应类型
        const contentType = response.headers.get('content-type');
        
        if (response.status === 204) {
            // No Content 响应
            return null;
        } else if (contentType && contentType.includes('application/json')) {
            // JSON 响应
            return await response.json();
        } else {
            // 文本响应
            return await response.text();
        }
    }

    /**
     * 发起 HTTP 请求
     * @param {string} method - HTTP 方法
     * @param {string} endpoint - API 端点
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     * @private
     */
    async _request(method, endpoint, options = {}) {
        const url = this._buildURL(endpoint);
        const headers = this._buildHeaders(options.headers);
        
        const requestOptions = {
            method: method.toUpperCase(),
            headers,
            ...options
        };

        // 如果有请求体且不是 FormData，序列化为 JSON
        if (options.body && !(options.body instanceof FormData)) {
            if (typeof options.body === 'object') {
                requestOptions.body = JSON.stringify(options.body);
            }
        }

        try {
            console.log(`API_CLIENT: ${method.toUpperCase()} ${url}`);
            const response = await fetch(url, requestOptions);
            return await this._handleResponse(response);
        } catch (error) {
            console.error(`API_CLIENT: 请求失败 ${method.toUpperCase()} ${url}:`, error.message);
            throw error;
        }
    }

    /**
     * GET 请求
     * @param {string} endpoint - API 端点
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     */
    async get(endpoint, options = {}) {
        return this._request('GET', endpoint, options);
    }

    /**
     * POST 请求
     * @param {string} endpoint - API 端点
     * @param {any} data - 请求数据
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     */
    async post(endpoint, data = null, options = {}) {
        return this._request('POST', endpoint, {
            ...options,
            body: data
        });
    }

    /**
     * PUT 请求
     * @param {string} endpoint - API 端点
     * @param {any} data - 请求数据
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     */
    async put(endpoint, data = null, options = {}) {
        return this._request('PUT', endpoint, {
            ...options,
            body: data
        });
    }

    /**
     * PATCH 请求
     * @param {string} endpoint - API 端点
     * @param {any} data - 请求数据
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     */
    async patch(endpoint, data = null, options = {}) {
        return this._request('PATCH', endpoint, {
            ...options,
            body: data
        });
    }

    /**
     * DELETE 请求
     * @param {string} endpoint - API 端点
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     */
    async delete(endpoint, options = {}) {
        return this._request('DELETE', endpoint, options);
    }

    /**
     * 上传文件
     * @param {string} endpoint - API 端点
     * @param {File|FormData} fileOrFormData - 文件或 FormData 对象
     * @param {Object} options - 请求选项
     * @returns {Promise<any>} 响应数据
     */
    async upload(endpoint, fileOrFormData, options = {}) {
        let formData;
        
        if (fileOrFormData instanceof FormData) {
            formData = fileOrFormData;
        } else if (fileOrFormData instanceof File) {
            formData = new FormData();
            formData.append('file', fileOrFormData);
        } else {
            throw new Error('上传数据必须是 File 或 FormData 对象');
        }

        // 移除 Content-Type 头，让浏览器自动设置（包含 boundary）
        const headers = { ...options.headers };
        delete headers['Content-Type'];

        return this._request('POST', endpoint, {
            ...options,
            headers,
            body: formData
        });
    }

    /**
     * 批量请求
     * @param {Array} requests - 请求配置数组
     * @returns {Promise<Array>} 所有请求的结果数组
     */
    async batch(requests) {
        const promises = requests.map(req => {
            const { method, endpoint, data, options } = req;
            return this._request(method, endpoint, { ...options, body: data });
        });

        return Promise.allSettled(promises);
    }

    /**
     * 设置默认请求头
     * @param {Object} headers - 默认请求头
     */
    setDefaultHeaders(headers) {
        this.defaultHeaders = { ...this.defaultHeaders, ...headers };
    }

    /**
     * 清除认证令牌
     */
    clearAuth() {
        this.authToken = null;
    }
}

// 导出 ApiClient 类，支持 ES6 模块和全局使用
if (typeof module !== 'undefined' && module.exports) {
    module.exports = ApiClient;
} else {
    window.ApiClient = ApiClient;
}
