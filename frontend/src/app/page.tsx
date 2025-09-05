'use client';
import React, { useState, useEffect, useCallback } from 'react';
import {
  Plus,
  Search,
  Filter,
  CheckCircle2,
  Circle,
  Clock,
  User as UserIcon,
  Calendar,
  Tag,
  MoreHorizontal,
  Edit3,
  Trash2,
  Upload,
  BarChart3,
  Settings,
  AlertCircle,
  Loader2,
  Wifi,
  WifiOff,
  Users,
  ClipboardList,
  TrendingUp,
  RefreshCw,
  FileText,
  UserPlus,
} from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Progress } from '@/components/ui/progress';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

import { userClient, taskClient } from '@/lib/client';
import {
  User,
  UserRole,
  UserStatus,
  Task,
  TaskAnalytics,
  TaskStatus,
  TaskPriority,
  TaskSortField,
} from '@/proto/message_pb';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

// Utility functions
const toTimestamp = (date: Date) => ({
  seconds: BigInt(Math.floor(date.getTime() / 1000)),
  nanos: (date.getTime() % 1000) * 1_000_000,
});

const timestampToDate = (ts: { seconds: bigint | number; nanos: number }): Date => {
  const seconds = typeof ts.seconds === 'bigint' ? Number(ts.seconds) : ts.seconds;
  const millis = seconds * 1000 + Math.floor(ts.nanos / 1_000_000);
  return new Date(millis);
};

// Helper functions for enum conversion
const getStatusLabel = (status: TaskStatus) => {
  switch (status) {
    case TaskStatus.TODO:
      return 'To Do';
    case TaskStatus.IN_PROGRESS:
      return 'In Progress';
    case TaskStatus.REVIEW:
      return 'Review';
    case TaskStatus.DONE:
      return 'Done';
    case TaskStatus.CANCELLED:
      return 'Cancelled';
    default:
      return 'Unknown';
  }
};

const getPriorityLabel = (priority: TaskPriority) => {
  switch (priority) {
    case TaskPriority.LOW:
      return 'Low';
    case TaskPriority.MEDIUM:
      return 'Medium';
    case TaskPriority.HIGH:
      return 'High';
    case TaskPriority.CRITICAL:
      return 'Critical';
    default:
      return 'Unknown';
  }
};

const getRoleLabel = (role: UserRole) => {
  switch (role) {
    case UserRole.VIEWER:
      return 'Viewer';
    case UserRole.MEMBER:
      return 'Member';
    case UserRole.ADMIN:
      return 'Admin';
    default:
      return 'Unknown';
  }
};

const getUserStatusLabel = (status: UserStatus) => {
  switch (status) {
    case UserStatus.ACTIVE:
      return 'Active';
    case UserStatus.INACTIVE:
      return 'Inactive';
    case UserStatus.SUSPENDED:
      return 'Suspended';
    default:
      return 'Unknown';
  }
};

