use r2d2::{Pool, PooledConnection};
use redis::{Client, Commands, ErrorKind, RedisResult};

use crate::config;

const SKIP_TOPICS_KEY: &str = "topos:skip_topics";

type RedisPool = Pool<redis::Client>;
type RedisPooledConnection = PooledConnection<Client>;

#[derive(Debug)]
pub struct SkipTopics {
    pool: RedisPool,
}

impl SkipTopics {
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

        println!("Successfully initialized SkipTopics with R2D2 Redis Pool.");

        Ok(SkipTopics { pool })
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

    pub fn contains(&self, topic_num: u32) -> RedisResult<bool> {
        let mut conn = self.get_conn()?;

        conn.sismember(SKIP_TOPICS_KEY, topic_num)?
    }

    pub fn add(&self, topic_num: u32) -> RedisResult<()> {
        let mut conn = self.get_conn()?;

        let added_count: u32 = conn.sadd(SKIP_TOPICS_KEY, topic_num)?;

        if added_count > 0 {
            println!("Added topic {} to Redis skip list.", topic_num);
        }

        Ok(())
    }

    pub fn delete(&self) -> RedisResult<()> {
        let mut conn = self.get_conn()?;

        let deleted_count: u32 = conn.del(SKIP_TOPICS_KEY)?;

        if deleted_count > 0 {
            println!(
                "Successfully deleted Redis skip list key: '{}'",
                SKIP_TOPICS_KEY
            );
        } else {
            println!(
                "Redis skip list key '{}' not found. Nothing to delete.",
                SKIP_TOPICS_KEY
            );
        }

        Ok(())
    }
}
