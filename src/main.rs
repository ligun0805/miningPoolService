use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

// Data structures for the mining reports and statistics
#[derive(Debug, Clone, Deserialize, Serialize)]
struct WorkerReport {
    worker_id: String,
    pool: String,
    hashrate: f64,
    temperature: i32,
    timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
struct PoolStats {
    workers: usize,
    avg_hashrate: f64,
    avg_temp: f64,
}

#[derive(Debug, Clone, Serialize)]
struct StatsResponse {
    pools: HashMap<String, PoolStats>,
}

// In-memory storage for reports
type ReportsStorage = Arc<Mutex<Vec<WorkerReport>>>;

// Application state
#[derive(Clone)]
struct AppState {
    reports: ReportsStorage,
}

impl AppState {
    fn new() -> Self {
        Self {
            reports: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

// Handler for POST /report
async fn post_report(
    State(state): State<AppState>,
    Json(report): Json<WorkerReport>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate the report data
    if report.worker_id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "worker_id cannot be empty".to_string()));
    }
    
    if report.pool.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "pool cannot be empty".to_string()));
    }
    
    if report.hashrate < 0.0 {
        return Err((StatusCode::BAD_REQUEST, "hashrate cannot be negative".to_string()));
    }
    
    if report.timestamp <= 0 {
        return Err((StatusCode::BAD_REQUEST, "timestamp must be positive".to_string()));
    }

    // Store the report in memory
    match state.reports.lock() {
        Ok(mut reports) => {
            reports.push(report);
            Ok(StatusCode::OK)
        }
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to store report".to_string())),
    }
}

// Handler for GET /stats
async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    // Get current time and calculate 5 minutes ago
    let now = Utc::now().timestamp();
    let five_minutes_ago = now - 300; // 5 minutes = 300 seconds

    // Get all reports and filter for recent ones
    let recent_reports = match state.reports.lock() {
        Ok(reports) => {
            reports
                .iter()
                .filter(|report| report.timestamp > five_minutes_ago)
                .cloned()
                .collect::<Vec<_>>()
        }
        Err(_) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to access reports".to_string()));
        }
    };

    // Group reports by pool and calculate statistics
    let mut pool_stats: HashMap<String, PoolStats> = HashMap::new();
    let mut pool_reports: HashMap<String, Vec<&WorkerReport>> = HashMap::new();

    // Group reports by pool
    for report in &recent_reports {
        pool_reports
            .entry(report.pool.clone())
            .or_insert_with(Vec::new)
            .push(report);
    }

    // Calculate statistics for each pool
    for (pool_name, reports) in pool_reports {
        if reports.is_empty() {
            continue;
        }

        // Count unique workers
        let mut unique_workers = std::collections::HashSet::new();
        for report in &reports {
            unique_workers.insert(&report.worker_id);
        }

        // Calculate averages
        let total_hashrate: f64 = reports.iter().map(|r| r.hashrate).sum();
        let total_temp: f64 = reports.iter().map(|r| r.temperature as f64).sum();
        let report_count = reports.len() as f64;

        let avg_hashrate = total_hashrate / report_count;
        let avg_temp = total_temp / report_count;

        pool_stats.insert(
            pool_name,
            PoolStats {
                workers: unique_workers.len(),
                avg_hashrate: (avg_hashrate * 10.0).round() / 10.0, // Round to 1 decimal place
                avg_temp: (avg_temp * 10.0).round() / 10.0, // Round to 1 decimal place
            },
        );
    }

    Ok(Json(StatsResponse { pools: pool_stats }))
}

// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

