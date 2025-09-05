import { createClient, Transport } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';

import { UserService, TaskService } from '@/proto/message_pb';

const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:50051'; //envoy proxy

export const transport: Transport = createGrpcWebTransport({
  baseUrl: apiUrl,
});

export const userClient = createClient(UserService, transport);
export const taskClient = createClient(TaskService, transport);

export const createClientBundle = () => ({
  userClient: createClient(UserService, transport),
  taskClient: createClient(TaskService, transport),
});
