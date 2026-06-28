use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use serde::Serialize;
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::error;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

type DbPool = Pool<ConnectionManager<SqliteConnection>>;

diesel::table! {
    queries (id) {
        id -> Integer,
        ts -> BigInt,
        domain -> Text,
        blocked -> Integer,
    }
}

#[derive(Insertable)]
#[diesel(table_name = queries)]
struct NewQuery {
    ts: i64,
    domain: String,
    blocked: i32,
}

#[derive(Serialize)]
pub struct Summary {
    pub total: i64,
    pub blocked: i64,
    pub total_today: i64,
    pub blocked_today: i64,
}

#[derive(Serialize)]
pub struct DomainCount {
    pub domain: String,
    pub count: i64,
}

struct QueryEvent {
    ts: i64,
    domain: String,
    blocked: bool,
}

pub struct Analytics {
    pool: Arc<DbPool>,
    sender: UnboundedSender<QueryEvent>,
}

impl Analytics {
    pub fn new(db_path: &str) -> anyhow::Result<Arc<Self>> {
        let manager = ConnectionManager::<SqliteConnection>::new(db_path);
        let pool = Arc::new(Pool::builder().build(manager)?);

        pool.get()?
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(Self::background_worker(pool.clone(), receiver));

        Ok(Arc::new(Self { pool, sender }))
    }

    async fn background_worker(pool: Arc<DbPool>, mut rx: mpsc::UnboundedReceiver<QueryEvent>) {
        loop {
            let Some(first) = rx.recv().await else { break };

            let mut batch = vec![NewQuery {
                ts: first.ts,
                domain: first.domain,
                blocked: first.blocked as i32,
            }];

            while batch.len() < 200 {
                match rx.try_recv() {
                    Ok(ev) => batch.push(NewQuery {
                        ts: ev.ts,
                        domain: ev.domain,
                        blocked: ev.blocked as i32,
                    }),
                    Err(_) => break,
                }
            }

            let pool = pool.clone();
            tokio::task::spawn_blocking(move || {
                let mut conn = match pool.get() {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "analytics: failed to get DB connection");
                        return;
                    }
                };
                if let Err(e) = diesel::insert_into(queries::table)
                    .values(&batch)
                    .execute(&mut conn)
                {
                    error!(error = %e, "analytics: batch insert failed");
                }
            })
            .await
            .ok();
        }
    }

    pub fn record(&self, domain: String, blocked: bool) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let _ = self.sender.send(QueryEvent { ts, domain, blocked });
    }

    pub async fn summary(&self) -> anyhow::Result<Summary> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            let total: i64 = queries::table.count().get_result(&mut conn)?;
            let blocked: i64 = queries::table
                .filter(queries::blocked.eq(1i32))
                .count()
                .get_result(&mut conn)?;

            let today_start = today_midnight_ts();
            let total_today: i64 = queries::table
                .filter(queries::ts.ge(today_start))
                .count()
                .get_result(&mut conn)?;

            let blocked_today: i64 = queries::table
                .filter(queries::ts.ge(today_start))
                .filter(queries::blocked.eq(1i32))
                .count()
                .get_result(&mut conn)?;

            Ok(Summary { total, blocked, total_today, blocked_today })
        })
        .await?
    }

    pub async fn top_blocked(&self, limit: i64) -> anyhow::Result<Vec<DomainCount>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            use diesel::dsl::count;
            let mut conn = pool.get()?;

            let rows: Vec<(String, i64)> = queries::table
                .filter(queries::blocked.eq(1i32))
                .group_by(queries::domain)
                .select((queries::domain, count(queries::id)))
                .order_by(count(queries::id).desc())
                .limit(limit)
                .load(&mut conn)?;

            Ok(rows
                .into_iter()
                .map(|(domain, count)| DomainCount { domain, count })
                .collect())
        })
        .await?
    }

    pub async fn top_queried(&self, limit: i64) -> anyhow::Result<Vec<DomainCount>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            use diesel::dsl::count;
            let mut conn = pool.get()?;
            let rows: Vec<(String, i64)> = queries::table
                .group_by(queries::domain)
                .select((queries::domain, count(queries::id)))
                .order_by(count(queries::id).desc())
                .limit(limit)
                .load(&mut conn)?;
            Ok(rows
                .into_iter()
                .map(|(domain, count)| DomainCount { domain, count })
                .collect())
        })
        .await?
    }
}

fn today_midnight_ts() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    now - (now % 86400)
}
