use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::Mutex, time::sleep};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;

const DEFAULT_URL: &str =
    "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/pro-onlydomains.txt";

pub struct Blocklist {
    pub update_freq: Duration,
    pub last_update: Instant,
    pub domains: HashSet<String>,
    pub user_added: HashSet<String>,
    pub user_removed: HashSet<String>,
    pub url: String,
}

impl Serialize for Blocklist {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut state = serializer.serialize_struct("Blocklist", 3)?;
        state.serialize_field("update_freq", &self.update_freq.as_secs())?;
        state.serialize_field("last_update", &self.last_update.elapsed().as_secs())?;
        state.serialize_field("user_added", &self.user_added)?;
        state.serialize_field("user_removed", &self.user_removed)?;
        state.serialize_field("url", &self.url)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Blocklist {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field { UpdateFreq, LastUpdate, UserAdded, UserRemoved, Url }

        struct BlocklistVisitor;

        impl<'de> Visitor<'de> for BlocklistVisitor {
            type Value = Blocklist;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("struct Blocklist")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut update_freq: Option<u64> = None;
                let mut last_update_secs: Option<u64> = None;
                let mut user_added: Option<HashSet<String>> = None;
                let mut user_removed: Option<HashSet<String>> = None;
                let mut url: Option<String> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::UpdateFreq => update_freq = Some(map.next_value()?),
                        Field::LastUpdate => last_update_secs = Some(map.next_value()?),
                        Field::UserAdded => user_added = Some(map.next_value()?),
                        Field::UserRemoved => user_removed = Some(map.next_value()?),
                        Field::Url => url = Some(map.next_value()?),
                    }
                }

                let update_freq = update_freq.ok_or_else(|| de::Error::missing_field("update_freq"))?;
                let last_update_secs = last_update_secs.ok_or_else(|| de::Error::missing_field("last_update"))?;

                let last_update = Instant::now()
                    .checked_sub(Duration::from_secs(last_update_secs))
                    .unwrap_or_else(Instant::now);

                Ok(Blocklist {
                    update_freq: Duration::from_secs(update_freq),
                    last_update,
                    domains: HashSet::new(),
                    user_added: user_added.unwrap_or_default(),
                    user_removed: user_removed.unwrap_or_default(),
                    url: url.unwrap_or_else(|| DEFAULT_URL.to_string()),
                })
            }
        }

        const FIELDS: &[&str] = &["update_freq", "last_update", "user_added", "user_removed", "url"];
        deserializer.deserialize_struct("Blocklist", FIELDS, BlocklistVisitor)
    }
}

impl Blocklist {
    pub fn new() -> Self {
        Blocklist {
            update_freq: Duration::from_secs(24 * 60 * 60),
            last_update: Instant::now(),
            domains: HashSet::new(),
            user_added: HashSet::new(),
            user_removed: HashSet::new(),
            url: DEFAULT_URL.to_string(),
        }
    }

    pub async fn spawn(this: Arc<Mutex<Self>>) -> anyhow::Result<()> {
        let mut guard = this.lock().await;
        guard.load_config().await?;
        drop(guard);

        loop {
            println!("Updating blocklist");
            let mut guard = this.lock().await;
            match guard.update().await {
                Ok(_) => {
                    println!("Blocklist updated successfully");
                }
                Err(e) => {
                    println!("Failed to update blocklist: {}", e);
                    sleep(Duration::from_secs(60)).await;
                    continue;
                }
            }

            guard.last_update = Instant::now();
            let update_freq = guard.update_freq;
            drop(guard);

            sleep(update_freq).await;
        }
    }

    pub async fn update(&mut self) -> anyhow::Result<()> {
        match reqwest::get(self.url.clone()).await {
            Ok(resp) => {
                let text = resp.text().await?;

                self.domains.clear();
                for line in text.lines() {
                    if line.contains('#') {
                        continue;
                    }
                    self.domains.insert(line.to_string());
                }

                let user_added = self.user_added.clone();
                self.domains.extend(user_added);

                let user_removed = self.user_removed.clone();
                for domain in user_removed {
                    self.domains.remove(&domain);
                }

                self.last_update = Instant::now();
                Ok(())
            }

            Err(e) => {
                println!("Failed to fetch blocklist: {}", e);
                anyhow::bail!("Failed to fetch blocklist: {}", e);
            }
        }
    }

    pub fn check(&self, domain: &str) -> bool {
        self.domains.contains(domain)
    }

    pub async fn save_config(&self) -> anyhow::Result<()> {
        let config = serde_json::to_string_pretty(&self)?;
        tokio::fs::write("config.json", config).await?;
        Ok(())
    }

    pub async fn load_config(&mut self) -> anyhow::Result<()> {
        match tokio::fs::read_to_string("config.json").await {
            Ok(config) => {
                let loaded: Blocklist = serde_json::from_str(&config)?;
                self.update_freq = loaded.update_freq;
                self.last_update = loaded.last_update;
                self.user_added = loaded.user_added;
                self.user_removed = loaded.user_removed;
                self.url = loaded.url;
            }
            Err(e) => {
                println!("Failed to read config file: {}", e);
            }
        }
        Ok(())
    }
}
