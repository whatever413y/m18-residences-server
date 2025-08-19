# M18 Residences Server

A backend service for managing room rental, tenants, bills, and electricity readings built with Rust and Axum.

## Features

- Tenant and room management
- Bill generation and tracking
- Electricity reading records
- JWT-based authentication (admin and tenant)
- Cloudflare R2 file uploads and signed URLs for receipts.
- RESTful API endpoints

## Project Structure

m18-residences-service/
├── src/
│   ├── main.rs                # Application entry point
│   ├── entities/              # Database models (SeaORM entities)
│   ├── handlers/              # HTTP request handlers (Axum)
│   ├── middleware/            # Authentication, CORS, etc.
│   ├── repository/            # Database access logic
│   ├── routes/                # Route definitions
│   ├── services/              # Business logic, integrations (e.g., S3, JWT)
├── Cargo.toml                 # Rust package manifest
├── .env                       # Environment variables
├── .env.example               # Example env file for reference
├── README.md                  # Project documentation

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- PostgreSQL database
- Cloudflare R2 account (for file storage)

### Setup

1. Clone the repository:

   ```sh
   git clone https://github.com/yourusername/m18-residences-server.git
   cd m18-residences-server

2. Copy `.env.example` to `.env` and fill in your configuration:

   - Database connection string
   - JWT secret
   - Admin credentials
   - Cloudflare R2 credentials

3. Run database migrations.

4. Build and run the server:

   ```sh
   cargo build
   cargo run
   ```

The server will start on the port specified in your `.env` file.

## API Endpoints

- `/api/auth` - Authentication routes (admin and tenant login, token validation)
- `/api/rooms` - Room management (CRUD)
- `/api/tenants` - Tenant management (CRUD)
- `/api/electricity-readings` - Electricity readings (CRUD)
- `/api/bills` - Bill management (CRUD, file upload)
- `/api/signed-urls` - Generate signed URLs for receipts and payments

All routes except `/api/auth` require JWT authentication.
