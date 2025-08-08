use chrono::{DateTime, Utc};
use common::{TaskInstance, InstanceStatus, ServiceError, User};
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

    #[deprecated]
    pub fn create_instance(&self, inst: &TaskInstance) -> Result<TaskInstance, ServiceError> {
        use crate::schema::instances;

        let mut conn = self.get_conn()?;

        let new_inst = NewInstance {
            task_name: inst.task_name.clone(),
            container_id: inst.container_id.clone(),
            expires_at: inst.expires_at,
            status: inst.status.as_str().to_string(),
            endpoint: inst.endpoint.clone(),
            user_id: inst.user_id,
        };

        let saved_row: RowInstance = diesel::insert_into(instances::table)
            .values(&new_inst)
            .get_result(&mut conn)?;

        Ok(saved_row.into())
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
    pub fn find_instance_by_id(&self, id_: i32) -> Result<Option<TaskInstance>, ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut conn = self.get_conn()?;
        let row = instances
            .filter(id.eq(id_))
            .first::<RowInstance>(&mut conn)
            .optional()?;
        Ok(row.map(|r| r.into()))
    }

    pub fn count_running_instances_for_user(&self, uid: i32) -> Result<i64, ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut c = self.get_conn()?;
        let cnt: i64 = instances
            .filter(user_id.eq(uid))
            .filter(status.eq("Running"))
            .count()
            .get_result(&mut c)?;
        Ok(cnt)
    }

    pub fn create_instance_for_user(
        &self,
        inst: &TaskInstance,
        uid: i32,
    ) -> Result<TaskInstance, ServiceError> {
        use crate::schema::instances;

        let mut conn = self.get_conn()?;

        let new_inst = NewInstance {
            task_name: inst.task_name.clone(),
            container_id: inst.container_id.clone(),
            expires_at: inst.expires_at,
            status: inst.status.as_str().to_string(),
            endpoint: inst.endpoint.clone(),
            user_id: uid,
        };

        let saved_row: RowInstance = diesel::insert_into(instances::table)
            .values(&new_inst)
            .get_result(&mut conn)?;

        Ok(saved_row.into())
    }


    pub fn find_or_create_user(&self, name: &str) -> Result<User, ServiceError> {
        let mut conn = self.get_conn()?;

        if let Some(row) = users::dsl::users
            .filter(users::dsl::username.eq(name))
            .first::<RowUser>(&mut conn)
            .optional()?
        {
            return Ok(User {
                id: row.id,
                username: row.username,
                created_at: row.created_at,
            });
        }

        let new = NewUser { username: name };
        let row = diesel::insert_into(users::table)
            .values(&new)
            .get_result::<RowUser>(&mut conn)?;
        Ok(User {
            id: row.id,
            username: row.username,
            created_at: row.created_at,
        })
    }
    pub fn find_valid_session_for_user(&self, uid: i32) -> Result<Option<String>, ServiceError> {
        use crate::schema::sessions::dsl::*;
        let mut conn = self.get_conn()?;
        let now = Utc::now();
        let token_opt = sessions
            .filter(user_id.eq(uid))
            .filter(expires_at.gt(now))
            .select(id)
            .first::<String>(&mut conn)
            .optional()?;
        Ok(token_opt)
    }

    /// Fetch the full session row for a given token, including its expiry.
    pub fn get_session(&self, token_str: &str) -> Result<Option<common::UserSession>, ServiceError> {
        use crate::schema::sessions::dsl::*;
        let mut conn = self.get_conn()?;
        // Query the raw row
        let opt_row = sessions
            .filter(id.eq(token_str))
            .first::<RowSession>(&mut conn)
            .optional()?;
        // Map RowSession → common::UserSession
        Ok(opt_row.map(|r| common::UserSession {
            session_id: r.id,
            user_id: r.user_id,
            created_at: r.created_at,
            expires_at: r.expires_at,
        }))
    }

    pub fn list_instances_for_user(&self, uid: i32) -> Result<Vec<TaskInstance>, ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut conn = self.get_conn()?;
        let rows = instances
            .filter(user_id.eq(uid))
            .filter(status.eq("Running"))
            .load::<RowInstance>(&mut conn)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
    pub fn create_session(
        &self,
        token_str: &str,
        uid: i32,
        expires_at_val: DateTime<Utc>,
    ) -> Result<(), ServiceError> {
        use crate::schema::sessions;
        let mut conn = self.get_conn()?;
        let new = NewSession {
            id: token_str,
            user_id: uid,
            expires_at: expires_at_val,
        };
        diesel::insert_into(sessions::table)
            .values(&new)
            .execute(&mut conn)?;
        Ok(())
    }

    pub fn validate_session(&self, token: &str) -> Result<Option<User>, ServiceError> {
        use crate::schema::{sessions, users};
        use diesel::prelude::*;

        let mut conn = self.get_conn()?;
        let now = Utc::now();

        let opt_row = sessions::table
            .inner_join(users::table)
            .filter(sessions::id.eq(token))
            .filter(sessions::expires_at.gt(now))
            .select((users::id, users::username, users::created_at))
            .first::<RowUser>(&mut conn)
            .optional()?;

        // Map RowUser → common::User
        Ok(opt_row.map(|r| User {
            id: r.id,
            username: r.username,
            created_at: r.created_at,
        }))
    }
    pub fn list_expired_instances(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<TaskInstance>, ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut conn = self.get_conn()?;
        let rows: Vec<RowInstance> = instances
            .filter(status.eq(InstanceStatus::Running.as_str()))
            .filter(expires_at.lt(now))
            .load(&mut conn)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
    pub fn update_instance_status(
        &self,
        inst_id: i32,
        new_status: InstanceStatus,
    ) -> Result<(), ServiceError> {
        use crate::schema::instances::dsl::*;
        let mut conn = self.get_conn()?;
        diesel::update(instances.filter(id.eq(inst_id)))
            .set(status.eq(new_status.as_str()))
            .execute(&mut conn)?;
        Ok(())
    }
    pub fn ensure_task(&self, name: &str, dockerfile_path: &str) -> Result<(), ServiceError> {
        use crate::schema::tasks;
        let mut conn = self.get_conn()?;
        diesel::insert_into(tasks::table)
            .values((
                tasks::dsl::name.eq(name),
                tasks::dsl::dockerfile_path.eq(dockerfile_path),
            ))
            .on_conflict(tasks::dsl::name)
            .do_nothing()
            .execute(&mut conn)?;
        Ok(())
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
            created_at -> Timestamptz,
            expires_at -> Timestamptz,
            status -> Text,
            endpoint -> Text,
            user_id -> Int4,
        }
    }
}
joinable!(sessions -> users (user_id));

