// src/storage/mod.rs
use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use tokio::sync::RwLock;
use tokio::fs;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

use crate::protogen::{User, Task, TaskStatus, TaskPriority};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StorageData {
    users: HashMap<String, User>,
    users_by_email: HashMap<String, String>,
    users_by_username: HashMap<String, String>,
    tasks: HashMap<String, Task>,
    user_tasks: HashMap<String, Vec<String>>,
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            users: HashMap::new(),
            users_by_email: HashMap::new(),
            users_by_username: HashMap::new(),
            tasks: HashMap::new(),
            user_tasks: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Storage {
    data: Arc<RwLock<StorageData>>,
    persistence_path: Option<String>,
    auto_save: bool,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(StorageData::default())),
            persistence_path: None,
            auto_save: false,
        }
    }

    pub fn with_persistence<P: AsRef<Path>>(path: P, auto_save: bool) -> Self {
        Self {
            data: Arc::new(RwLock::new(StorageData::default())),
            persistence_path: Some(path.as_ref().to_string_lossy().to_string()),
            auto_save,
        }
    }

    pub async fn load_from_disk(&self) -> Result<()> {
        if let Some(path) = &self.persistence_path {
            if Path::new(path).exists() {
                let content = fs::read_to_string(path).await
                    .context("Failed to read storage file")?;
                let storage_data: StorageData = serde_json::from_str(&content)
                    .context("Failed to deserialize storage data")?;
                *self.data.write().await = storage_data;
                println!("Loaded data from {}", path);
            }
        }
        Ok(())
    }

    pub async fn save_to_disk(&self) -> Result<()> {
        if let Some(path) = &self.persistence_path {
            let data = self.data.read().await;
            let json = serde_json::to_string_pretty(&*data)
                .context("Failed to serialize storage data")?;
            
            // Create parent directories if they don't exist
            if let Some(parent) = Path::new(path).parent() {
                fs::create_dir_all(parent).await
                    .context("Failed to create storage directory")?;
            }
            
            // Write to temporary file first, then rename (atomic operation)
            let temp_path = format!("{}.tmp", path);
            fs::write(&temp_path, json).await
                .context("Failed to write temporary storage file")?;
            fs::rename(&temp_path, path).await
                .context("Failed to rename temporary storage file")?;
            
            println!("Saved data to {}", path);
        }
        Ok(())
    }

    async fn auto_save_if_enabled(&self) {
        if self.auto_save {
            if let Err(e) = self.save_to_disk().await {
                eprintln!("Auto-save failed: {}", e);
            }
        }
    }

    // User methods
    pub async fn create_user(&self, user: User) -> Result<()> {
        let user_id = user.id.clone();
        let email = user.email.clone();
        let username = user.username.clone();
        
        {
            let mut data = self.data.write().await;
            data.users.insert(user_id.clone(), user);
            data.users_by_email.insert(email, user_id.clone());
            data.users_by_username.insert(username, user_id.clone());
            data.user_tasks.insert(user_id, Vec::new());
        }
        
        self.auto_save_if_enabled().await;
        Ok(())
    }

    pub async fn get_user(&self, user_id: &str) -> Option<User> {
        self.data.read().await.users.get(user_id).cloned()
    }

    pub async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let data = self.data.read().await;
        if let Some(user_id) = data.users_by_email.get(email) {
            data.users.get(user_id).cloned()
        } else {
            None
        }
    }

    pub async fn get_user_by_username(&self, username: &str) -> Option<User> {
        let data = self.data.read().await;
        if let Some(user_id) = data.users_by_username.get(username) {
            data.users.get(user_id).cloned()
        } else {
            None
        }
    }

    pub async fn update_user(&self, user: User) -> Result<()> {
        let user_id = user.id.clone();
        {
            let mut data = self.data.write().await;
            data.users.insert(user_id, user);
        }
        self.auto_save_if_enabled().await;
        Ok(())
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<bool> {
        let result = {
            let mut data = self.data.write().await;
            if let Some(user) = data.users.remove(user_id) {
                // Clean up related data
                data.users_by_email.remove(&user.email);
                data.users_by_username.remove(&user.username);
                data.user_tasks.remove(user_id);
                true
            } else {
                false
            }
        };
        
        if result {
            self.auto_save_if_enabled().await;
        }
        Ok(result)
    }

    pub async fn list_users(&self, page_size: i32, page_token: &str) -> Vec<User> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        
        data.users.values()
            .skip(start)
            .take(page_size as usize)
            .cloned()
            .collect()
    }

    pub async fn count_users(&self) -> i32 {
        self.data.read().await.users.len() as i32
    }

    pub async fn count_user_tasks(&self, user_id: &str) -> i32 {
        self.data.read().await.user_tasks
            .get(user_id)
            .map(|tasks| tasks.len() as i32)
            .unwrap_or(0)
    }

    // Task methods
    pub async fn create_task(&self, task: Task) -> Result<()> {
        let task_id = task.id.clone();
        let assignee_id = task.assigned_to.clone();
        
        {
            let mut data = self.data.write().await;
            data.tasks.insert(task_id.clone(), task);
            
            // Add to user's tasks
            if !assignee_id.is_empty() {
                data.user_tasks
                    .entry(assignee_id)
                    .or_default()
                    .push(task_id);
            }
        }
        
        self.auto_save_if_enabled().await;
        Ok(())
    }

    pub async fn get_task(&self, task_id: &str) -> Option<Task> {
        self.data.read().await.tasks.get(task_id).cloned()
    }

    /// Partially update a task based on the given field mask
    pub async fn patch_task(&self, task_id: &str, patch: Task, mask: &[String]) -> anyhow::Result<()> {
        {
            let mut data = self.data.write().await;

            let existing = data.tasks.get_mut(task_id)
                .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

            for field in mask {
                match field.as_str() {
                    "title"        => existing.title = patch.title.clone(),
                    "description"  => existing.description = patch.description.clone(),
                    "status"       => existing.status = patch.status,
                    "priority"     => existing.priority = patch.priority,
                    "tags"         => existing.tags = patch.tags.clone(),
                    "assignedTo"  => existing.assigned_to = patch.assigned_to.clone(),
                    "due_date"     => existing.due_date = patch.due_date.clone(),
                    "metrics"      => existing.metrics = patch.metrics.clone(),
                    "comments"     => existing.comments = patch.comments.clone(),
                    "attachments"  => existing.attachments = patch.attachments.clone(),
                    _ => {
                        // ignore or return Err(anyhow!("unknown field: {field}"))
                    }
                }
            }
        } 

        self.auto_save_if_enabled().await;
        Ok(())
    }


    pub async fn update_task(&self, task: Task) -> Result<()> {
        let task_id = task.id.clone();
        {
            let mut data = self.data.write().await;
            data.tasks.insert(task_id, task);
        }
        self.auto_save_if_enabled().await;
        Ok(())
    }

    pub async fn delete_task(&self, task_id: &str) -> Result<bool> {
        let result = {
            let mut data = self.data.write().await;
            if let Some(task) = data.tasks.remove(task_id) {
                // Remove from user's tasks
                if !task.assigned_to.is_empty() {
                    if let Some(user_tasks) = data.user_tasks.get_mut(&task.assigned_to) {
                        user_tasks.retain(|id| id != task_id);
                    }
                }
                true
            } else {
                false
            }
        };
        
        if result {
            self.auto_save_if_enabled().await;
        }
        Ok(result)
    }

    pub async fn list_tasks(&self, page_size: i32, page_token: &str) -> Vec<Task> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        
        data.tasks.values()
            .skip(start)
            .take(page_size as usize)
            .cloned()
            .collect()
    }

    pub async fn get_tasks_by_user(&self, user_id: &str, page_size: i32, page_token: &str) -> Vec<Task> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        
        if let Some(task_ids) = data.user_tasks.get(user_id) {
            task_ids.iter()
                .skip(start)
                .take(page_size as usize)
                .filter_map(|task_id| data.tasks.get(task_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn get_tasks_by_status(&self, status: TaskStatus, page_size: i32, page_token: &str) -> Vec<Task> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        let status_value = status as i32;
        
        data.tasks.values()
            .filter(|task| task.status == status_value)
            .skip(start)
            .take(page_size as usize)
            .cloned()
            .collect()
    }

    pub async fn get_tasks_by_priority(&self, priority: TaskPriority, page_size: i32, page_token: &str) -> Vec<Task> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        let priority_value = priority as i32;
        
        data.tasks.values()
            .filter(|task| task.priority == priority_value)
            .skip(start)
            .take(page_size as usize)
            .cloned()
            .collect()
    }

    pub async fn count_tasks(&self) -> i32 {
        self.data.read().await.tasks.len() as i32
    }

    pub async fn count_tasks_by_status(&self, status: TaskStatus) -> i32 {
        let data = self.data.read().await;
        let status_value = status as i32;
        data.tasks.values()
            .filter(|task| task.status == status_value)
            .count() as i32
    }

    pub async fn count_tasks_by_priority(&self, priority: TaskPriority) -> i32 {
        let data = self.data.read().await;
        let priority_value = priority as i32;
        data.tasks.values()
            .filter(|task| task.priority == priority_value)
            .count() as i32
    }

    pub async fn count_overdue_tasks(&self) -> i32 {
        let data = self.data.read().await;
        let now = std::time::SystemTime::now();
        let now_timestamp = now.duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        
        data.tasks.values()
            .filter(|task| {
                if let Some(due_date) = &task.due_date {
                    due_date.seconds < now_timestamp && task.status != 4 // Not completed
                } else {
                    false
                }
            })
            .count() as i32
    }

    // Search methods
    pub async fn search_tasks(&self, query: &str, page_size: i32, page_token: &str) -> Vec<Task> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        let query_lower = query.to_lowercase();
        
        data.tasks.values()
            .filter(|task| {
                task.title.to_lowercase().contains(&query_lower) ||
                task.description.to_lowercase().contains(&query_lower)
            })
            .skip(start)
            .take(page_size as usize)
            .cloned()
            .collect()
    }

    pub async fn search_users(&self, query: &str, page_size: i32, page_token: &str) -> Vec<User> {
        let data = self.data.read().await;
        let page_num: usize = page_token.strip_prefix("page_")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        let start = page_num * page_size as usize;
        let query_lower = query.to_lowercase();
        
        data.users.values()
            .filter(|user| {
                user.username.to_lowercase().contains(&query_lower) ||
                user.email.to_lowercase().contains(&query_lower) ||
                user.full_name.to_lowercase().contains(&query_lower)
            })
            .skip(start)
            .take(page_size as usize)
            .cloned()
            .collect()
    }

    // Batch operations for better performance
    pub async fn batch_create_tasks(&self, tasks: Vec<Task>) -> Result<()> {
        {
            let mut data = self.data.write().await;
            for task in tasks {
                let task_id = task.id.clone();
                let assignee_id = task.assigned_to.clone();
                
                data.tasks.insert(task_id.clone(), task);
                
                if !assignee_id.is_empty() {
                    data.user_tasks
                        .entry(assignee_id)
                        .or_default()
                        .push(task_id);
                }
            }
        }
        self.auto_save_if_enabled().await;
        Ok(())
    }

    pub async fn batch_create_users(&self, users: Vec<User>) -> Result<()> {
        {
            let mut data = self.data.write().await;
            for user in users {
                let user_id = user.id.clone();
                let email = user.email.clone();
                let username = user.username.clone();
                
                data.users.insert(user_id.clone(), user);
                data.users_by_email.insert(email, user_id.clone());
                data.users_by_username.insert(username, user_id.clone());
                data.user_tasks.insert(user_id, Vec::new());
            }
        }
        self.auto_save_if_enabled().await;
        Ok(())
    }

    // Manual save/load operations
    pub async fn force_save(&self) -> Result<()> {
        self.save_to_disk().await
    }

    pub async fn reload(&self) -> Result<()> {
        self.load_from_disk().await
    }

    // Backup functionality
    pub async fn backup_to<P: AsRef<Path>>(&self, backup_path: P) -> Result<()> {
        let data = self.data.read().await;
        let json = serde_json::to_string_pretty(&*data)
            .context("Failed to serialize storage data for backup")?;
        
        let path = backup_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create backup directory")?;
        }
        
        fs::write(path, json).await
            .context("Failed to write backup file")?;
        
        println!("Backup saved to {}", path.display());
        Ok(())
    }

    pub async fn restore_from<P: AsRef<Path>>(&self, backup_path: P) -> Result<()> {
        let content = fs::read_to_string(backup_path).await
            .context("Failed to read backup file")?;
        let storage_data: StorageData = serde_json::from_str(&content)
            .context("Failed to deserialize backup data")?;
        
        *self.data.write().await = storage_data;
        self.auto_save_if_enabled().await;
        
        println!("Data restored from backup");
        Ok(())
    }
}

// Example usage in main.rs or lib.rs:
/*
#[tokio::main]
async fn main() -> Result<()> {
    // Create storage with persistence
    let storage = Storage::with_persistence("data/storage.json", true);
    
    // Load existing data on startup
    storage.load_from_disk().await?;
    
    // Your application logic here...
    
    // Manual save if needed (auto-save is enabled)
    storage.force_save().await?;
    
    Ok(())
}
*/