const TaskManagementApp: React.FC = () => {
  // State
  const [activeTab, setActiveTab] = useState<'tasks' | 'analytics' | 'users'>('tasks');
  const [tasks, setTasks] = useState<Task[]>([]);
  const [users, setUsers] = useState<User[]>([]);
  const [analytics, setAnalytics] = useState<TaskAnalytics | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [filterStatus, setFilterStatus] = useState<number | 'all'>('all');

  const [filterStatuses, setFilterStatuses] = useState<number[]>([]); // Multiple statuses
  const [filterUsers, setFilterUsers] = useState<string[]>([]); // Multiple users (IDs or names)

  const [loading, setLoading] = useState(false);
  const [globalLoading, setGlobalLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<
    'connected' | 'disconnected' | 'connecting'
  >('connecting');

  // Dialog states
  const [showCreateTask, setShowCreateTask] = useState(false);
  const [showCreateUser, setShowCreateUser] = useState(false);
  const [editingTask, setEditingTask] = useState<Task | null>(null);
  const [editingUser, setEditingUser] = useState<User | null>(null);

  // Connection status check
  const checkConnection = useCallback(async () => {
    try {
      setConnectionStatus('connecting');
      // Try a simple health check or list operation
      await taskClient.listTasks({
        pageSize: 1,
        filter: {},
        sort: { field: TaskSortField.CREATED_AT, direction: 1 },
      });
      setConnectionStatus('connected');
      return true;
    } catch (error) {
      setConnectionStatus('disconnected');
      console.error('gRPC connection failed:', error);
      setError('Failed to connect to server. Please check your connection.');
      return false;
    }
  }, []);

  // Fetch all data
  const fetchAllData = useCallback(async () => {
    const isConnected = await checkConnection();
    if (!isConnected) return;

    setGlobalLoading(true);
    try {
      await Promise.all([fetchTasks(), fetchUsers(), fetchAnalytics()]);
      setError(null);
    } catch (err) {
      console.error('Failed to fetch data:', err);
      setError('Failed to load application data');
    } finally {
      setGlobalLoading(false);
    }
  }, []);

  // Fetch Tasks
  const fetchTasks = useCallback(async () => {
    try {
      const response = await taskClient.listTasks({
        pageSize: 100,
        filter: {},
        sort: {
          field: TaskSortField.CREATED_AT,
          direction: 1,
        },
      });
      setTasks(response.tasks || []);
    } catch (err) {
      console.error('Failed to fetch tasks:', err);
      throw err;
    }
  }, []);

  // Fetch Users
  const fetchUsers = useCallback(async () => {
    try {
      const response = await userClient.listUsers({
        pageSize: 100,
        activeOnly: false,
      });
      setUsers(response.users || []);
    } catch (err) {
      console.error('Failed to fetch users:', err);
      throw err;
    }
  }, []);

  // Fetch Analytics
  const fetchAnalytics = useCallback(async () => {
    try {
      const now = new Date();
      const monthAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);

      const response = await taskClient.getTaskAnalytics({
        startDate: toTimestamp(monthAgo),
        endDate: toTimestamp(now),
        groupBy: 'status',
      });

      setAnalytics(response.analytics);
    } catch (err) {
      console.error('Failed to fetch analytics:', err);
      // Don't throw - analytics are optional
    }
  }, []);

  // Create Task
  const createTask = async (taskData: Partial<Task>) => {
    setLoading(true);
    try {
      const request = {
        title: taskData.title ?? '',
        description: taskData.description ?? '',
        priority: taskData.priority ?? TaskPriority.MEDIUM,
        assignedTo: taskData.assignedTo ?? '',
        tags: taskData.tags ?? [],
        dueDate: taskData.dueDate ? toTimestamp(timestampToDate(taskData.dueDate)) : undefined,
      };

      const response = await taskClient.createTask(request);

      if (response.success) {
        await fetchAllData(); // Refresh all data
        setShowCreateTask(false);
        setError(null);
      } else {
        throw new Error(response.message || 'Failed to create task');
      }
    } catch (err) {
      setError('Failed to create task');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Update Task
  const updateTask = async (taskId: string, updates: Partial<Task>) => {
    setLoading(true);
    try {
      const request = {
        id: taskId,
        task: updates,
        updateMask: Object.keys(updates),
      };

      const response = await taskClient.updateTask(request);
      console.log(response.data);
      if (response.success) {
        await fetchAllData(); // Refresh all data
        setEditingTask(null);
        setError(null);
      } else {
        throw new Error(response.message || 'Failed to update task');
      }
    } catch (err) {
      setError('Failed to update task');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Delete Task
  const deleteTask = async (taskId: string) => {
    if (!window.confirm('Are you sure you want to delete this task?')) return;

    setLoading(true);
    try {
      const response = await taskClient.deleteTask({
        id: taskId,
        force: false,
      });

      if (response.success) {
        await fetchAllData(); // Refresh all data
        setError(null);
      } else {
        throw new Error(response.message || 'Failed to delete task');
      }
    } catch (err) {
      setError('Failed to delete task');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Create User
  const createUser = async (userData: any) => {
    setLoading(true);
    try {
      const request = {
        username: userData.username,
        email: userData.email,
        password: userData.password,
        fullName: userData.fullName,
        role: userData.role,
      };

      const response = await userClient.createUser(request);

      if (response.success) {
        await fetchAllData(); // Refresh all data
        setShowCreateUser(false);
        setError(null);
      } else {
        throw new Error(response.message || 'Failed to create user');
      }
    } catch (err) {
      setError('Failed to create user');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Update User
  const updateUser = async (userId: string, updates: Partial<User>) => {
    setLoading(true);
    try {
      const request = {
        id: userId,
        user: updates,
        updateMask: Object.keys(updates),
      };

      const response = await userClient.updateUser(request);

      if (response.success) {
        await fetchAllData(); // Refresh all data
        setEditingUser(null);
        setError(null);
      } else {
        throw new Error(response.message || 'Failed to update user');
      }
    } catch (err) {
      setError('Failed to update user');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Delete User
  const deleteUser = async (userId: string) => {
    if (!window.confirm('Are you sure you want to delete this user?')) return;

    setLoading(true);
    try {
      const response = await userClient.deleteUser({
        id: userId,
        force: false,
      });

      if (response.success) {
        await fetchAllData(); // Refresh all data
        setError(null);
      } else {
        throw new Error(response.message || 'Failed to delete user');
      }
    } catch (err) {
      setError('Failed to delete user');
      throw err;
    } finally {
      setLoading(false);
    }
  };

  // Search Tasks
  const searchTasks = useCallback(
    async (query: string) => {
      if (!query.trim()) {
        await fetchTasks();
        return;
      }

      setLoading(true);
      try {
        const response = await taskClient.searchTasks({
          query: query,
          pageSize: 100,
          filters: filterStatus !== 'all' ? { status: [filterStatus] } : {},
        });

        setTasks(response.tasks || []);
        setError(null);
      } catch (err) {
        setError('Failed to search tasks');
      } finally {
        setLoading(false);
      }
    },
    [filterStatus]
  );

  // Initialize app
  useEffect(() => {
    fetchAllData();
  }, [fetchAllData]);

  // Handle search with debounce
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      if (connectionStatus === 'connected') {
        searchTasks(searchTerm);
      }
    }, 500);

    return () => clearTimeout(timeoutId);
  }, [searchTerm, searchTasks, connectionStatus]);

  // Filtered tasks
  const filteredTasks = tasks.filter((task) => {
    const matchesStatus = filterStatuses.length === 0 || filterStatuses.includes(task.status);

    // Fix: Compare against task.assignedTo (username) instead of user.id
    const matchesUser =
      filterUsers.length === 0 ||
      (task.assignedTo && filterUsers.includes(task.assignedTo)) ||
      (filterUsers.includes('unassigned') && !task.assignedTo);

    const matchesSearch =
      searchTerm.trim() === '' ||
      task.title.toLowerCase().includes(searchTerm.toLowerCase()) ||
      task.description.toLowerCase().includes(searchTerm.toLowerCase());

    return matchesStatus && matchesUser && matchesSearch;
  });

  // UI helpers
  const getPriorityVariant = (priority: TaskPriority) => {
    switch (priority) {
      case TaskPriority.CRITICAL:
        return 'destructive';
      case TaskPriority.HIGH:
        return 'destructive';
      case TaskPriority.MEDIUM:
        return 'secondary';
      case TaskPriority.LOW:
        return 'outline';
      default:
        return 'default';
    }
  };

  const getStatusIcon = (status: TaskStatus) => {
    switch (status) {
      case TaskStatus.DONE:
        return <CheckCircle2 className="w-4 h-4 text-green-600" />;
      case TaskStatus.IN_PROGRESS:
        return <Clock className="w-4 h-4 text-blue-600" />;
      case TaskStatus.REVIEW:
        return <Clock className="w-4 h-4 text-yellow-600" />;
      default:
        return <Circle className="w-4 h-4 text-gray-400" />;
    }
  };

  const updateTaskStatus = async (taskId: string, newStatus: TaskStatus) => {
    await updateTask(taskId, { status: newStatus });
  };

  // Connection Status Component
  const ConnectionStatus = () => (
    <div className="flex items-center gap-2 text-sm">
      {connectionStatus === 'connected' && (
        <>
          <Wifi className="w-4 h-4 text-green-600" />
          <span className="text-green-600">Connected</span>
        </>
      )}
      {connectionStatus === 'disconnected' && (
        <>
          <WifiOff className="w-4 h-4 text-red-600" />
          <span className="text-red-600">Disconnected</span>
        </>
      )}
      {connectionStatus === 'connecting' && (
        <>
          <Loader2 className="w-4 h-4 text-yellow-600 animate-spin" />
          <span className="text-yellow-600">Connecting...</span>
        </>
      )}
    </div>
  );

  // Task Form Component
  const TaskForm = ({
    task,
    onSubmit,
    onCancel,
  }: {
    task?: Task | null;
    onSubmit: (data: Partial<Task>) => Promise<void>;
    onCancel: () => void;
  }) => {
    const [formData, setFormData] = useState({
      title: task?.title || '',
      description: task?.description || '',
      priority: task?.priority || TaskPriority.MEDIUM,
      assignedTo: task?.assignedTo || '',
      dueDate: task?.dueDate ? timestampToDate(task.dueDate).toISOString().split('T')[0] : '',
      tags: task?.tags?.join(', ') || '',
    });

    const handleSubmit = async (e: React.FormEvent) => {
      e.preventDefault();
      try {
        await onSubmit({
          title: formData.title,
          description: formData.description,
          priority: formData.priority,
          assignedTo: formData.assignedTo,
          dueDate: formData.dueDate ? toTimestamp(new Date(formData.dueDate)) : undefined,
          tags: formData.tags
            .split(',')
            .map((t) => t.trim())
            .filter(Boolean),
        });
      } catch (err) {
        // Error handled by parent
      }
    };

    return (
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="title">Title</Label>
          <Input
            id="title"
            required
            value={formData.title}
            onChange={(e) => setFormData({ ...formData, title: e.target.value })}
            placeholder="Enter task title"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="description">Description</Label>
          <Textarea
            id="description"
            value={formData.description}
            onChange={(e) => setFormData({ ...formData, description: e.target.value })}
            placeholder="Enter task description"
            rows={3}
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="priority">Priority</Label>
            <Select
              value={formData.priority.toString()}
              onValueChange={(value) =>
                setFormData({ ...formData, priority: Number(value) as TaskPriority })
              }
            >
              <SelectTrigger>
                <SelectValue placeholder="Select priority" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={TaskPriority.LOW.toString()}>Low</SelectItem>
                <SelectItem value={TaskPriority.MEDIUM.toString()}>Medium</SelectItem>
                <SelectItem value={TaskPriority.HIGH.toString()}>High</SelectItem>
                <SelectItem value={TaskPriority.CRITICAL.toString()}>Critical</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="assignedTo">Assigned To</Label>
            <Select
              value={formData.assignedTo}
              onValueChange={(value) => setFormData({ ...formData, assignedTo: value })}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select assignee" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="unassigned">Unassigned</SelectItem>
                {users.map((user) => (
                  <SelectItem key={user.id} value={user.username}>
                    {user.fullName}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="space-y-2">
          <Label htmlFor="dueDate">Due Date</Label>
          <Input
            id="dueDate"
            type="date"
            value={formData.dueDate}
            onChange={(e) => setFormData({ ...formData, dueDate: e.target.value })}
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor="tags">Tags</Label>
          <Input
            id="tags"
            value={formData.tags}
            onChange={(e) => setFormData({ ...formData, tags: e.target.value })}
            placeholder="frontend, urgent, bug"
          />
        </div>

        <div className="flex justify-end gap-2 pt-4">
          <Button type="button" variant="outline" onClick={onCancel} disabled={loading}>
            Cancel
          </Button>
          <Button type="submit" disabled={loading || connectionStatus !== 'connected'}>
            {loading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {task ? 'Update Task' : 'Create Task'}
          </Button>
        </div>
      </form>
    );
  };

  // User Form Component
  const UserForm = ({
    user,
    onSubmit,
    onCancel,
  }: {
    user?: User | null;
    onSubmit: (data: any) => Promise<void>;
    onCancel: () => void;
  }) => {
    const [formData, setFormData] = useState({
      fullName: user?.fullName || '',
      username: user?.username || '',
      email: user?.email || '',
      password: '',
      role: user?.role?.toString() || UserRole.MEMBER.toString(),
      status: user?.status?.toString() || UserStatus.ACTIVE.toString(),
    });

    const handleSubmit = async (e: React.FormEvent) => {
      e.preventDefault();
      try {
        await onSubmit({
          fullName: formData.fullName,
          username: formData.username,
          email: formData.email,
          password: formData.password || undefined,
          role: Number(formData.role),
          status: Number(formData.status),
        });
      } catch (err) {
        // Error handled by parent
      }
    };

    return (
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="fullName">Full Name</Label>
          <Input
            id="fullName"
            required
            value={formData.fullName}
            onChange={(e) => setFormData({ ...formData, fullName: e.target.value })}
          />
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="username">Username</Label>
            <Input
              id="username"
              required
              value={formData.username}
              onChange={(e) => setFormData({ ...formData, username: e.target.value })}
              disabled={!!user} // Don't allow username changes
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              required
              value={formData.email}
              onChange={(e) => setFormData({ ...formData, email: e.target.value })}
            />
          </div>
        </div>

        {!user && (
          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <Input
              id="password"
              type="password"
              required
              value={formData.password}
              onChange={(e) => setFormData({ ...formData, password: e.target.value })}
            />
          </div>
        )}

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="role">Role</Label>
            <Select
              value={formData.role}
              onValueChange={(value) => setFormData({ ...formData, role: value })}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={UserRole.VIEWER.toString()}>Viewer</SelectItem>
                <SelectItem value={UserRole.MEMBER.toString()}>Member</SelectItem>
                <SelectItem value={UserRole.ADMIN.toString()}>Admin</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="status">Status</Label>
            <Select
              value={formData.status}
              onValueChange={(value) => setFormData({ ...formData, status: value })}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select status" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={UserStatus.ACTIVE.toString()}>Active</SelectItem>
                <SelectItem value={UserStatus.INACTIVE.toString()}>Inactive</SelectItem>
                <SelectItem value={UserStatus.SUSPENDED.toString()}>Suspended</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-4">
          <Button type="button" variant="outline" onClick={onCancel} disabled={loading}>
            Cancel
          </Button>
          <Button type="submit" disabled={loading || connectionStatus !== 'connected'}>
            {loading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {user ? 'Update User' : 'Create User'}
          </Button>
        </div>
      </form>
    );
  };

  // Task Card Component
  const TaskCard = ({ task }: { task: Task }) => {
    const updatedOrCreated = task.updatedAt || task.createdAt;
    const assignedUser = users.find((u) => u.username === task.assignedTo);

    return (
      <Card className="hover:shadow-lg transition-all duration-200 border-l-4 border-l-blue-500">
        <CardHeader className="pb-3">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-2 flex-1">
              {getStatusIcon(task.status)}
              <CardTitle className="text-base line-clamp-2">{task.title}</CardTitle>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant={getPriorityVariant(task.priority)}>
                {getPriorityLabel(task.priority)}
              </Badge>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm">
                    <MoreHorizontal className="w-4 h-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                  <DropdownMenuItem onClick={() => setEditingTask(task)}>
                    <Edit3 className="mr-2 h-4 w-4" />
                    Edit
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="text-red-600"
                    onClick={() => deleteTask(task.id)}
                    disabled={connectionStatus !== 'connected'}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    Delete
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>
        </CardHeader>

        <CardContent className="space-y-4">
          {task.description && (
            <CardDescription className="line-clamp-3">{task.description}</CardDescription>
          )}

          <div className="flex items-center gap-4 text-sm text-gray-500">
            <div className="flex items-center gap-1">
              <UserIcon className="w-4 h-4" />
              <span>{assignedUser?.fullName || task.assignedTo || 'Unassigned'}</span>
            </div>
            {task.dueDate && (
              <div className="flex items-center gap-1">
                <Calendar className="w-4 h-4" />
                <span>{timestampToDate(task.dueDate).toLocaleDateString()}</span>
              </div>
            )}
          </div>

          {task.tags && task.tags.length > 0 && (
            <div className="flex flex-wrap gap-1">
              {task.tags.map((tag) => (
                <Badge key={tag} variant="outline" className="text-xs">
                  <Tag className="w-3 h-3 mr-1" />
                  {tag}
                </Badge>
              ))}
            </div>
          )}

          {task.metrics && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span>Progress</span>
                <span>{task.metrics.completionPercentage}%</span>
              </div>
              <Progress value={task.metrics.completionPercentage} className="h-2" />
            </div>
          )}

          <div className="flex items-center justify-between">
            <div className="text-xs text-gray-500">
              Updated:{' '}
              {updatedOrCreated
                ? timestampToDate(updatedOrCreated).toLocaleDateString()
                : 'Unknown'}
            </div>
            <Select
              value={task.status.toString()}
              onValueChange={(value) => updateTaskStatus(task.id, Number(value) as TaskStatus)}
              disabled={connectionStatus !== 'connected'}
            >
              <SelectTrigger className="w-auto">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={TaskStatus.TODO.toString()}>To Do</SelectItem>
                <SelectItem value={TaskStatus.IN_PROGRESS.toString()}>In Progress</SelectItem>
                <SelectItem value={TaskStatus.REVIEW.toString()}>Review</SelectItem>
                <SelectItem value={TaskStatus.DONE.toString()}>Done</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>
    );
  };

  // Empty State Component
  const EmptyState = ({
    icon: Icon,
    title,
    description,
    actionLabel,
    onAction,
  }: {
    icon: React.ComponentType<any>;
    title: string;
    description: string;
    actionLabel?: string;
    onAction?: () => void;
  }) => (
    <Card className="text-center py-12">
      <CardContent className="pt-6">
        <div className="w-24 h-24 mx-auto mb-6 bg-gradient-to-br from-blue-50 to-indigo-100 rounded-full flex items-center justify-center">
          <Icon className="w-10 h-10 text-blue-500" />
        </div>
        <CardTitle className="mb-2 text-xl">{title}</CardTitle>
        <CardDescription className="mb-6 text-base max-w-md mx-auto">{description}</CardDescription>
        {actionLabel && onAction && (
          <Button
            onClick={onAction}
            size="lg"
            className="bg-gradient-to-r from-blue-500 to-indigo-600 hover:from-blue-600 hover:to-indigo-700"
          >
            <Plus className="w-4 h-4 mr-2" />
            {actionLabel}
          </Button>
        )}
      </CardContent>
    </Card>
  );

  // Analytics Dashboard Component
  const AnalyticsDashboard = () => {
    if (!analytics) {
      return (
        <div className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            {[1, 2, 3, 4].map((i) => (
              <Card key={i}>
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <div className="h-4 bg-gray-200 rounded w-20 animate-pulse"></div>
                  <div className="h-4 w-4 bg-gray-200 rounded animate-pulse"></div>
                </CardHeader>
                <CardContent>
                  <div className="h-8 bg-gray-200 rounded w-16 animate-pulse"></div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      );
    }

    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <Card className="bg-gradient-to-br from-blue-50 to-blue-100 border-blue-200">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-blue-700">Total Tasks</CardTitle>
              <BarChart3 className="h-5 w-5 text-blue-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-blue-900">{analytics.totalTasks}</div>
              <p className="text-xs text-blue-600 mt-1">All time</p>
            </CardContent>
          </Card>

          <Card className="bg-gradient-to-br from-green-50 to-green-100 border-green-200">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-green-700">Completed</CardTitle>
              <CheckCircle2 className="h-5 w-5 text-green-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-green-900">{analytics.completedTasks}</div>
              <p className="text-xs text-green-600 mt-1">Tasks finished</p>
            </CardContent>
          </Card>

          <Card className="bg-gradient-to-br from-yellow-50 to-yellow-100 border-yellow-200">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-yellow-700">In Progress</CardTitle>
              <Clock className="h-5 w-5 text-yellow-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-yellow-900">{analytics.inProgressTasks}</div>
              <p className="text-xs text-yellow-600 mt-1">Active tasks</p>
            </CardContent>
          </Card>

          <Card className="bg-gradient-to-br from-purple-50 to-purple-100 border-purple-200">
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium text-purple-700">Completion Rate</CardTitle>
              <TrendingUp className="h-5 w-5 text-purple-600" />
            </CardHeader>
            <CardContent>
              <div className="text-3xl font-bold text-purple-900">
                {analytics.completionRate.toFixed(1)}%
              </div>
              <p className="text-xs text-purple-600 mt-1">Success rate</p>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BarChart3 className="w-5 h-5" />
              Task Status Distribution
            </CardTitle>
            <CardDescription>Overview of current task statuses</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Circle className="w-4 h-4 text-gray-400" />
                  <span className="text-sm font-medium">To Do</span>
                </div>
                <div className="flex items-center gap-3">
                  <Progress
                    value={
                      analytics.totalTasks ? (analytics.todoTasks / analytics.totalTasks) * 100 : 0
                    }
                    className="w-32"
                  />
                  <span className="text-sm font-medium w-8 text-right">{analytics.todoTasks}</span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Clock className="w-4 h-4 text-blue-500" />
                  <span className="text-sm font-medium">In Progress</span>
                </div>
                <div className="flex items-center gap-3">
                  <Progress
                    value={
                      analytics.totalTasks
                        ? (analytics.inProgressTasks / analytics.totalTasks) * 100
                        : 0
                    }
                    className="w-32"
                  />
                  <span className="text-sm font-medium w-8 text-right">
                    {analytics.inProgressTasks}
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 text-green-500" />
                  <span className="text-sm font-medium">Completed</span>
                </div>
                <div className="flex items-center gap-3">
                  <Progress
                    value={
                      analytics.totalTasks
                        ? (analytics.completedTasks / analytics.totalTasks) * 100
                        : 0
                    }
                    className="w-32"
                  />
                  <span className="text-sm font-medium w-8 text-right">
                    {analytics.completedTasks}
                  </span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  };

  // User Management Component
  const UserManagement = () => (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold flex items-center gap-2">
            <Users className="w-6 h-6" />
            User Management
          </h2>
          <p className="text-gray-600 mt-1">Manage team members and their permissions</p>
        </div>
        <Button
          onClick={() => setShowCreateUser(true)}
          className="bg-gradient-to-r from-green-500 to-emerald-600 hover:from-green-600 hover:to-emerald-700"
        >
          <UserPlus className="w-4 h-4 mr-2" />
          Add User
        </Button>
      </div>

      {users.length === 0 ? (
        <EmptyState
          icon={Users}
          title="No users found"
          description="Start building your team by adding the first user to your workspace."
          actionLabel="Add First User"
          onAction={() => setShowCreateUser(true)}
        />
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>User</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Tasks Assigned</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {users.map((user) => (
                <TableRow key={user.id} className="hover:bg-gray-50">
                  <TableCell>
                    <div className="flex items-center gap-3">
                      <Avatar className="h-10 w-10">
                        <AvatarFallback className="bg-gradient-to-br from-blue-400 to-purple-500 text-white">
                          {user.fullName
                            .split(' ')
                            .map((n) => n[0])
                            .join('')
                            .toUpperCase()}
                        </AvatarFallback>
                      </Avatar>
                      <div>
                        <div className="font-medium">{user.fullName}</div>
                        <div className="text-sm text-gray-500">@{user.username}</div>
                        <div className="text-xs text-gray-400">{user.email}</div>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary" className="font-medium">
                      {getRoleLabel(user.role)}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={
                        getUserStatusLabel(user.status) === 'Active' ? 'default' : 'destructive'
                      }
                    >
                      {getUserStatusLabel(user.status)}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <ClipboardList className="w-4 h-4 text-gray-400" />
                      {tasks.filter((task) => task.assignedTo === user.username).length}
                    </div>
                  </TableCell>
                  <TableCell className="text-sm text-gray-500">
                    {user.createdAt
                      ? timestampToDate(user.createdAt).toLocaleDateString()
                      : 'Unknown'}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex gap-1 justify-end">
                      <Button variant="ghost" size="sm" onClick={() => setEditingUser(user)}>
                        <Edit3 className="w-4 h-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => deleteUser(user.id)}
                        className="text-red-600 hover:text-red-700 hover:bg-red-50"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      )}
    </div>
  );

  // Global loading state
  if (globalLoading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-blue-50 via-white to-indigo-50 flex items-center justify-center">
        <Card className="w-96 text-center p-8">
          <div className="w-16 h-16 mx-auto mb-4 bg-gradient-to-br from-blue-100 to-indigo-100 rounded-full flex items-center justify-center">
            <Loader2 className="w-8 h-8 text-blue-600 animate-spin" />
          </div>
          <h2 className="text-xl font-semibold mb-2">Loading Task Management System</h2>
          <p className="text-gray-600 mb-4">Connecting to server and loading data...</p>
          <ConnectionStatus />
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 via-white to-indigo-50">
      {/* Header */}
      <header className="bg-white/80 backdrop-blur-sm border-b border-blue-100 sticky top-0 z-50">
        <div className="container mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-gradient-to-br from-blue-500 to-indigo-600 rounded-lg flex items-center justify-center">
                <ClipboardList className="w-5 h-5 text-white" />
              </div>
              <h1 className="text-xl font-bold bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent">
                Task Management System
              </h1>
            </div>
            <div className="flex items-center space-x-4">
              <ConnectionStatus />
              <Button
                variant="ghost"
                size="sm"
                onClick={fetchAllData}
                disabled={loading || connectionStatus !== 'connected'}
              >
                <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
              </Button>
              <Button variant="ghost" size="sm">
                <Settings className="w-4 h-4" />
              </Button>
              <div className="flex items-center space-x-2">
                <Avatar>
                  <AvatarFallback className="bg-gradient-to-br from-blue-500 to-indigo-600 text-white">
                    JD
                  </AvatarFallback>
                </Avatar>
                <span className="text-sm font-medium">John Doe</span>
              </div>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {error && (
          <Alert className="mb-6" variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertTitle>Error</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
            <Button variant="outline" size="sm" className="mt-2" onClick={fetchAllData}>
              <RefreshCw className="w-4 h-4 mr-2" />
              Retry
            </Button>
          </Alert>
        )}

        <Tabs value={activeTab} onValueChange={setActiveTab} className="space-y-6">
          <TabsList className="bg-white/60 backdrop-blur-sm border border-blue-100">
            <TabsTrigger
              value="tasks"
              className="data-[state=active]:bg-blue-500 data-[state=active]:text-white"
            >
              <ClipboardList className="w-4 h-4 mr-2" />
              Tasks
            </TabsTrigger>
            <TabsTrigger
              value="analytics"
              className="data-[state=active]:bg-blue-500 data-[state=active]:text-white"
            >
              <BarChart3 className="w-4 h-4 mr-2" />
              Analytics
            </TabsTrigger>
            <TabsTrigger
              value="users"
              className="data-[state=active]:bg-blue-500 data-[state=active]:text-white"
            >
              <Users className="w-4 h-4 mr-2" />
              Users
            </TabsTrigger>
          </TabsList>

          <TabsContent value="tasks" className="space-y-6">
            {/* Task Controls */}
            <Card className="bg-white/60 backdrop-blur-sm border-blue-100">
              <CardContent className="pt-6">
                <div className="flex flex-col sm:flex-row gap-4 justify-between items-start sm:items-center">
                  <div className="flex flex-col sm:flex-row gap-4 flex-1">
                    <div className="relative">
                      <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
                      <Input
                        placeholder="Search tasks..."
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                        className="pl-10 bg-white/80"
                      />
                    </div>
                    <div className="flex items-center gap-2">
                      <Filter className="w-4 h-4 text-gray-500" />
                      <Popover>
                        <PopoverTrigger asChild>
                          <Button variant="outline" className="w-40 bg-white/80">
                            {filterStatuses.length === 0
                              ? 'All Statuses'
                              : `${filterStatuses.length} selected`}
                          </Button>
                        </PopoverTrigger>
                        <PopoverContent className="w-48">
                          <div className="flex flex-col gap-2">
                            {Object.values(TaskStatus)
                              .filter((value): value is TaskStatus => typeof value === 'number')
                              .map((status) => (
                                <label key={status} className="flex items-center gap-2">
                                  <input
                                    type="checkbox"
                                    checked={filterStatuses.includes(status)}
                                    onChange={(e) => {
                                      if (e.target.checked) {
                                        setFilterStatuses([...filterStatuses, status]);
                                      } else {
                                        setFilterStatuses(
                                          filterStatuses.filter((s) => s !== status)
                                        );
                                      }
                                    }}
                                  />
                                  {getStatusLabel(status)}
                                </label>
                              ))}
                          </div>
                        </PopoverContent>
                      </Popover>

                      <Popover>
                        <PopoverTrigger asChild>
                          <Button variant="outline" className="w-40 bg-white/80">
                            {filterUsers.length === 0
                              ? 'All Users'
                              : `${filterUsers.length} selected`}
                          </Button>
                        </PopoverTrigger>
                        <PopoverContent className="w-48">
                          <div className="flex flex-col gap-2">
                            {/* Add unassigned option */}
                            <label className="flex items-center gap-2">
                              <input
                                type="checkbox"
                                checked={filterUsers.includes('unassigned')}
                                onChange={(e) => {
                                  if (e.target.checked) {
                                    setFilterUsers([...filterUsers, 'unassigned']);
                                  } else {
                                    setFilterUsers(filterUsers.filter((u) => u !== 'unassigned'));
                                  }
                                }}
                              />
                              Unassigned
                            </label>
                            {/* Use username instead of user.id */}
                            {users.map((user) => (
                              <label key={user.id} className="flex items-center gap-2">
                                <input
                                  type="checkbox"
                                  checked={filterUsers.includes(user.username)}
                                  onChange={(e) => {
                                    if (e.target.checked) {
                                      setFilterUsers([...filterUsers, user.username]);
                                    } else {
                                      setFilterUsers(
                                        filterUsers.filter((u) => u !== user.username)
                                      );
                                    }
                                  }}
                                />
                                {user.fullName} (@{user.username})
                              </label>
                            ))}
                          </div>
                        </PopoverContent>
                      </Popover>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    {/* <Button variant="outline" className="bg-white/80">
                      <Upload className="w-4 h-4 mr-2" />
                      Import
                    </Button> */}
                    <Button
                      onClick={() => setShowCreateTask(true)}
                      className="bg-gradient-to-r from-blue-500 to-indigo-600 hover:from-blue-600 hover:to-indigo-700"
                    >
                      <Plus className="w-4 h-4 mr-2" />
                      New Task
                    </Button>
                  </div>
                </div>
              </CardContent>
            </Card>

            {/* Loading State */}
            {loading && (
              <div className="flex items-center justify-center py-12">
                <Card className="bg-white/60 backdrop-blur-sm border-blue-100 p-8">
                  <div className="flex items-center gap-3">
                    <Loader2 className="h-6 w-6 animate-spin text-blue-600" />
                    <span className="text-gray-700">Loading tasks...</span>
                  </div>
                </Card>
              </div>
            )}

            {/* Tasks Grid */}
            {!loading && filteredTasks.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {filteredTasks.map((task) => (
                  <TaskCard key={task.id} task={task} />
                ))}
              </div>
            )}

            {/* Empty State */}
            {!loading && filteredTasks.length === 0 && (
              <EmptyState
                icon={searchTerm || filterStatus !== 'all' ? Search : FileText}
                title={searchTerm || filterStatus !== 'all' ? 'No tasks found' : 'No tasks yet'}
                description={
                  searchTerm || filterStatus !== 'all'
                    ? "Try adjusting your search or filter criteria to find what you're looking for."
                    : 'Start organizing your work by creating your first task. You can assign it to team members, set priorities, and track progress.'
                }
                actionLabel={searchTerm || filterStatus !== 'all' ? undefined : 'Create First Task'}
                onAction={
                  searchTerm || filterStatus !== 'all' ? undefined : () => setShowCreateTask(true)
                }
              />
            )}
          </TabsContent>

          <TabsContent value="analytics">
            <AnalyticsDashboard />
          </TabsContent>

          <TabsContent value="users">
            <UserManagement />
          </TabsContent>
        </Tabs>
      </main>

      {/* Create Task Dialog */}
      <Dialog open={showCreateTask} onOpenChange={setShowCreateTask}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Create New Task</DialogTitle>
            <DialogDescription>
              Add a new task to your project. Fill in the details below.
            </DialogDescription>
          </DialogHeader>
          <TaskForm onSubmit={createTask} onCancel={() => setShowCreateTask(false)} />
        </DialogContent>
      </Dialog>

      {/* Edit Task Dialog */}
      <Dialog open={!!editingTask} onOpenChange={() => setEditingTask(null)}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Edit Task</DialogTitle>
            <DialogDescription>Update task details and settings.</DialogDescription>
          </DialogHeader>
          <TaskForm
            task={editingTask}
            onSubmit={(data) => updateTask(editingTask!.id, data)}
            onCancel={() => setEditingTask(null)}
          />
        </DialogContent>
      </Dialog>

      {/* Create User Dialog */}
      <Dialog open={showCreateUser} onOpenChange={setShowCreateUser}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Add New User</DialogTitle>
            <DialogDescription>
              Create a new user account with credentials and role.
            </DialogDescription>
          </DialogHeader>
          <UserForm onSubmit={createUser} onCancel={() => setShowCreateUser(false)} />
        </DialogContent>
      </Dialog>

      {/* Edit User Dialog */}
      <Dialog open={!!editingUser} onOpenChange={() => setEditingUser(null)}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Edit User</DialogTitle>
            <DialogDescription>Update user information and permissions.</DialogDescription>
          </DialogHeader>
          <UserForm
            user={editingUser}
            onSubmit={(data) => updateUser(editingUser!.id, data)}
            onCancel={() => setEditingUser(null)}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default TaskManagementApp;
