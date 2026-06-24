use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::Mutex, time::sleep};
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

const URL: &str =
    "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/pro-onlydomains.txt";

pub struct Blocklist {
    pub update_freq: Duration,
    pub last_update: Instant,
    pub domains: HashSet<String>,
    pub user_added: HashSet<String>,
    pub user_removed: HashSet<String>,
}

impl Serialize for Blocklist {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut state = serializer.serialize_struct("Blocklist", 3)?;
        state.serialize_field("update_freq", &self.update_freq.as_secs())?;
        state.serialize_field("last_update", &self.last_update.elapsed().as_secs())?;
        state.serialize_field("domains", &self.domains)?;
        state.end()
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
        }
    }

    pub async fn spawn(this: Arc<Mutex<Self>>) -> anyhow::Result<()> {
        loop {
            if let Ok(resp) = reqwest::get(URL).await {
                let text = resp.text().await?;

                let mut guard = this.lock().await;
                guard.domains.clear();
                for line in text.lines() {
                    if line.contains('#') {
                        continue;
                    }
                    guard.domains.insert(line.to_string());
                }

                let user_added = guard.user_added.clone();
                guard.domains.extend(user_added);

                let user_removed = guard.user_removed.clone();
                for domain in user_removed {
                    guard.domains.remove(&domain);
                }

                guard.last_update = Instant::now();
                let update_freq = guard.update_freq;
                drop(guard);

                sleep(update_freq).await;
            } else {
                println!("Failed to fetch");
                sleep(Duration::from_secs(60)).await;
            }
        }
    }

    pub fn check(&self, domain: &str) -> bool {
        self.domains.contains(domain)
    }
}
