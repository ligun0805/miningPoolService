# Mining Pool Service

## Overview

A Rust HTTP service for collecting mining worker reports and providing real-time pool statistics. The service accepts worker reports via POST requests and provides aggregated statistics through GET requests. Data is stored in-memory with automatic filtering of old reports (older than 5 minutes) for statistics generation.

## User Preferences

Preferred communication style: Simple, everyday language.

## System Architecture

### Technology Stack
- **Language**: Rust (2021 edition)
- **HTTP Framework**: Axum 0.7 (async web framework)
- **Runtime**: Tokio 1.0 (async runtime)
- **Serialization**: Serde 1.0 with JSON support
- **Time Handling**: Chrono 0.4 for timestamp operations
- **Data Storage**: In-memory using Arc<Mutex<Vec<>>> for thread-safe storage

### Architecture Pattern
RESTful HTTP service with async request handling. Uses Rust's ownership model with shared state management through Arc and Mutex for concurrent access to the in-memory data store.

## Key Components

1. **HTTP API Layer**: 
   - POST /report - Accepts worker mining reports
   - GET /stats - Returns aggregated pool statistics
   - GET /health - Health check endpoint

2. **Data Models**:
   - WorkerReport: Individual worker mining report
   - PoolStats: Aggregated statistics per pool
   - StatsResponse: API response structure

3. **In-Memory Storage**: Thread-safe vector storage for reports

4. **Business Logic**: 
   - Report validation (worker_id, pool, hashrate, timestamp)
   - Statistics aggregation with 5-minute sliding window
   - Unique worker counting per pool
   - Automatic data filtering and cleanup

## Data Flow

1. **Report Submission**: POST /report receives JSON worker reports
2. **Validation**: Validates required fields and data constraints
3. **Storage**: Stores valid reports in in-memory vector
4. **Statistics Generation**: GET /stats filters recent reports (last 5 minutes) from memory
5. **Aggregation**: Calculates worker count, average hashrate, and temperature per pool
6. **Response**: Returns aggregated statistics as JSON

## API Endpoints

### POST /report
Accepts mining worker reports in JSON format:
```json
{
  "worker_id": "worker-123",
  "pool": "us-east", 
  "hashrate": 50.5,
  "temperature": 68,
  "timestamp": 1720708712
}
```

### GET /stats
Returns aggregated pool statistics:
```json
{
  "pools": {
    "us-east": {
      "workers": 2,
      "avg_hashrate": 46.4,
      "avg_temp": 66.5
    },
    "eu-west": {
      "workers": 1,
      "avg_hashrate": 39.0,
      "avg_temp": 61.0
    }
  }
}
```

## External Dependencies

- **axum**: Web framework for HTTP handling
- **tokio**: Async runtime for concurrent operations
- **serde**: JSON serialization/deserialization
- **chrono**: Timestamp handling and time operations


## Deployment Strategy

- **Port**: Service runs on 0.0.0.0:5000
- **Build**: `cargo build` for compilation
- **Run**: `cargo run` to start the service


## Validation Rules

- worker_id: Cannot be empty
- pool: Cannot be empty  
- hashrate: Must be >= 0.0
- timestamp: Must be > 0
- Only reports from last 5 minutes included in statistics