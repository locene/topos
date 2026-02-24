use r2d2::{Pool, PooledConnection};
use redis::{Client, Commands, ErrorKind, RedisResult};

use crate::config;

const RESUME_POINT_KEY: &str = "topos:resume_point_id";

type RedisPool = Pool<redis::Client>;
type RedisPooledConnection = PooledConnection<Client>;

#[derive(Debug)]
pub struct ResumePoint {
    pool: RedisPool,
    id: Option<u32>,
}

impl ResumePoint {
    pub fn new() -> RedisResult<Self> {
        let config = config::get_app_config();
        let manager = redis::Client::open(config.redis_url.clone())?;

        let pool = r2d2::Pool::builder()
            .max_size(10)
            .build(manager)
            .map_err(|e| {
                redis::RedisError::from((
                    redis::ErrorKind::Io,
                    "R2D2 Pool Build Failed",
                    format!("Failed to build r2d2 pool: {}", e),
                ))
            })?;

        println!("Successfully initialized ResumePoint with R2D2 Redis Pool.");

        Ok(ResumePoint { pool, id: None })
    }

    fn get_conn(&self) -> RedisResult<RedisPooledConnection> {
        self.pool.get().map_err(|e| {
            redis::RedisError::from((
                ErrorKind::Client,
                "Connection Pool Error",
                format!("Failed to get connection from pool for set_id: {}", e),
            ))
        })
    }

    pub fn get_id(&self) -> Option<u32> {
        let mut conn = self.get_conn().ok()?;

        let result_option_string: RedisResult<Option<String>> = conn.get(RESUME_POINT_KEY);

        match result_option_string {
            Ok(Some(s)) => match s.parse::<u32>() {
                Ok(id) => Some(id),
                Err(_) => None,
            },
            Ok(None) => None,
            Err(_) => None,
        }
    }

    pub fn set_id(&mut self, topic_num: u32) -> RedisResult<()> {
        let mut conn = self.get_conn()?;

        let _: () = conn.set(RESUME_POINT_KEY, topic_num)?;

        self.id = Some(topic_num);

        println!(
            "Updated resume point to {} and saved to Redis key '{}'.",
            topic_num, RESUME_POINT_KEY
        );

        Ok(())
    }

    pub fn delete(&mut self) -> RedisResult<()> {
        let mut conn = self.get_conn()?;

        let deleted_count: u32 = conn.del(RESUME_POINT_KEY)?;

        if deleted_count > 0 {
            println!(
                "Successfully deleted resume point key: '{}'",
                RESUME_POINT_KEY
            );

            self.id = None;
        } else {
            println!(
                "Resume point key '{}' not found. Nothing to delete.",
                RESUME_POINT_KEY
            );
        }

        Ok(())
    }
}
