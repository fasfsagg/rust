/**
 * 任务管理模块
 * 
 * 提供完整的任务管理功能，包括：
 * - 任务的 CRUD 操作（创建、读取、更新、删除）
 * - 任务列表的渲染和筛选
 * - 任务编辑模式的切换
 * - 与认证系统的集成
 * - 统一的错误处理和用户反馈
 * 
 * 依赖：
 * - ApiClient: 用于 HTTP API 调用
 * - AuthManager: 用于认证状态检查
 * 
 * 使用示例：
 * const taskManager = new TaskManager({
 *   apiClient: apiClient,
 *   authManager: authManager,
 *   updateApiResponse: updateApiResponseFunction
 * });
 * await taskManager.initialize();
 */

class TaskManager {
    /**
     * 构造函数
     * @param {Object} options - 配置选项
     * @param {ApiClient} options.apiClient - API 客户端实例
     * @param {AuthManager} options.authManager - 认证管理器实例
     * @param {Function} options.updateApiResponse - 更新 API 响应显示的回调函数
     * @param {string} options.tasksListId - 任务列表容器的 DOM ID
     * @param {string} options.createFormId - 创建任务表单的 DOM ID
     */
    constructor(options = {}) {
        // 依赖注入
        this._apiClient = options.apiClient;
        this._authManager = options.authManager;
        this._updateApiResponse = options.updateApiResponse;
        
        // DOM 元素 ID
        this._tasksListId = options.tasksListId || 'tasksList';
        this._createFormId = options.createFormId || 'createTaskForm';
        
        // 内部状态
        this._tasks = [];
        this._currentFilter = 'all'; // 'all', 'active', 'completed'
        this._isInitialized = false;
        
        // 验证依赖
        this._validateDependencies();
    }

    /**
     * 初始化任务管理器
     * @returns {Promise<void>}
     */
    async initialize() {
        if (this._isInitialized) {
            console.log('TASK_MANAGER: 已经初始化，跳过重复初始化');
            return;
        }

        console.log('TASK_MANAGER: 开始初始化任务管理器');

        try {
            // 绑定事件处理器
            this._bindEventHandlers();
            
            // 如果用户已认证，获取任务列表
            if (this._authManager.isAuthenticated()) {
                await this.fetchTasks();
            } else {
                // 显示未认证状态
                this._renderUnauthenticatedState();
            }
            
            this._isInitialized = true;
            console.log('TASK_MANAGER: 任务管理器初始化完成');
            
        } catch (error) {
            console.error('TASK_MANAGER: 初始化失败:', error.message);
            this._showError('任务管理器初始化失败: ' + error.message);
        }
    }

    /**
     * 获取所有任务
     * @returns {Promise<void>}
     */
    async fetchTasks() {
        console.log('TASK_MANAGER: 开始获取任务列表');

        // 检查认证状态
        if (!this._authManager.isAuthenticated()) {
            this._renderUnauthenticatedState();
            this._updateApiResponse('获取任务列表', null, { error: '未认证' });
            return;
        }

        // 显示加载状态
        this._renderLoadingState();

        try {
            // 使用 API 客户端发起请求（自动包含认证头）
            const data = await this._apiClient.get('/tasks');

            // 更新 API 响应显示
            this._updateApiResponse('获取任务列表', { status: 200, statusText: 'OK' }, data);

            // 存储任务数据并渲染
            this._tasks = data;
            this._renderTasks();

            console.log('TASK_MANAGER: 成功获取任务列表，任务数量:', data.length);

        } catch (error) {
            console.error('TASK_MANAGER: 获取任务失败:', error.message);
            this._showError('获取任务失败: ' + error.message);
            this._updateApiResponse('获取任务列表', null, { error: error.message });
        }
    }

