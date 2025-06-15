graph TD
    subgraph "用户操作"
        A[1. 用户在终端输入 `cargo run`]
    end

    subgraph "Axum 应用启动过程 (startup.rs)"
        A --> B{2. 应用初始化};
        B --> C[3. 调用 `Migrator::up(db_connection, ...)`];
    end

    subgraph "SeaORM 迁移核心逻辑"
        C --> D{4. 连接到数据库 (task_manager.db)};
        D --> E{5. 检查 `seaql_migrations` 表是否存在?};
        E -- 不存在 --> F[创建 `seaql_migrations` 表];
        E -- 存在 --> G;
        F --> G;
        
        G[6. 读取数据库 `seaql_migrations` 表<br>获取 **已应用的迁移版本** (数据库账本)];
        G --> H[7. 读取项目 `migration/src/` 目录<br>获取 **所有可用的迁移文件** (操作清单)];
        
        H --> I{8. 对比两个列表<br>找出未在“数据库账本”中的“操作清单”};
        
        I -- 发现未应用的迁移 --> J;
        I -- 所有迁移都已应用 --> K;
        
        subgraph "按时间顺序执行待处理的迁移"
            J(9. 获取第一个待处理迁移<br>`..._create_task_table.rs`) --> J1[执行其 `up()` 方法<br>在数据库中创建 `tasks` 表];
            J1 -- 成功 --> J2[在 `seaql_migrations` 表中插入一条记录<br>`version = 'm20250610_035426_...'`];
            
            J2 --> L(10. 获取下一个待处理迁移<br>`..._create_users_table.rs`);
            L --> L1[执行其 `up()` 方法<br>在数据库中创建 `users` 表];
            L1 -- 成功 --> L2[在 `seaql_migrations` 表中插入一条记录<br>`version = 'm20250615_045841_...'`];
            L2 --> K;
        end

    end

    subgraph "应用继续运行"
        K[11. 数据库 Schema 已是最新状态];
        K --> M[12. Axum 服务器成功启动<br>监听 127.0.0.1:3000];
    end

    %% Styling
    style F fill:#f9f,stroke:#333,stroke-width:2px;
    style J2 fill:#cff,stroke:#333,stroke-width:2px;
    style L2 fill:#cff,stroke:#333,stroke-width:2px;
