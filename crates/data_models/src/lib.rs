use chrono::{DateTime, Utc};
use common::{TaskInstance, InstanceStatus, ServiceError};
use config_manager::get_config;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

type PgPool = Pool<ConnectionManager<PgConnection>>;

pub struct Db {
    pool: PgPool,
}

impl Db {
    pub fn new() -> Result<Self, ServiceError> {
        let cfg = get_config();
        let manager = ConnectionManager::<PgConnection>::new(&cfg.database.url);
        let pool = Pool::builder().build(manager)?;
        let mut conn = pool.get()?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| ServiceError::Other(format!("Migration error: {}", e)))?;
        Ok(Db { pool })
    }

    pub fn get_conn(&self) -> Result<PooledConnection<ConnectionManager<PgConnection>>, ServiceError> {
        let conn = self.pool.get()?;
        Ok(conn)
    }

    pub fn list_instances(&self) -> Result<Vec<TaskInstance>, ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut c = self.get_conn()?;
        let rows = instances.load::<RowInstance>(&mut c)?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub fn create_instance(&self, inst: &TaskInstance) -> Result<TaskInstance, ServiceError> {
        use crate::schema::instances;
        let mut c = self.get_conn()?;
        let new = NewInstance::from(inst);
        diesel::insert_into(instances::table)
            .values(&new)
            .get_result::<RowInstance>(&mut c)
            .map(|r| r.into())
            .map_err(ServiceError::from)
    }

    pub fn update_instance(&self, id_: i32, status_: InstanceStatus, expires_at_: DateTime<Utc>)
                           -> Result<(), ServiceError>
    {
        use crate::schema::instances::dsl::*;
        let mut c = self.get_conn()?;
        diesel::update(instances.filter(id.eq(id_)))
            .set((status.eq(status_.as_str()), expires_at.eq(expires_at_)))
            .execute(&mut c)?;
        Ok(())
    }

    pub fn find_by_port(&self, port_: i32) -> Result<Option<TaskInstance>, ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut c = self.get_conn()?;
        let opt = instances.filter(port.eq(port_)).first::<RowInstance>(&mut c).optional()?;
        Ok(opt.map(|r| r.into()))
    }
}


pub mod schema {
    diesel::table! {
        tasks (name) {
            name -> Text,
            dockerfile_path -> Text,
            created_at -> Timestamptz,
        }
    }

    diesel::table! {
        users (id) {
            id -> Int4,
            username -> Text,
            created_at -> Timestamptz,
        }
    }

    diesel::table! {
        sessions (id) {
            id -> Text,
            user_id -> Int4,
            created_at -> Timestamptz,
            expires_at -> Timestamptz,
        }
    }

    diesel::table! {
        instances (id) {
            id -> Int4,
            task_name -> Text,
            container_id -> Text,
            port -> Int4,
            created_at -> Timestamptz,
            expires_at -> Timestamptz,
            status -> Text,
        }
    }
}

use schema::instances;

#[derive(Queryable)]
struct RowInstance {
    id: i32,
    task_name: String,
    container_id: String,
    port: i32,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    status: String,
}

#[derive(Insertable)]
#[diesel(table_name = instances)]
struct NewInstance {
    task_name: String,
    container_id: String,
    port: i32,
    expires_at: DateTime<Utc>,
    status: String,
}

impl From<&TaskInstance> for NewInstance {
    fn from(t: &TaskInstance) -> Self {
        NewInstance {
            task_name: t.task_name.clone(),
            container_id: t.container_id.clone(),
            port: t.port as i32,
            expires_at: t.expires_at,
            status: t.status.as_str().to_owned(),
        }
    }
}

impl From<RowInstance> for TaskInstance {
    fn from(r: RowInstance) -> Self {
        TaskInstance {
            id: r.id,
            task_name: r.task_name,
            container_id: r.container_id,
            port: r.port as u16,
            created_at: r.created_at,
            expires_at: r.expires_at,
            status: match r.status.as_str() {
                "Running" => InstanceStatus::Running,
                "Stopped"  => InstanceStatus::Stopped,
                _         => InstanceStatus::Expired,
            },
        }
    }
}