// Allow both tables in the same query
allow_tables_to_appear_in_same_query!(
    sessions,
    users,
);

use crate::schema::{sessions, users, instances};

#[derive(Queryable)]
struct RowUser {
    id: i32,
    username: String,
    created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::users)]
struct NewUser<'a> {
    username: &'a str,
}

#[derive(Queryable)]
struct RowSession {
    id: String,
    user_id: i32,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::sessions)]
struct NewSession<'a> {
    id: &'a str,
    user_id: i32,
    expires_at: DateTime<Utc>,
}

#[derive(Queryable)]
struct RowInstance {
    id: i32,
    task_name: String,
    container_id: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    status: String,
    endpoint: String,
    user_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = instances)]
struct NewInstance {
    task_name: String,
    container_id: String,
    expires_at: DateTime<Utc>,
    status: String,
    endpoint: String,
    user_id: i32,
}

impl From<(&TaskInstance, i32)> for NewInstance {
    fn from((t, uid): (&TaskInstance, i32)) -> Self {
        NewInstance {
            task_name: t.task_name.clone(),
            container_id: t.container_id.clone(),
            expires_at: t.expires_at,
            status: t.status.as_str().to_string(),
            user_id: uid,
            endpoint: t.endpoint.clone(),
        }
    }
}


impl From<RowInstance> for TaskInstance {
    fn from(r: RowInstance) -> Self {
        TaskInstance {
            id: r.id,
            task_name: r.task_name,
            container_id: r.container_id,
            created_at: r.created_at,
            expires_at: r.expires_at,
            status: match r.status.as_str() {
                "Running" => InstanceStatus::Running,
                "Stopped" => InstanceStatus::Stopped,
                _ => InstanceStatus::Expired,
            },
            endpoint: r.endpoint,
            user_id: r.user_id,
        }
    }
}
