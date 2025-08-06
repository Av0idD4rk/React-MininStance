use data_models::Db;
use common::{TaskInstance, InstanceStatus};
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;

#[test]
fn test_crud_instance() {
    let db = Db::new().expect("DB init failed");

    {
        let mut conn = db.get_conn().expect("get_conn failed");
        sql_query(
            "INSERT INTO tasks (name, dockerfile_path) VALUES ($1, $2)
             ON CONFLICT (name) DO NOTHING",
        )
            .bind::<Text, _>("foo_task")
            .bind::<Text, _>("./tasks/foo_task")
            .execute(&mut conn)
            .expect("insert task");
    }

    // Prepare a TaskInstance struct
    let now = Utc::now();
    let inst = TaskInstance {
        id: 0,
        task_name: "foo_task".into(),
        container_id: "abc123".into(),
        port: 3000,
        created_at: now,
        expires_at: now + chrono::Duration::minutes(30),
        status: InstanceStatus::Running,
    };

    // Create
    let created = db.create_instance(&inst).expect("create");
    assert!(created.id > 0);

    // List
    let all = db.list_instances().expect("list");
    assert!(all.iter().any(|i| i.id == created.id));

    // Update
    db.update_instance(created.id, InstanceStatus::Stopped, Utc::now())
        .expect("update");
    let fetched = db.find_by_port(3000).expect("find").unwrap();
    assert_eq!(fetched.status, InstanceStatus::Stopped);
}
