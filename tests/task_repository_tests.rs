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

use axum_tutorial::app::repository::task_repository::{ TaskRepository, TaskRepositoryContract };
use fake::{ Fake, Faker };
use migration::task_entity;
use sea_orm::{ prelude::Uuid, Set };

/// 测试 `create` 函数是否能成功创建一个任务。
#[tokio::test]
async fn test_create_task_success() {
    // 准备 (Arrange)
    let app = common::spawn_app().await;
    let task_repo = TaskRepository::new(app.db_connection.clone());
    let new_task_title = format!("测试任务-{}", Faker.fake::<String>());
    let new_task_description: String = Faker.fake();
    let new_task_data = task_entity::ActiveModel {
        title: Set(new_task_title.clone()),
        description: Set(Some(new_task_description.clone())),
        ..Default::default()
    };

    // 行动 (Act)
    let created_task = task_repo.create(new_task_data).await.expect("创建任务失败");

    // 断言 (Assert)
    assert_eq!(created_task.title, new_task_title);
    assert_eq!(created_task.description.unwrap(), new_task_description);

    // 直接从数据库中查询，再次确认数据已成功写入
    let found_task = task_repo
        .find_by_id(created_task.id).await
        .expect("从数据库按ID查找任务失败")
        .expect("创建的任务未在数据库中找到");

    assert_eq!(found_task.id, created_task.id);
    assert_eq!(found_task.title, new_task_title);
    assert_eq!(found_task.description.unwrap(), new_task_description);
}

/// 测试 `find_by_id` 函数在任务存在时能成功返回任务。
#[tokio::test]
async fn test_find_by_id_success() {
    // 准备
    let app = common::spawn_app().await;
    let task_repo = TaskRepository::new(app.db_connection.clone());
    let task_title = format!("测试任务-{}", Faker.fake::<String>());
    let task_desc = Some(Faker.fake::<String>());

    // 先创建一个任务
    let created_task = task_repo
        .create(task_entity::ActiveModel {
            title: Set(task_title.clone()),
            description: Set(task_desc.clone()),
            ..Default::default()
        }).await
        .expect("为测试find_by_id而创建任务时失败");

    // 行动
    let found_task = task_repo
        .find_by_id(created_task.id).await
        .expect("执行find_by_id失败")
        .expect("任务应该存在但未找到");

    // 断言
    assert_eq!(found_task.id, created_task.id);
    assert_eq!(found_task.title, created_task.title);
    assert_eq!(found_task.description, task_desc);
}

/// 测试 `find_by_id` 函数在任务不存在时返回 `None`。
#[tokio::test]
async fn test_find_by_id_not_found() {
    // 准备
    let app = common::spawn_app().await;
    let task_repo = TaskRepository::new(app.db_connection.clone());
    let non_existent_id = Uuid::new_v4();

    // 行动
    let result = task_repo
        .find_by_id(non_existent_id).await
        .expect("执行find_by_id查询未找到的ID时失败");

    // 断言
    assert!(result.is_none());
}

/// 测试 `find_all` 函数能返回所有存在的任务。
#[tokio::test]
async fn test_find_all_success() {
    // 准备
    let app = common::spawn_app().await;
    let task_repo = TaskRepository::new(app.db_connection.clone());

    // 因为 spawn_app 保证了数据库是全新的，所以可以直接开始创建任务
    // 创建两个任务
    task_repo
        .create(task_entity::ActiveModel {
            title: Set(format!("测试任务1-{}", Faker.fake::<String>())),
            description: Set(Some(Faker.fake())),
            ..Default::default()
        }).await
        .expect("创建第一个任务失败");

    task_repo
        .create(task_entity::ActiveModel {
            title: Set(format!("测试任务2-{}", Faker.fake::<String>())),
            description: Set(Some(Faker.fake())),
            ..Default::default()
        }).await
        .expect("创建第二个任务失败");

    // 行动
    let tasks = task_repo.find_all().await.expect("执行find_all失败");

    // 断言
    assert_eq!(tasks.len(), 2, "数据库中应该有两个任务");
}

/// 测试 `update` 函数能成功更新一个任务。
#[tokio::test]
async fn test_update_task_success() {
    // 准备
    let app = common::spawn_app().await;
    let task_repo = TaskRepository::new(app.db_connection.clone());
    // 创建一个任务
    let created_task = task_repo
        .create(task_entity::ActiveModel {
            title: Set("测试任务-更新".to_string()),
            description: Set(Some("Initial Description".to_string())),
            ..Default::default()
        }).await
        .expect("为测试update而创建任务时失败");

    let updated_description = "Updated Description".to_string();
    let mut task_to_update: task_entity::ActiveModel = created_task.into();
    task_to_update.description = Set(Some(updated_description.clone()));

    // 行动
    let updated_task = task_repo.update(task_to_update).await.expect("更新任务失败");

    // 断言
    assert_eq!(updated_task.description.as_ref(), Some(&updated_description));

    // 再次从数据库确认
    let found_task = task_repo
        .find_by_id(updated_task.id).await
        .expect("为验证更新而查询任务时失败")
        .expect("更新后的任务在数据库中未找到");
    assert_eq!(found_task.description, Some(updated_description));
}

/// 测试 `delete` 函数能成功删除一个任务。
#[tokio::test]
async fn test_delete_task_success() {
    // 准备
    let app = common::spawn_app().await;
    let task_repo = TaskRepository::new(app.db_connection.clone());
    // 创建一个任务
    let created_task = task_repo
        .create(task_entity::ActiveModel {
            title: Set(format!("测试任务-删除-{}", Faker.fake::<String>())),
            description: Set(Some(Faker.fake())),
            ..Default::default()
        }).await
        .expect("为测试delete而创建任务时失败");

    // 行动
    let delete_result = task_repo.delete(created_task.id).await.expect("删除任务失败");

    // 断言
    assert_eq!(delete_result.rows_affected, 1);

    // 确认任务已被删除
    let result = task_repo.find_by_id(created_task.id).await.expect("为验证删除而查询任务时失败");
    assert!(result.is_none(), "任务删除后，在数据库中应该找不到");
}
