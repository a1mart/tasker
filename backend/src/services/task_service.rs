// src/services/task_service.rs
use std::sync::Arc;
use std::pin::Pin;
use std::time::SystemTime;

use futures::Stream;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;
use prost_types::Timestamp;

use crate::protogen::{
    task_service_server::TaskService,
    *,
};
use crate::types::timestamp::SerdeTimestamp;
use crate::storage::Storage;

pub struct TaskServiceImpl {
    storage: Arc<Storage>,
}

impl TaskServiceImpl {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    fn system_time_to_timestamp(time: SystemTime) -> SerdeTimestamp {
        let duration = time.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        let timestamp = Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos() as i32,
        };
        SerdeTimestamp(timestamp)
    }
}

#[tonic::async_trait]
impl TaskService for TaskServiceImpl {
    async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<CreateTaskResponse>, Status> {
        let req = request.into_inner();
        
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: req.title,
            description: req.description,
            status: TaskStatus::Todo as i32,
            priority: req.priority,
            tags: req.tags,
            assigned_to: req.assigned_to,
            created_at: Some(Self::system_time_to_timestamp(SystemTime::now())),
            updated_at: Some(Self::system_time_to_timestamp(SystemTime::now())),
            due_date: req.due_date,
            metrics: Some(TaskMetrics {
                estimated_hours: 0,
                actual_hours: 0,
                completion_percentage: 0.0,
            }),
            comments: vec![],
            attachments: vec![],
        };

        self.storage.create_task(task.clone()).await;

        let response = CreateTaskResponse {
            task: Some(task),
            success: true,
            message: "Task created successfully".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_task(
        &self,
        request: Request<GetTaskRequest>,
    ) -> Result<Response<GetTaskResponse>, Status> {
        let req = request.into_inner();
        
        if let Some(task) = self.storage.get_task(&req.id).await {
            let response = GetTaskResponse {
                task: Some(task),
                found: true,
            };
            Ok(Response::new(response))
        } else {
            let response = GetTaskResponse {
                task: None,
                found: false,
            };
            Ok(Response::new(response))
        }
    }

    async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<UpdateTaskResponse>, Status> {
        let req = request.into_inner();
    
        // Ensure task data is provided
        if let Some(mut patch) = req.task {
            // Ensure ID is set
            patch.id = req.id.clone();
            patch.updated_at = Some(Self::system_time_to_timestamp(SystemTime::now()));
    
            // Apply patch via storage, mapping errors to tonic::Status
            let updated = self
                .storage
                .patch_task(&req.id, patch.clone(), &req.update_mask)
                .await
                .map_err(|e| Status::not_found(format!("Failed to update task: {}", e)))?;
    
            // Return the updated task (clone patch, since storage returns `()`)
            let response = UpdateTaskResponse {
                task: Some(patch),
                success: true,
                message: "Task updated successfully".to_string(),
            };
    
            Ok(Response::new(response))
        } else {
            Err(Status::invalid_argument("Task data is required"))
        }
    }

    async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        let req = request.into_inner();
    
        // Await the Result<bool, E> and map errors to a tonic::Status
        let success = self
            .storage
            .delete_task(&req.id)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete task: {}", e)))?;
    
        let response = DeleteTaskResponse {
            success,
            message: if success {
                "Task deleted successfully".to_string()
            } else {
                "Task not found".to_string()
            },
        };
    
        Ok(Response::new(response))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let req = request.into_inner();
        
        let tasks = self.storage.list_tasks(req.page_size, &req.page_token).await;
        
        let response = ListTasksResponse {
            tasks: tasks.clone(),
            next_page_token: if tasks.len() >= req.page_size as usize {
                format!("page_{}", req.page_token.parse::<u32>().unwrap_or(0) + 1)
            } else {
                String::new()
            },
            total_count: self.storage.count_tasks().await,
        };
        
        Ok(Response::new(response))
    }

    async fn bulk_update_tasks(
        &self,
        request: Request<BulkUpdateTasksRequest>,
    ) -> Result<Response<BulkUpdateTasksResponse>, Status> {
        let req = request.into_inner();
        
        let mut updated_count = 0;
        let mut failed_ids = Vec::new();
        
        for task_id in req.task_ids {
            if let Some(mut task) = self.storage.get_task(&task_id).await {
                if req.status != TaskStatus::Unspecified as i32 {
                    task.status = req.status;
                }
                if !req.assigned_to.is_empty() {
                    task.assigned_to = req.assigned_to.clone();
                }
                task.tags.extend(req.tags_to_add.clone());
                task.tags.retain(|tag| !req.tags_to_remove.contains(tag));
                task.updated_at = Some(Self::system_time_to_timestamp(SystemTime::now()));
                
                self.storage.update_task(task).await;
                updated_count += 1;
            } else {
                failed_ids.push(task_id);
            }
        }
        
        let response = BulkUpdateTasksResponse {
            updated_count,
            failed_ids,
            message: format!("Updated {} tasks", updated_count),
        };
        
        Ok(Response::new(response))
    }