// Web interface endpoint
async fn web_interface() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mining Pool Service</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 20px;
            background-color: #f5f5f5;
        }
        .container {
            background-color: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            text-align: center;
        }
        .endpoint {
            background-color: #f8f9fa;
            padding: 15px;
            margin: 10px 0;
            border-radius: 5px;
            border-left: 4px solid #007bff;
        }
        .method {
            font-weight: bold;
            color: #007bff;
        }
        .stats {
            margin-top: 20px;
            padding: 15px;
            background-color: #e9ecef;
            border-radius: 5px;
        }
        button {
            background-color: #007bff;
            color: white;
            border: none;
            padding: 10px 20px;
            border-radius: 5px;
            cursor: pointer;
            margin: 5px;
        }
        button:hover {
            background-color: #0056b3;
        }
        #stats-output {
            background-color: #f8f9fa;
            padding: 15px;
            margin-top: 10px;
            border-radius: 5px;
            border: 1px solid #dee2e6;
            font-family: monospace;
            white-space: pre-wrap;
        }
        .form-group {
            margin-bottom: 15px;
        }
        label {
            display: block;
            margin-bottom: 5px;
            font-weight: bold;
        }
        input[type="text"], input[type="number"] {
            width: 100%;
            padding: 8px;
            border: 1px solid #ddd;
            border-radius: 4px;
            box-sizing: border-box;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🏗️ Mining Pool Service</h1>
        
        <div class="endpoint">
            <div class="method">POST /report</div>
            <p>Submit mining worker reports in JSON format:</p>
            <div class="form-group">
                <label for="worker_id">Worker ID:</label>
                <input type="text" id="worker_id" value="worker-123" />
            </div>
            <div class="form-group">
                <label for="pool">Pool:</label>
                <input type="text" id="pool" value="us-east" />
            </div>
            <div class="form-group">
                <label for="hashrate">Hashrate:</label>
                <input type="number" id="hashrate" value="50.5" step="0.1" />
            </div>
            <div class="form-group">
                <label for="temperature">Temperature:</label>
                <input type="number" id="temperature" value="68" />
            </div>
            <button onclick="submitReport()">Submit Report</button>
        </div>

        <div class="endpoint">
            <div class="method">GET /stats</div>
            <p>Get aggregated pool statistics (last 5 minutes):</p>
            <button onclick="getStats()">Get Statistics</button>
        </div>

        <div class="endpoint">
            <div class="method">GET /health</div>
            <p>Health check endpoint:</p>
            <button onclick="healthCheck()">Health Check</button>
        </div>

        <div class="stats" id="output">
            <h3>Output:</h3>
            <div id="stats-output">Click any button to test the API endpoints</div>
        </div>
    </div>

    <script>
        async function submitReport() {
            const report = {
                worker_id: document.getElementById('worker_id').value,
                pool: document.getElementById('pool').value,
                hashrate: parseFloat(document.getElementById('hashrate').value),
                temperature: parseInt(document.getElementById('temperature').value),
                timestamp: Math.floor(Date.now() / 1000)
            };
            
            try {
                const response = await fetch('/report', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify(report)
                });
                
                if (response.ok) {
                    document.getElementById('stats-output').textContent = 'Report submitted successfully!';
                } else {
                    const errorText = await response.text();
                    document.getElementById('stats-output').textContent = `Error: ${errorText}`;
                }
            } catch (error) {
                document.getElementById('stats-output').textContent = `Network error: ${error.message}`;
            }
        }

        async function getStats() {
            try {
                const response = await fetch('/stats');
                const data = await response.json();
                document.getElementById('stats-output').textContent = JSON.stringify(data, null, 2);
            } catch (error) {
                document.getElementById('stats-output').textContent = `Error: ${error.message}`;
            }
        }

        async function healthCheck() {
            try {
                const response = await fetch('/health');
                const data = await response.text();
                document.getElementById('stats-output').textContent = `Health Status: ${data}`;
            } catch (error) {
                document.getElementById('stats-output').textContent = `Error: ${error.message}`;
            }
        }
    </script>
</body>
</html>
    "#)
}

#[tokio::main]
async fn main() {
    // Initialize application state with in-memory storage
    let app_state = AppState::new();

    // Create the router with all endpoints
    let app = Router::new()
        .route("/", get(web_interface))
        .route("/report", post(post_report))
        .route("/stats", get(get_stats))
        .route("/health", get(health_check))
        .with_state(app_state);

    // Start the server
    let listener = TcpListener::bind("0.0.0.0:5000")
        .await
        .expect("Failed to bind to address");

    println!("Mining pool service started on http://0.0.0.0:5000");
    println!("Using in-memory storage");
    println!("Endpoints:");
    println!("  POST /report - Submit worker report");
    println!("  GET /stats - Get pool statistics");
    println!("  GET /health - Health check");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use chrono::Utc;

    #[tokio::test]
    async fn test_post_report_valid() {
        let state = AppState::new();
        let report = WorkerReport {
            worker_id: "worker-123".to_string(),
            pool: "us-east".to_string(),
            hashrate: 50.5,
            temperature: 68,
            timestamp: Utc::now().timestamp(),
        };

        let result = post_report(State(state.clone()), Json(report)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_post_report_invalid_worker_id() {
        let state = AppState::new();
        let report = WorkerReport {
            worker_id: "".to_string(),
            pool: "us-east".to_string(),
            hashrate: 50.5,
            temperature: 68,
            timestamp: Utc::now().timestamp(),
        };

        let result = post_report(State(state.clone()), Json(report)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_stats_empty() {
        let state = AppState::new();
        let result = get_stats(State(state)).await;
        assert!(result.is_ok());
        let stats = result.unwrap().0;
        assert!(stats.pools.is_empty());
    }
}
