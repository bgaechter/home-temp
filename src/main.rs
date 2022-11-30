use base64;
use chrono::Utc;
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use tokio_postgres::NoTls;

#[derive(Serialize, Deserialize, Debug)]
struct Token {
    access_token: String,
    token_type: String,
    expires_in: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DevicesResponse {
    pub result: Vec<Device>,
    pub t: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Device {
    pub active_time: i64,
    pub create_time: i64,
    pub id: String,
    pub name: String,
    pub online: bool,
    pub status: Vec<Status>,
    pub sub: bool,
    pub time_zone: String,
    pub update_time: i64,
    pub device_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub code: String,
    pub value: Value,
}

#[derive(Debug)]
struct Home {
    devices: Vec<Device>,
    token: Token,
    api_key: String,
    api_secret: String,
    time_since_update: Instant,
    time_since_token_renewal: Instant,
    postgres_password: String,
    postgres_user: String,
    postgres_host: String,
    postgres_dbname: String,
    reqwest_client: reqwest::Client,
}

impl Home {
    fn new() -> Self {
        let api_key = env::var("DANFOSS_API_KEY").expect("No Danfoss API key provided");

        let api_secret = env::var("DANFOSS_API_SECRET").expect("No Danfoss API secret provided.");
        let postgres_password =
            env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
        let postgres_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
        let postgres_host =
            env::var("POSTGRES_HOST").unwrap_or_else(|_| "home-temp-database-1".to_string());
        let postgres_dbname =
            env::var("POSTGRES_DBNAME").unwrap_or_else(|_| "home-temp".to_string());

        Self {
            devices: vec![],
            token: Token {
                access_token: String::new(),
                token_type: String::new(),
                expires_in: "0".to_string(),
            },
            api_key,
            api_secret,
            time_since_update: Instant::now(),
            time_since_token_renewal: Instant::now(),
            postgres_user,
            postgres_password,
            postgres_dbname,
            postgres_host,
            reqwest_client: reqwest::Client::new(),
        }
    }

    async fn write_to_pg(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (client, connection) = tokio_postgres::connect(
            format!(
                "host={} user={} password={} dbname={}",
                self.postgres_host,
                self.postgres_user,
                self.postgres_password,
                self.postgres_dbname
            )
            .as_str(),
            NoTls,
        )
        .await?;

        // The connection object performs the actual communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("connection error: {}", e);
            }
        });
        let update_time: SystemTime = SystemTime::now();
        for device in &self.devices {
            client.execute(
            "INSERT INTO device (active_time, create_time, id, name, online, sub, time_zone, update_time, device_type) VALUES ($1, $2,$3,$4,$5,$6,$7,$8, $9) ON CONFLICT DO NOTHING", // Might want to change the ON CONFLICT to DO UPDATE
            &[&device.active_time, &device.create_time, &device.id, &device.name, &device.online, &device.sub, &device.time_zone, &device.update_time, &device.device_type],
        ).await?;
            for status in &device.status {
                let id: String = format!("{}_{}", device.id, Utc::now());
                let value: &String = &status.value.to_string();
                client.execute(
                    "INSERT INTO device_status(code, value, device_id, id, update_time) VALUES ($1, $2, $3, $4, $5)",
                    &[&status.code, &value, &device.id, &id, &update_time],
                ).await?;
            }
        }
        Ok(())
    }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            sleep(Duration::new(30, 0));
            if self.time_since_token_renewal.elapsed().as_secs()
                >= self.token.expires_in.parse::<u64>()?
            {
                self.get_token()
                    .await
                    .unwrap_or_else(|e| error!("Could not fetch token. {:?}", e));
                self.time_since_token_renewal = Instant::now();
            }
            self.get_devices()
                .await
                .unwrap_or_else(|e| error!("Could not get devices. {:?}", e));
            self.time_since_update = Instant::now();
            self.print_room_temperatures();
            self.write_to_pg()
                .await
                .unwrap_or_else(|e| error!("Could not write to database. {:?}", e));
        }
    }

    fn print_room_temperatures(&self) {
        for device in &self.devices {
            for status in &device.status {
                if status.code == "va_temperature" || status.code == "temp_current" {
                    debug!("{}: {}", device.name, status.value);
                }
            }
        }
    }

    async fn get_token(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let basic_auth: String = base64::encode(format!("{}:{}", self.api_key, self.api_secret));
        let authorization_header: String = format!("Basic {}", basic_auth);

        let params = [("grant_type", "client_credentials")];
        let res = self
            .reqwest_client
            .post("https://api.danfoss.com/oauth2/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("accept", "application/json")
            .header("authorization", authorization_header)
            .form(&params)
            .send()
            .await?;

        self.token = serde_json::from_str(res.text().await?.as_str())?;
        Ok(())
    }
    async fn get_devices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let res = self
            .reqwest_client
            .get("https://api.danfoss.com/ally/devices")
            .header("accept", "application/json")
            .header(
                "authorization",
                format!("Bearer {}", self.token.access_token),
            )
            .send()
            .await?;
        let devices: DevicesResponse = serde_json::from_str(res.text().await?.as_str())?;
        self.devices = devices.result;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info! {"Starting up"};
    let mut home: Home = Home::new();
    home.run().await?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