    type StreamTaskEventsStream = Pin<Box<dyn Stream<Item = Result<TaskEvent, Status>> + Send>>;

    async fn stream_task_events(
        &self,
        request: Request<StreamTaskEventsRequest>,
    ) -> Result<Response<Self::StreamTaskEventsStream>, Status> {
        let _req = request.into_inner();
        
        let (tx, rx) = mpsc::channel(10);
        
        // Simulate real-time events
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            let mut counter = 0;
            
            loop {
                interval.tick().await;
                counter += 1;
                
                let event = TaskEvent {
                    event_id: Uuid::new_v4().to_string(),
                    event_type: TaskEventType::Updated as i32,
                    task: Some(Task {
                        id: format!("task_{}", counter),
                        title: format!("Sample Task {}", counter),
                        description: "Auto-generated task event".to_string(),
                        status: TaskStatus::InProgress as i32,
                        priority: TaskPriority::Medium as i32,
                        tags: vec!["auto".to_string()],
                        assigned_to: "system".to_string(),
                        created_at: Some(TaskServiceImpl::system_time_to_timestamp(SystemTime::now())),
                        updated_at: Some(TaskServiceImpl::system_time_to_timestamp(SystemTime::now())),
                        due_date: None,
                        metrics: Some(TaskMetrics {
                            estimated_hours: 2,
                            actual_hours: 1,
                            completion_percentage: 50.0,
                        }),
                        comments: vec![],
                        attachments: vec![],
                    }),
                    user_id: "system".to_string(),
                    timestamp: Some(TaskServiceImpl::system_time_to_timestamp(SystemTime::now())),
                    metadata: std::collections::HashMap::new(),
                };
                
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::StreamTaskEventsStream))
    }

    type ImportTasksStream = Pin<Box<dyn Stream<Item = Result<CreateTaskResponse, Status>> + Send>>;

    async fn import_tasks(
        &self,
        request: Request<Streaming<CreateTaskRequest>>,
    ) -> Result<Response<Self::ImportTasksStream>, Status> {
        let mut stream = request.into_inner();
        let storage = self.storage.clone();
        
        let (tx, rx) = mpsc::channel(10);
        
        tokio::spawn(async move {
            while let Some(request) = stream.next().await {
                match request {
                    Ok(req) => {
                        let task = Task {
                            id: Uuid::new_v4().to_string(),
                            title: req.title,
                            description: req.description,
                            status: TaskStatus::Todo as i32,
                            priority: req.priority,
                            tags: req.tags,
                            assigned_to: req.assigned_to,
                            created_at: Some(TaskServiceImpl::system_time_to_timestamp(SystemTime::now())),
                            updated_at: Some(TaskServiceImpl::system_time_to_timestamp(SystemTime::now())),
                            due_date: req.due_date,
                            metrics: Some(TaskMetrics {
                                estimated_hours: 0,
                                actual_hours: 0,
                                completion_percentage: 0.0,
                            }),
                            comments: vec![],
                            attachments: vec![],
                        };

                        storage.create_task(task.clone()).await;

                        let response = CreateTaskResponse {
                            task: Some(task),
                            success: true,
                            message: "Task imported successfully".to_string(),
                        };

                        if tx.send(Ok(response)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let response = CreateTaskResponse {
                            task: None,
                            success: false,
                            message: format!("Import failed: {}", e),
                        };
                        if tx.send(Ok(response)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::ImportTasksStream))
    }

    type CollaborateOnTasksStream = Pin<Box<dyn Stream<Item = Result<TaskEvent, Status>> + Send>>;

    async fn collaborate_on_tasks(
        &self,
        request: Request<Streaming<TaskEvent>>,
    ) -> Result<Response<Self::CollaborateOnTasksStream>, Status> {
        let mut stream = request.into_inner();
        
        let (tx, rx) = mpsc::channel(10);
        
        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        // Echo the event back (in real implementation, broadcast to other clients)
                        if tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error in collaboration stream: {}", e);
                        break;
                    }
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::CollaborateOnTasksStream))
    }

    async fn upload_task_attachment(
        &self,
        request: Request<Streaming<UploadTaskAttachmentRequest>>,
    ) -> Result<Response<UploadTaskAttachmentResponse>, Status> {
        let mut stream = request.into_inner();
        let mut file_data = Vec::new();
        let mut filename = String::new();
        let mut task_id = String::new();
        let mut content_type = String::new();
        
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(request) => {
                    if !request.task_id.is_empty() && task_id.is_empty() {
                        task_id = request.task_id;
                    }
                    if !request.filename.is_empty() {
                        filename = request.filename;
                    }
                    if !request.content_type.is_empty() {
                        content_type = request.content_type;
                    }
                    file_data.extend(request.chunk);
                }
                Err(e) => {
                    return Err(Status::internal(format!("Upload failed: {}", e)));
                }
            }
        }
        
        // Store attachment (in real implementation, save to file system or cloud storage)
        let attachment_id = Uuid::new_v4().to_string();
        let file_size = file_data.len() as u64;
        
        // Update task with attachment info
        if let Some(mut task) = self.storage.get_task(&task_id).await {
            let _attachment = TaskAttachment {
                id: attachment_id.clone(),
                filename: filename.clone(),
                content_type,
                file_size,
                uploaded_at: Some(Self::system_time_to_timestamp(SystemTime::now())),
                uploaded_by: "user".to_string(), // In real implementation, get from auth
                url: format!("/attachments/{}", attachment_id),
            };
            
            // In real implementation, you'd have an attachments field in Task
            // For now, we'll just update the task
            task.updated_at = Some(Self::system_time_to_timestamp(SystemTime::now()));
            self.storage.update_task(task).await;
        }
        
        let response = UploadTaskAttachmentResponse {
            attachment_id,
            filename,
            file_size,
            success: true,
            message: "Attachment uploaded successfully".to_string(),
        };
        
        Ok(Response::new(response))
    }

    async fn search_tasks(
        &self,
        request: Request<SearchTasksRequest>,
    ) -> Result<Response<SearchTasksResponse>, Status> {
        let req = request.into_inner();
        
        // Fix: Use the correct method signature with 3 parameters
        let tasks = self.storage.search_tasks(&req.query, 50, "").await;
        let total_count = tasks.len() as u32;

        let response = SearchTasksResponse {
            tasks,
            total_count,
            search_time_ms: 50,
        };

        Ok(Response::new(response))
    }

    async fn get_task_analytics(
        &self,
        request: Request<GetTaskAnalyticsRequest>,
    ) -> Result<Response<GetTaskAnalyticsResponse>, Status> {
        let _req = request.into_inner();
        
        // Simulate analytics calculation
        let total_tasks = self.storage.count_tasks().await;
        let completed_tasks = self.storage.count_tasks_by_status(TaskStatus::Done).await;
        let in_progress_tasks = self.storage.count_tasks_by_status(TaskStatus::InProgress).await;
        let todo_tasks = self.storage.count_tasks_by_status(TaskStatus::Todo).await;
        
        let analytics = TaskAnalytics {
            total_tasks: total_tasks.try_into().unwrap_or(0),
            completed_tasks: completed_tasks.try_into().unwrap_or(0),
            in_progress_tasks: in_progress_tasks.try_into().unwrap_or(0),
            todo_tasks: todo_tasks.try_into().unwrap_or(0),
            completion_rate: if total_tasks > 0 {
                (completed_tasks as f32 / total_tasks as f32) * 100.0
            } else {
                0.0
            },
            average_completion_time_hours: 24.5, // Simulated
            overdue_tasks: self.storage.count_overdue_tasks().await.try_into().unwrap_or(0),
            tasks_by_priority: std::collections::HashMap::from([
                (TaskPriority::High as i32, 15),
                (TaskPriority::Medium as i32, 25),
                (TaskPriority::Low as i32, 10),
            ]),
            tasks_created_this_week: 12,
            tasks_completed_this_week: 8,
        };
        
        let response = GetTaskAnalyticsResponse {
            analytics: Some(analytics),
            generated_at: Some(Self::system_time_to_timestamp(SystemTime::now())),
        };
        
        Ok(Response::new(response))
    }

    async fn health(
        &self,
        _request: Request<()>,
    ) -> Result<Response<HealthResponse>, Status> {
        let response = HealthResponse {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: Some(Self::system_time_to_timestamp(SystemTime::now())),
        };
        
        Ok(Response::new(response))
    }
}