    /**
     * 创建新任务
     * @param {Object} taskData - 任务数据
     * @param {string} taskData.title - 任务标题
     * @param {string} taskData.description - 任务描述
     * @param {boolean} taskData.completed - 是否已完成
     * @returns {Promise<void>}
     */
    async createTask(taskData) {
        console.log('TASK_MANAGER: 开始创建任务:', taskData.title);

        // 检查认证状态
        if (!this._authManager.isAuthenticated()) {
            this._updateApiResponse('创建任务', null, { error: '请先登录' });
            return;
        }

        // 输入验证
        if (!taskData.title || taskData.title.trim() === '') {
            this._showError('任务标题不能为空');
            return;
        }

        // 构建请求数据
        const payload = {
            title: taskData.title.trim(),
            description: taskData.description ? taskData.description.trim() : null,
            completed: Boolean(taskData.completed)
        };

        try {
            // 发送创建请求
            const data = await this._apiClient.post('/tasks', payload);

            // 更新 API 响应显示
            this._updateApiResponse('创建任务', { status: 201, statusText: 'Created' }, data);

            // 刷新任务列表
            await this.fetchTasks();

            console.log('TASK_MANAGER: 任务创建成功:', data.id);

        } catch (error) {
            console.error('TASK_MANAGER: 创建任务失败:', error.message);
            this._showError('创建任务失败: ' + error.message);
            this._updateApiResponse('创建任务', null, { error: error.message });
        }
    }

    /**
     * 查看单个任务详情
     * @param {string} taskId - 任务 ID
     * @returns {Promise<void>}
     */
    async viewTask(taskId) {
        console.log('TASK_MANAGER: 查看任务详情:', taskId);

        // 检查认证状态
        if (!this._authManager.isAuthenticated()) {
            this._updateApiResponse(`查看任务 ${taskId}`, null, { error: '请先登录' });
            return;
        }

        try {
            // 发送查看请求
            const data = await this._apiClient.get(`/tasks/${taskId}`);

            // 更新 API 响应显示
            this._updateApiResponse(`查看任务 ${taskId}`, { status: 200, statusText: 'OK' }, data);

        } catch (error) {
            console.error('TASK_MANAGER: 查看任务失败:', error.message);
            this._updateApiResponse(`查看任务 ${taskId}`, null, { error: error.message });
        }
    }

    /**
     * 更新任务
     * @param {string} taskId - 任务 ID
     * @param {Object} updateData - 更新数据
     * @returns {Promise<void>}
     */
    async updateTask(taskId, updateData) {
        console.log('TASK_MANAGER: 开始更新任务:', taskId, updateData);

        // 检查认证状态
        if (!this._authManager.isAuthenticated()) {
            this._updateApiResponse(`更新任务 ${taskId}`, null, { error: '请先登录' });
            return;
        }

        try {
            // 发送更新请求
            const data = await this._apiClient.put(`/tasks/${taskId}`, updateData);

            // 更新 API 响应显示
            this._updateApiResponse(`更新任务 ${taskId}`, { status: 200, statusText: 'OK' }, data);

            // 刷新任务列表
            await this.fetchTasks();

            console.log('TASK_MANAGER: 任务更新成功:', taskId);

        } catch (error) {
            console.error('TASK_MANAGER: 更新任务失败:', error.message);
            this._showError('更新任务失败: ' + error.message);
            this._updateApiResponse(`更新任务 ${taskId}`, null, { error: error.message });
        }
    }

    /**
     * 删除任务
     * @param {string} taskId - 任务 ID
     * @returns {Promise<void>}
     */
    async deleteTask(taskId) {
        console.log('TASK_MANAGER: 开始删除任务:', taskId);

        // 检查认证状态
        if (!this._authManager.isAuthenticated()) {
            this._updateApiResponse(`删除任务 ${taskId}`, null, { error: '请先登录' });
            return;
        }

        try {
            // 发送删除请求
            await this._apiClient.delete(`/tasks/${taskId}`);

            // 更新 API 响应显示
            this._updateApiResponse(`删除任务 ${taskId}`, { status: 204, statusText: 'No Content' }, { message: '删除成功' });

            // 刷新任务列表
            await this.fetchTasks();

            console.log('TASK_MANAGER: 任务删除成功:', taskId);

        } catch (error) {
            console.error('TASK_MANAGER: 删除任务失败:', error.message);
            this._showError('删除任务失败: ' + error.message);
            this._updateApiResponse(`删除任务 ${taskId}`, null, { error: error.message });
        }
    }

