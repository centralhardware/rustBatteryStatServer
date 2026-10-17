use battery_model::BatteryHealth;
use clickhouse::Client;

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
        let mut insert = self.client.insert("battery_health_buffer")?;
        insert.write(&health).await?;
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
