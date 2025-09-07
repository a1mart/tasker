# gRPC-Based Task Management Application

![Home](https://raw.githubusercontent.com/a1mart/tasker/main/docs/assets/tasks.png)

Personal task management system to support DevOps and increase productivity.

## Setup

### Backend
```bash
# scaffold Rust binary
cargo new backend
```

### Frontend
```bash
# scaffold the Next.js application
npx create-next-app@latest frontend --typescript --tailwind --eslint
# initalize shadcn component library
npx shadcn@latest init
# add shadcn components
npx shadcn@latest add 
```

## Running
### Backend
```bash
cargo run
```

### Frontend
```bash
npx run dev
```

Or use the Makefile