// src/services/user_service.rs
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tonic::{Request, Response, Status};
use uuid::Uuid;
use prost_types::Timestamp;

use crate::protogen::{
    user_service_server::UserService,
    *,
};
use crate::storage::Storage;
use crate::types::timestamp::SerdeTimestamp; // Add this import

pub struct UserServiceImpl {
    storage: Arc<Storage>,
}

impl UserServiceImpl {
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
impl UserService for UserServiceImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        let req = request.into_inner();
        
        // Check if user already exists
        if self.storage.get_user_by_email(&req.email).await.is_some() {
            return Err(Status::already_exists("User with this email already exists"));
        }
        
        let now = SystemTime::now();
        let user = User {
            id: Uuid::new_v4().to_string(),
            username: req.username,
            email: req.email,
            full_name: req.full_name,
            role: req.role,
            is_active: true,
            permissions: vec![],
            status: UserStatus::Active as i32,
            created_at: Some(Self::system_time_to_timestamp(now)),
            updated_at: Some(Self::system_time_to_timestamp(now)),
            last_login: None,
            preferences: Some(UserPreferences {
                theme: "light".to_string(),
                language: "en".to_string(),
                timezone: "UTC".to_string(),
                notifications_enabled: true,
                email_notifications: true,
            }),
            profile: Some(UserProfile {
                avatar_url: String::new(),
                bio: String::new(),
                department: String::new(),
                phone: String::new(),
                location: String::new(),
            }),
        };

        self.storage.create_user(user.clone()).await;

        let response = CreateUserResponse {
            user: Some(user),
            success: true,
            message: "User created successfully".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let req = request.into_inner();
        
        if let Some(user) = self.storage.get_user(&req.id).await {
            let response = GetUserResponse {
                user: Some(user),
                found: true,
            };
            Ok(Response::new(response))
        } else {
            let response = GetUserResponse {
                user: None,
                found: false,
            };
            Ok(Response::new(response))
        }
    }

    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResponse>, Status> {
        let req = request.into_inner();
        
        if let Some(mut user) = req.user {
            user.id = req.id.clone();
            user.updated_at = Some(Self::system_time_to_timestamp(SystemTime::now()));
            
            self.storage.update_user(user.clone()).await;
            
            let response = UpdateUserResponse {
                user: Some(user),
                success: true,
                message: "User updated successfully".to_string(),
            };
            Ok(Response::new(response))
        } else {
            Err(Status::invalid_argument("User data is required"))
        }
    }

    async fn delete_user(
        &self,
        request: Request<DeleteUserRequest>,
    ) -> Result<Response<DeleteUserResponse>, Status> {
        let req = request.into_inner();
    
        let success = self
            .storage
            .delete_user(&req.id)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete user: {}", e)))?;
    
        let response = DeleteUserResponse {
            success,
            message: if success {
                "User deleted successfully".to_string()
            } else {
                "User not found".to_string()
            },
        };
    
        Ok(Response::new(response))
    }

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let req = request.into_inner();
        
        let users = self.storage.list_users(req.page_size, &req.page_token).await;
        
        let response = ListUsersResponse {
            users: users.clone(),
            next_page_token: if users.len() >= req.page_size as usize {
                format!("page_{}", req.page_token.parse::<u32>().unwrap_or(0) + 1)
            } else {
                String::new()
            },
            total_count: self.storage.count_users().await,
        };
        
        Ok(Response::new(response))
    }

    async fn authenticate_user(
        &self,
        request: Request<AuthenticateUserRequest>,
    ) -> Result<Response<AuthenticateUserResponse>, Status> {
        let req = request.into_inner();
        
        if let Some(mut user) = self.storage.get_user_by_email(&req.email).await {
            // In real implementation, verify password hash
            // For demo purposes, assume authentication succeeds
            
            let now = SystemTime::now();
            user.last_login = Some(Self::system_time_to_timestamp(now));
            self.storage.update_user(user.clone()).await;
            
            let token = format!("jwt_token_{}", Uuid::new_v4());
            let tomorrow = now + Duration::from_secs(3600 * 24);

            let response = AuthenticateUserResponse {
                user: Some(user),
                token,
                success: true,
                message: "Authentication successful".to_string(),
                expires_at: Some(Self::system_time_to_timestamp(tomorrow)),
            };
            
            Ok(Response::new(response))
        } else {
            let response = AuthenticateUserResponse {
                user: None,
                token: String::new(),
                success: false,
                message: "Invalid credentials".to_string(),
                expires_at: None,
            };
            
            Ok(Response::new(response))
        }
    }

    async fn get_user_tasks(
        &self,
        request: Request<GetUserTasksRequest>,
    ) -> Result<Response<GetUserTasksResponse>, Status> {
        let req = request.into_inner();
        
        let tasks = self.storage.get_tasks_by_user(&req.user_id, req.page_size, &req.page_token).await;
        
        let response = GetUserTasksResponse {
            tasks: tasks.clone(),
            next_page_token: if tasks.len() >= req.page_size as usize {
                format!("page_{}", req.page_token.parse::<u32>().unwrap_or(0) + 1)
            } else {
                String::new()
            },
            total_count: self.storage.count_user_tasks(&req.user_id).await,
        };
        
        Ok(Response::new(response))
    }

    async fn update_user_preferences(
        &self,
        request: Request<UpdateUserPreferencesRequest>,
    ) -> Result<Response<UpdateUserPreferencesResponse>, Status> {
        let req = request.into_inner();
        
        if let Some(mut user) = self.storage.get_user(&req.user_id).await {
            user.preferences = req.preferences;
            user.updated_at = Some(Self::system_time_to_timestamp(SystemTime::now()));
            
            self.storage.update_user(user.clone()).await;
            
            let response = UpdateUserPreferencesResponse {
                preferences: user.preferences,
                success: true,
                message: "Preferences updated successfully".to_string(),
            };
            
            Ok(Response::new(response))
        } else {
            Err(Status::not_found("User not found"))
        }
    }

    // Legacy authentication methods (required by the trait)
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        
        // In a real implementation, you'd authenticate via username/password
        // For demo purposes, we'll just check if a user exists with that username
        if let Some(mut user) = self.storage.get_user_by_username(&req.username).await {
            let now = SystemTime::now();
            user.last_login = Some(Self::system_time_to_timestamp(now));
            self.storage.update_user(user.clone()).await;
            
            let access_token = format!("access_token_{}", Uuid::new_v4());
            let refresh_token = format!("refresh_token_{}", Uuid::new_v4());
            let expires_at = now + Duration::from_secs(3600); // 1 hour

            let response = LoginResponse {
                access_token,
                refresh_token,
                user: Some(user),
                expires_at: Some(Self::system_time_to_timestamp(expires_at)),
            };
            
            Ok(Response::new(response))
        } else {
            Err(Status::unauthenticated("Invalid credentials"))
        }
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let _req = request.into_inner();
        
        // In a real implementation, you'd validate the refresh token
        // For demo purposes, we'll just generate a new access token
        let access_token = format!("access_token_{}", Uuid::new_v4());
        let expires_at = SystemTime::now() + Duration::from_secs(3600); // 1 hour

        let response = RefreshTokenResponse {
            access_token,
            expires_at: Some(Self::system_time_to_timestamp(expires_at)),
        };
        
        Ok(Response::new(response))
    }

    async fn logout(
        &self,
        _request: Request<LogoutRequest>,
    ) -> Result<Response<()>, Status> {
        // In a real implementation, you'd invalidate the token
        // For demo purposes, we'll just return success
        Ok(Response::new(()))
    }
}