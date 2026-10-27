use battery_model::BatteryHealth;
use clickhouse::Client;

use crate::row::BatteryRow;

/// What the routes need from storage, so they can be tested against a fake.
pub trait Store: Send + Sync + 'static {
    fn ping(&self) -> impl Future<Output = Result<(), String>> + Send;
    fn save(&self, health: BatteryHealth) -> impl Future<Output = Result<(), String>> + Send;
}

impl Store for BatteryRepository {
    async fn ping(&self) -> Result<(), String> {
        BatteryRepository::ping(self).await.map_err(|e| e.to_string())
    }

    async fn save(&self, health: BatteryHealth) -> Result<(), String> {
        BatteryRepository::save(self, health).await.map_err(|e| e.to_string())
    }
}

pub struct BatteryRepository {
    client: Client,
}

impl BatteryRepository {
    pub fn new(url: &str, database: &str, user: &str, password: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_database(database)
            .with_user(user)
            .with_password(password);
        Self { client }
    }

    /// Round trip to ClickHouse: the server is useless without it, so this is
    /// what the /ping health check answers on.
    pub async fn ping(&self) -> Result<(), clickhouse::error::Error> {
        self.client.query("SELECT 1").execute().await
    }

    pub async fn save(&self, mut health: BatteryHealth) -> Result<(), clickhouse::error::Error> {
        if health.date_time == 0 {
            health.date_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as u32);
        }
        let mut insert = self.client.insert::<BatteryRow>("battery_health_buffer").await?;
        insert.write(&BatteryRow::from(health.clone())).await?;
        insert.end().await?;

        tracing::info!(
            "Battery health saved: device={}, cycles={}, health={}%, charge={}%, temp={:?}°C, charging={}",
            health.device_id,
            health.cycle_count,
            health.health_percent,
            health.current_charge,
            health.temperature,
            health.is_charging,
        );

        Ok(())
    }
}