    /**
     * 设置任务筛选器
     * @param {string} filter - 筛选器类型 ('all', 'active', 'completed')
     */
    setFilter(filter) {
        console.log('TASK_MANAGER: 设置筛选器:', filter);
        
        if (!['all', 'active', 'completed'].includes(filter)) {
            console.warn('TASK_MANAGER: 无效的筛选器类型:', filter);
            return;
        }

        this._currentFilter = filter;
        this._updateFilterButtons();
        this._renderTasks();
    }

    /**
     * 切换任务编辑模式
     * @param {string} taskId - 任务 ID
     */
    toggleEditMode(taskId) {
        console.log('TASK_MANAGER: 切换编辑模式:', taskId);

        const displayDiv = document.getElementById(`task-display-${taskId}`);
        const editDiv = document.getElementById(`task-edit-${taskId}`);
        const controlsDiv = document.getElementById(`task-controls-${taskId}`);
        const task = this._tasks.find(t => t.id === taskId);

        if (!displayDiv || !editDiv || !controlsDiv || !task) {
            console.error('TASK_MANAGER: 找不到任务元素或任务数据:', taskId);
            return;
        }

        // 切换显示和编辑区域的可见性
        if (displayDiv.style.display !== 'none') {
            // 进入编辑模式
            displayDiv.style.display = 'none';
            editDiv.style.display = 'block';
            // 更新按钮为"保存"和"取消"
            controlsDiv.innerHTML = `
                <button onclick="taskManager.handleSaveTask('${taskId}')">保存</button>
                <button onclick="taskManager.cancelEdit('${taskId}')">取消</button>
            `;
        } else {
            // 退出编辑模式
            this._exitEditMode(taskId, task);
        }
    }

    /**
     * 取消编辑
     * @param {string} taskId - 任务 ID
     */
    cancelEdit(taskId) {
        console.log('TASK_MANAGER: 取消编辑:', taskId);
        this.toggleEditMode(taskId);
    }

    /**
     * 处理保存任务
     * @param {string} taskId - 任务 ID
     * @returns {Promise<void>}
     */
    async handleSaveTask(taskId) {
        console.log('TASK_MANAGER: 保存任务编辑:', taskId);

        // 获取编辑输入框中的新值
        const titleInput = document.getElementById(`edit-title-${taskId}`);
        const descInput = document.getElementById(`edit-desc-${taskId}`);

        if (!titleInput || !descInput) {
            console.error('TASK_MANAGER: 找不到编辑输入框:', taskId);
            return;
        }

        const newTitle = titleInput.value.trim();
        const newDescription = descInput.value.trim();

        // 验证输入
        if (!newTitle) {
            this._showError('任务标题不能为空');
            return;
        }

        // 构建更新数据
        const updateData = {
            title: newTitle,
            description: newDescription || null
        };

        // 调用更新任务方法
        await this.updateTask(taskId, updateData);
    }

    /**
     * 确认删除任务
     * @param {string} taskId - 任务 ID
     * @param {string} taskTitle - 任务标题
     */
    confirmDelete(taskId, taskTitle) {
        if (confirm(`确定要删除任务 "${taskTitle}" 吗？`)) {
            this.deleteTask(taskId);
        }
    }

    // ===== 私有方法 =====

    /**
     * 验证依赖项
     * @private
     */
    _validateDependencies() {
        if (!this._apiClient) {
            throw new Error('TASK_MANAGER: ApiClient 实例是必需的');
        }
        if (!this._authManager) {
            throw new Error('TASK_MANAGER: AuthManager 实例是必需的');
        }
        if (typeof this._updateApiResponse !== 'function') {
            throw new Error('TASK_MANAGER: updateApiResponse 回调函数是必需的');
        }
    }

