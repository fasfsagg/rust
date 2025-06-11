//! `task_repository_tests.rs`
//!
//! 这是一个集成测试文件，专门用于测试 `TaskRepository` 的功能。
//!
//! ## 测试策略
//! - **独立数据库**: 每个测试用例都会使用 `common::spawn_app` 来启动一个
//!   拥有独立、干净的数据库的应用实例。这确保了测试之间的隔离性。
//! - **真实数据库交互**: 这些测试会真实地访问数据库（在测试环境中），
//!   验证 `TaskRepository` 中的 SQL 查询和操作是否按预期工作。
//! - **断言**: 每个测试都会对数据库操作的结果进行断言，例如，在创建
//!   一个任务后，会立即查询该任务以确认其已成功存入数据库。

// 引入通用测试辅助模块
mod common;

use fake::{ Fake, Faker };
use migration::task_entity;
use sea_orm::{ prelude::Uuid, Set };
use axum_tutorial::app::repository::task_repository::TaskRepository;

/// 测试 `create` 函数是否能成功创建一个任务。
#[tokio::test]
async fn test_create_task_success() {
    // 准备 (Arrange)
    // 1. 启动应用和数据库
    let app = common::spawn_app().await;
    // 2. 准备测试数据
    let new_task_description: String = Faker.fake();
    let new_task_data = task_entity::ActiveModel {
        description: Set(Some(new_task_description.clone())),
        title: Set(format!("测试任务-{}", Faker.fake::<String>())),
        ..Default::default() // 其他字段使用默认值
    };

    // 行动 (Act)
    // 3. 调用被测试的函数
    let created_task = TaskRepository::create(&app.db_connection, new_task_data).await.unwrap();

    // 断言 (Assert)
    // 4. 验证结果
    // 验证返回的模型是否包含了正确的数据
    assert_eq!(created_task.description, Some(new_task_description.clone()));

    // 5. 直接从数据库中查询，再次确认数据已成功写入
    let found_task = TaskRepository::find_by_id(&app.db_connection, created_task.id).await
        .unwrap()
        .unwrap();
    assert_eq!(found_task.description, Some(new_task_description));
}

/// 测试 `find_by_id` 函数在任务存在时能成功返回任务。
#[tokio::test]
async fn test_find_by_id_success() {
    // 准备
    let app = common::spawn_app().await;
    println!("测试 test_find_by_id_success: 生成测试应用");

    // 先创建一个任务
    let task_title = format!("测试任务-{}", Faker.fake::<String>());
    let task_desc = Some(Faker.fake::<String>());
    println!(
        "测试 test_find_by_id_success: 准备创建任务 - 标题: {}, 描述: {:?}",
        task_title,
        task_desc
    );

    let new_task_data = task_entity::ActiveModel {
        description: Set(task_desc.clone()),
        title: Set(task_title.clone()),
        ..Default::default()
    };

    let created_task = TaskRepository::create(&app.db_connection, new_task_data).await.unwrap();
    println!("测试 test_find_by_id_success: 创建任务成功 - ID: {}", created_task.id);

    // 行动
    let found_task_result = TaskRepository::find_by_id(&app.db_connection, created_task.id).await;

    if let Err(e) = &found_task_result {
        println!("测试 test_find_by_id_success: 查找任务失败 - 错误: {}", e);
    }

    let found_task = found_task_result.unwrap();

    if found_task.is_none() {
        println!("测试 test_find_by_id_success: 无法找到刚创建的任务，ID: {}", created_task.id);
        // 尝试查询所有任务，检查数据库状态
        let all_tasks = TaskRepository::find_all(&app.db_connection).await.unwrap();
        println!("测试 test_find_by_id_success: 数据库中现有任务数量: {}", all_tasks.len());
        for task in all_tasks {
            println!("测试 test_find_by_id_success: - ID: {}, 标题: {}", task.id, task.title);
        }
    }

    let found_task = found_task.expect("任务应该存在但未找到");

    // 断言
    assert_eq!(found_task.id, created_task.id);
    assert_eq!(found_task.description, created_task.description);
}

/// 测试 `find_by_id` 函数在任务不存在时返回 `None`。
#[tokio::test]
async fn test_find_by_id_not_found() {
    // 准备
    let app = common::spawn_app().await;
    let non_existent_id = Uuid::new_v4();

    // 行动
    let result = TaskRepository::find_by_id(&app.db_connection, non_existent_id).await.unwrap();

    // 断言
    assert!(result.is_none());
}

