use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

// These types should align with sync_server/src/models.rs
// For a larger project, these would be in a shared crate.
// Ensure these structs are also defined or compatible with your db.rs structs
// as they will be populated from your local database.

// Re-iterating the dependencies from db.rs needed for ChangesPayload
// You would typically have `use super::db::{ExerciseDefinition, Workout, AliasEntryForSync, BodyweightEntryForSync};`
// For this example, I'm showing the full struct definitions that ChangesPayload expects.
// If these are ALREADY in your db.rs and exported, just use `super::db::...`

// Example:
// pub use super::db::{ExerciseDefinition, Workout, AliasEntryForSync, BodyweightEntryForSync};
// If not, define them (or ensure db.rs has compatible ones):

/*
// --- Ensure these (or compatible versions) are in db.rs and exported ---
// Placeholder if not directly using db.rs structs, adjust as needed.
// It's CRITICAL that these match what your local DB queries will produce
// AND what the server expects (which are also based on task_athlete_lib::db types).

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExerciseDefinition {
    pub id: i64,
    pub name: String,
    pub type_: super::db::ExerciseType, // Assuming ExerciseType is in db.rs
    pub muscles: Option<String>,
    pub log_weight: bool,
    pub log_reps: bool,
    pub log_duration: bool,
    pub log_distance: bool,
    pub deleted: bool,
    pub last_edited: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workout {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub exercise_name: String,
    pub sets: Option<i64>,
    pub reps: Option<i64>,
    pub weight: Option<f64>,
    pub duration_minutes: Option<i64>,
    pub bodyweight: Option<f64>,
    pub distance: Option<f64>,
    pub notes: Option<String>,
    pub exercise_type: Option<super::db::ExerciseType>,
    pub deleted: bool,
    pub last_edited: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasEntryForSync {
    pub alias_name: String,
    pub exercise_name: String,
    pub deleted: bool,
    pub last_edited: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BodyweightEntryForSync {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub weight: f64,
    pub deleted: bool,
    pub last_edited: DateTime<Utc>,
}
// --- End of db.rs struct dependencies ---
*/

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConfigChange {
    pub content: String,            // TOML content as a string
    pub last_edited: DateTime<Utc>, // When this config version was saved
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ChangesPayload {
    pub config: Option<ConfigChange>,
    // These will be populated from your local DB, ensure their definitions match
    pub exercises: Vec<super::db::ExerciseDefinition>, // Use the struct from your db module
    pub workouts: Vec<super::db::Workout>,             // Use the struct from your db module
    pub aliases: Vec<super::db::AliasEntryForSync>,    // Use the struct from your db module
    pub bodyweights: Vec<super::db::BodyweightEntryForSync>, // Use the struct from your db module
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncRequestPayload {
    pub client_last_sync_ts: Option<DateTime<Utc>>,
    pub changes: ChangesPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SyncServerResponse {
    pub server_current_ts: DateTime<Utc>,
    pub data_to_client: ChangesPayload,
}

pub struct SyncClient {
    http_client: Client,
    server_url: String,
}

impl SyncClient {
    pub fn new(server_url: String) -> Self {
        Self {
            http_client: Client::new(),
            server_url,
        }
    }

    /// Pushes local changes to the server and pulls remote changes.
    ///
    /// # Arguments
    ///
    /// * `client_last_sync_ts` - The timestamp of the last successful synchronization.
    ///                           `None` if this is the first sync.
    /// * `local_changes` - A payload containing all local data items that have been
    ///                     created, modified, or deleted since `client_last_sync_ts`.
    ///                     If `client_last_sync_ts` is `None`, this should contain all
    ///                     local data.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `SyncServerResponse`. The `data_to_client` field in this
    /// response contains items from the server that are newer than `client_last_sync_ts`
    /// (or all items if `client_last_sync_ts` was `None`) and need to be applied locally.
    /// The `server_current_ts` should be stored locally and used as `client_last_sync_ts`
    /// for the next sync operation.
    ///
    /// # Errors
    ///
    /// Returns `anyhow::Error` if the network request fails, the server returns an error status,
    /// or deserialization of the server response fails.
    pub async fn push_and_pull_changes(
        &self,
        client_last_sync_ts: Option<DateTime<Utc>>,
        local_changes: ChangesPayload,
    ) -> Result<SyncServerResponse> {
        let sync_url = format!("{}/sync", self.server_url);
        info!(
            "Sending POST to {} with client_last_sync_ts: {:?}, {} exercises, {} workouts, {} aliases, {} bodyweights",
            sync_url,
            client_last_sync_ts,
            local_changes.exercises.len(),
            local_changes.workouts.len(),
            local_changes.aliases.len(),
            local_changes.bodyweights.len()
        );
        if local_changes.config.is_some() {
            info!("Config change included in push.");
        }

        let request_payload = SyncRequestPayload {
            client_last_sync_ts,
            changes: local_changes,
        };

        debug!("Pushing payload: {:?}", request_payload);

        let response = self
            .http_client
            .post(&sync_url)
            .json(&request_payload)
            .send()
            .await
            .context("Failed to send sync POST request to server")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error body".to_string());
            error!(
                "Sync POST request failed with status: {}. Body: {}",
                status, error_body
            );
            bail!("Server returned error: {} - {}", status, error_body);
        }

        let server_response: SyncServerResponse = response
            .json()
            .await
            .context("Failed to deserialize server response from POST /sync")?;

        info!(
            "Received server response. Server current_ts: {}. {} exercises, {} workouts, {} aliases, {} bodyweights to apply from server.",
            server_response.server_current_ts,
            server_response.data_to_client.exercises.len(),
            server_response.data_to_client.workouts.len(),
            server_response.data_to_client.aliases.len(),
            server_response.data_to_client.bodyweights.len()
        );
        if server_response.data_to_client.config.is_some() {
            info!("Server sent config update.");
        }

        Ok(server_response)
    }
}