    /**
     * 绑定事件处理器
     * @private
     */
    _bindEventHandlers() {
        // 绑定创建任务表单提交事件
        const createForm = document.getElementById(this._createFormId);
        if (createForm) {
            createForm.addEventListener('submit', (event) => this._handleCreateTaskForm(event));
        }

        // 绑定刷新按钮点击事件
        const refreshBtn = document.getElementById('refreshTasksBtn');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', () => this.fetchTasks());
        }

        console.log('TASK_MANAGER: 事件处理器绑定完成');
    }

    /**
     * 处理创建任务表单提交
     * @param {Event} event - 表单提交事件
     * @private
     */
    async _handleCreateTaskForm(event) {
        event.preventDefault();

        // 获取表单数据
        const titleInput = document.getElementById('title');
        const descInput = document.getElementById('description');
        const completedInput = document.getElementById('completed');

        if (!titleInput || !descInput || !completedInput) {
            console.error('TASK_MANAGER: 找不到表单输入元素');
            return;
        }

        const taskData = {
            title: titleInput.value,
            description: descInput.value,
            completed: completedInput.checked
        };

        // 创建任务
        await this.createTask(taskData);

        // 清空表单
        titleInput.value = '';
        descInput.value = '';
        completedInput.checked = false;
    }

    /**
     * 渲染任务列表
     * @private
     */
    _renderTasks() {
        const tasksList = document.getElementById(this._tasksListId);
        if (!tasksList) {
            console.error('TASK_MANAGER: 找不到任务列表容器');
            return;
        }

        // 清空当前列表
        tasksList.innerHTML = '';

        // 根据当前筛选器过滤任务
        const filteredTasks = this._getFilteredTasks();

        // 如果没有任务，显示提示信息
        if (filteredTasks.length === 0) {
            const message = this._currentFilter === 'all' ? '没有任务' : '没有符合条件的任务';
            tasksList.innerHTML = `<li>${message}</li>`;
            return;
        }

        // 渲染每个任务
        filteredTasks.forEach(task => {
            const li = document.createElement('li');
            li.id = `task-${task.id}`;
            li.className = `task-item ${task.completed ? 'completed' : ''}`;
            li.innerHTML = this._buildTaskContent(task);
            tasksList.appendChild(li);
        });

        console.log('TASK_MANAGER: 任务列表渲染完成，显示任务数量:', filteredTasks.length);
    }

    /**
     * 获取过滤后的任务列表
     * @returns {Array} 过滤后的任务数组
     * @private
     */
    _getFilteredTasks() {
        return this._tasks.filter(task => {
            switch (this._currentFilter) {
                case 'completed':
                    return task.completed;
                case 'active':
                    return !task.completed;
                case 'all':
                default:
                    return true;
            }
        });
    }

    /**
     * 构建单个任务项的 HTML 内容
     * @param {Object} task - 任务对象
     * @returns {string} 任务项的 HTML 字符串
     * @private
     */
    _buildTaskContent(task) {
        return `
            <div id="task-display-${task.id}">
                <h4>${this._escapeHtml(task.title)}</h4>
                <p>${task.description ? this._escapeHtml(task.description) : '(无描述)'}</p>
                <p>状态: <strong>${task.completed ? '已完成' : '未完成'}</strong></p>
                <p class="timestamp">创建于: ${this._formatDate(task.created_at)} | 更新于: ${this._formatDate(task.updated_at)}</p>
            </div>
            <div id="task-edit-${task.id}" style="display:none;">
                <input type="text" value="${this._escapeHtml(task.title)}" class="edit-input" id="edit-title-${task.id}">
                <textarea class="edit-input" id="edit-desc-${task.id}">${task.description ? this._escapeHtml(task.description) : ''}</textarea>
            </div>
            <div class="task-controls" id="task-controls-${task.id}">
                ${this._buildTaskControls(task)}
            </div>
        `;
    }

    /**
     * 构建任务控制按钮的 HTML 内容
     * @param {Object} task - 任务对象
     * @returns {string} 控制按钮的 HTML 字符串
     * @private
     */
    _buildTaskControls(task) {
        const toggleText = task.completed ? '标记为未完成' : '标记为已完成';
        const toggleValue = !task.completed;

        return `
            <button onclick="taskManager.viewTask('${task.id}')">查看</button>
            <button onclick="taskManager.toggleEditMode('${task.id}')">编辑</button>
            <button onclick="taskManager.updateTask('${task.id}', { completed: ${toggleValue} })">${toggleText}</button>
            <button class="delete-btn" onclick="taskManager.confirmDelete('${task.id}', '${this._escapeHtml(task.title)}')">删除</button>
        `;
    }

    /**
     * 退出编辑模式
     * @param {string} taskId - 任务 ID
     * @param {Object} task - 任务对象
     * @private
     */
    _exitEditMode(taskId, task) {
        const displayDiv = document.getElementById(`task-display-${taskId}`);
        const editDiv = document.getElementById(`task-edit-${taskId}`);
        const controlsDiv = document.getElementById(`task-controls-${taskId}`);

        if (displayDiv && editDiv && controlsDiv) {
            displayDiv.style.display = 'block';
            editDiv.style.display = 'none';
            controlsDiv.innerHTML = this._buildTaskControls(task);
        }
    }

    /**
     * 更新筛选按钮状态
     * @private
     */
    _updateFilterButtons() {
        // 移除所有按钮的激活状态
        document.querySelectorAll('.filter-btn').forEach(btn => {
            btn.classList.remove('active');
        });

        // 激活当前筛选器对应的按钮
        const activeBtn = document.querySelector(`.filter-btn[onclick="setFilter('${this._currentFilter}')"]`);
        if (activeBtn) {
            activeBtn.classList.add('active');
        }
    }

    /**
     * 渲染未认证状态
     * @private
     */
    _renderUnauthenticatedState() {
        const tasksList = document.getElementById(this._tasksListId);
        if (tasksList) {
            tasksList.innerHTML = '<li>请先登录以查看任务</li>';
        }
    }

    /**
     * 渲染加载状态
     * @private
     */
    _renderLoadingState() {
        const tasksList = document.getElementById(this._tasksListId);
        if (tasksList) {
            tasksList.innerHTML = '<li>正在加载...</li>';
        }
    }

    /**
     * 显示错误信息
     * @param {string} message - 错误消息
     * @private
     */
    _showError(message) {
        const tasksList = document.getElementById(this._tasksListId);
        if (tasksList) {
            tasksList.innerHTML = `<li>错误: ${this._escapeHtml(message)}</li>`;
        }
        console.error('TASK_MANAGER:', message);
    }

    /**
     * 转义 HTML 特殊字符
     * @param {string} text - 要转义的文本
     * @returns {string} 转义后的文本
     * @private
     */
    _escapeHtml(text) {
        if (!text) return '';
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    /**
     * 格式化日期
     * @param {string} timestamp - 时间戳字符串
     * @returns {string} 格式化后的日期字符串
     * @private
     */
    _formatDate(timestamp) {
        if (!timestamp) {
            return 'N/A';
        }

        try {
            const date = new Date(timestamp);
            if (isNaN(date.getTime())) {
                return '无效日期';
            }
            return date.toLocaleString();
        } catch (error) {
            console.error('TASK_MANAGER: 日期格式化失败:', error);
            return '无效日期';
        }
    }
}

// 导出 TaskManager 类，支持 ES6 模块和全局使用
if (typeof module !== 'undefined' && module.exports) {
    module.exports = TaskManager;
} else {
    window.TaskManager = TaskManager;
}
