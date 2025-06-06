# 服务层 (Service) 流程图

本文档使用 Mermaid 语法描述服务层中关键函数的执行流程，重点体现与 SeaORM 和认证逻辑的交互。

## 1. `AuthService::register_user` - 用户注册流程

```mermaid
graph TD
    A["控制器层调用 `AuthService::register_user(db, payload)`"] --> B{开始 `register_user`};
    B --> C{"`user::Entity::find().filter(username).one(db).await` (检查用户名是否存在)"};
    C -- "查询成功" --> D{用户是否已存在?};
    D -- "是" --> E["返回 `Err(AppError::UsernameAlreadyExists)`"];
    D -- "否" --> F["生成 salt (`SaltString::generate`)"];
    F --> G["哈希密码 (Argon2 `hash_password`)"];
    G --> H["创建 `user::ActiveModel` (设置 `username`, `password_hash`)"];
    H --> I{"`active_model.insert(db).await` (插入新用户)"};
    I -- "插入成功 (返回 `user::Model`)" --> J["返回 `Ok(user::Model)`"];
    I -- "数据库错误 (DbErr)" --> K["通过 `?` 或 `map_err` 返回 `Err(AppError::DatabaseError)`"];
    E --> Z["结束"]; J --> Z; K --> Z;
```

**说明**:
1.  控制器调用服务函数，传入数据库连接和注册载荷。
2.  服务层首先查询数据库，检查用户名是否已被占用。
3.  如果用户名已存在，返回 `UsernameAlreadyExists` 错误。
4.  如果用户名可用，生成密码盐，并使用 Argon2 算法哈希用户提供的密码。
5.  创建一个 `user::ActiveModel`，并设置用户名和哈希后的密码。
6.  调用 SeaORM 的 `insert` 方法将新用户数据存入数据库。
7.  根据数据库操作结果，返回包含新创建用户模型的 `Ok`，或相应的数据库错误。

## 2. `AuthService::login_user` - 用户登录流程

```mermaid
graph TD
    A["控制器层调用 `AuthService::login_user(db, config, payload)`"] --> B{开始 `login_user`};
    B --> C{"`user::Entity::find().filter(username).one(db).await` (根据用户名查找用户)"};
    C -- "查询成功" --> D{用户是否存在?};
    D -- "否 (返回 `None`)" --> E["通过 `.ok_or_else` 返回 `Err(AppError::UserNotFound)`"];
    D -- "是 (返回 `Some(user_model)`) " --> F["从 `user_model.password_hash` 创建 `PasswordHash` 实例"];
    F -- "解析哈希失败" --> F_ERR["返回 `Err(AppError::PasswordHashingError)`"];
    F -- "解析成功" --> G["验证密码 (Argon2 `verify_password` 使用提供的密码和存储的哈希)"];
    G -- "验证失败 (密码不匹配)" --> H["返回 `Err(AppError::InvalidPassword)`"];
    G -- "验证成功" --> I["准备 JWT `Claims` (设置 `sub`, `username`, `exp`, `iat`)"];
    I --> J{"`jsonwebtoken::encode(claims, &EncodingKey::from_secret(config.jwt_secret))`"};
    J -- "编码成功 (返回 `token_string`)" --> K["返回 `Ok(token_string)`"];
    J -- "编码失败" --> L["返回 `Err(AppError::JwtCreationError)`"];
    E --> Z["结束"]; F_ERR --> Z; H --> Z; K --> Z; L --> Z;
```

**说明**:
1.  控制器调用服务函数，传入数据库连接、应用配置 (含 JWT 密钥) 和登录载荷。
2.  服务层根据用户名查询用户。
3.  如果用户未找到，返回 `UserNotFound` 错误。
4.  如果用户找到，解析存储的密码哈希字符串。
5.  使用 Argon2 验证提供的密码与存储的哈希是否匹配。
6.  如果密码不匹配，返回 `InvalidPassword` 错误。
7.  如果密码匹配，创建 JWT `Claims`，包括用户ID (`sub`)、用户名、过期时间 (`exp`) 和签发时间 (`iat`)。
8.  使用 `jsonwebtoken`库和配置中的 JWT 密钥及算法 (HS512) 对 `Claims` 进行编码，生成 JWT 字符串。
9.  返回生成的 JWT 字符串或相应的错误。

## 3. `TaskService::create_task` - 创建任务流程

```mermaid
graph TD
    A["控制器层调用 `TaskService::create_task(db, payload)`"] --> B{开始 `create_task`};
    B --> C["创建 `task::ActiveModel` (调用 `task::ActiveModel::new()`)"];
    C --> D["通过 `Set()` 方法设置 `active_model` 的 `title`, `description`, `completed` 字段 (来自 `payload`)"];
    D --> E{"`active_model.insert(db).await` (将任务插入数据库)"};
    E -- "插入成功 (返回 `task::Model`)" --> F["返回 `Ok(task::Model)`"];
    E -- "数据库错误 (DbErr)" --> G["通过 `map_err` 返回 `Err(AppError::DatabaseError)`"];
    F --> Z["结束"]; G --> Z;
```
**说明**:
- `task::ActiveModel::new()` 在创建时，通过其 `ActiveModelBehavior` 实现，会自动设置 `created_at` 和 `updated_at` 的初始值。
- 服务层主要负责将 `CreateTaskPayload` 的数据映射到 `ActiveModel` 的相应字段。
- 调用 SeaORM 的 `insert` 方法完成数据库操作。

## 4. `TaskService::get_task_by_id` - 获取特定任务流程

```mermaid
graph TD
    A["控制器层调用 `TaskService::get_task_by_id(db, id)`"] --> B{开始 `get_task_by_id`};
    B --> C{"`task::Entity::find_by_id(id).one(db).await` (查询任务)"};
    C -- "查询成功 (返回 `Result<Option<task::Model>, DbErr>`)" --> D{任务是否存在 (`Option<task::Model>`) ?};
    D -- "是 (`Some(model)`)" --> E["返回 `Ok(model)`"];
    D -- "否 (`None`)" --> F["返回 `Err(AppError::TaskNotFound)`"];
    C -- "数据库错误 (DbErr)" --> G["通过 `map_err` 返回 `Err(AppError::DatabaseError)`"];
    E --> Z["结束"]; F --> Z; G --> Z;
```
**说明**:
- 使用 SeaORM 的 `find_by_id()` 方法按主键查询。
- `.one(db)` 表示期望最多一条记录。
- 结果是 `Option<task::Model>`，如果数据库中没有对应 ID 的记录，则为 `None`，服务层将其转换为 `TaskNotFound` 错误。

(更新和删除任务的流程与 `get_task_by_id` 类似，涉及先查找，然后分别调用 `active_model.update(db)` 或 `entity.delete_by_id(id).exec(db)`)