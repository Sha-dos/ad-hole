use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::{sync::Mutex, time::sleep};

const URL: &str =
    "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/wildcard/pro-onlydomains.txt";

pub struct Blocklist {
    update_freq: Duration,
    last_update: Instant,
    domains: HashSet<String>,
}

impl Blocklist {
    pub fn new() -> Self {
        Blocklist {
            update_freq: Duration::from_secs(24 * 60 * 60),
            last_update: Instant::now(),
            domains: HashSet::new(),
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