/// 测试 `find_all` 函数能返回所有存在的任务。
#[tokio::test]
async fn test_find_all_success() {
    // 准备
    let app = common::spawn_app().await;
    println!("测试 test_find_all_success: 生成测试应用");

    // 检查开始时数据库是否为空
    let initial_tasks = TaskRepository::find_all(&app.db_connection).await.unwrap();
    println!("测试 test_find_all_success: 初始任务数量: {}", initial_tasks.len());
    if !initial_tasks.is_empty() {
        for task in &initial_tasks {
            println!(
                "测试 test_find_all_success: 发现预存在任务 - ID: {}, 标题: {}",
                task.id,
                task.title
            );
        }
        // 清理现有任务
        println!("测试 test_find_all_success: 清理预存在的任务");
        common::cleanup_db(&app.db_connection).await.unwrap();
    }

    // 创建两个任务
    println!("测试 test_find_all_success: 开始创建第一个任务");
    let task1_title = format!("测试任务1-{}", Faker.fake::<String>());
    TaskRepository::create(&app.db_connection, task_entity::ActiveModel {
        description: Set(Some(Faker.fake())),
        title: Set(task1_title.clone()),
        ..Default::default()
    }).await.unwrap();
    println!("测试 test_find_all_success: 创建第一个任务成功 - 标题: {}", task1_title);

    println!("测试 test_find_all_success: 开始创建第二个任务");
    let task2_title = format!("测试任务2-{}", Faker.fake::<String>());
    TaskRepository::create(&app.db_connection, task_entity::ActiveModel {
        description: Set(Some(Faker.fake())),
        title: Set(task2_title.clone()),
        ..Default::default()
    }).await.unwrap();
    println!("测试 test_find_all_success: 创建第二个任务成功 - 标题: {}", task2_title);

    // 行动
    let tasks = TaskRepository::find_all(&app.db_connection).await.unwrap();
    println!("测试 test_find_all_success: 查询到的任务数量: {}", tasks.len());
    for task in &tasks {
        println!("测试 test_find_all_success: - ID: {}, 标题: {}", task.id, task.title);
    }

    // 断言
    assert_eq!(tasks.len(), 2, "应该有两个任务，但实际查询到 {} 个任务", tasks.len());
}

/// 测试 `update` 函数能成功更新一个任务。
#[tokio::test]
async fn test_update_task_success() {
    // 准备
    let app = common::spawn_app().await;
    // 创建一个任务
    let created_task = TaskRepository::create(&app.db_connection, task_entity::ActiveModel {
        description: Set(Some("Initial Description".to_string())),
        title: Set("测试任务-更新".to_string()),
        ..Default::default()
    }).await.unwrap();

    let updated_description = "Updated Description".to_string();
    let mut task_to_update: task_entity::ActiveModel = created_task.into();
    task_to_update.description = Set(Some(updated_description.clone()));

    // 行动
    let updated_task = TaskRepository::update(&app.db_connection, task_to_update).await.unwrap();

    // 断言
    assert_eq!(updated_task.description, Some(updated_description.clone()));

    // 再次从数据库确认
    let found_task = TaskRepository::find_by_id(&app.db_connection, updated_task.id).await
        .unwrap()
        .unwrap();
    assert_eq!(found_task.description, Some(updated_description));
}

/// 测试 `delete` 函数能成功删除一个任务。
#[tokio::test]
async fn test_delete_task_success() {
    // 准备
    let app = common::spawn_app().await;
    // 创建一个任务
    let created_task = TaskRepository::create(&app.db_connection, task_entity::ActiveModel {
        description: Set(Some(Faker.fake())),
        title: Set(format!("测试任务-删除-{}", Faker.fake::<String>())),
        ..Default::default()
    }).await.unwrap();

    // 行动
    let delete_result = TaskRepository::delete(&app.db_connection, created_task.id).await.unwrap();

    // 断言
    assert_eq!(delete_result.rows_affected, 1);

    // 确认任务已被删除
    let result = TaskRepository::find_by_id(&app.db_connection, created_task.id).await.unwrap();
    assert!(result.is_none());
}
