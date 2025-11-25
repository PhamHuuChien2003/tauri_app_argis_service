#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use std::path::PathBuf;
use tiny_http::{Header, Method, Response, Server};
use tauri::{
    Emitter,
    Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    generate_context, Builder,
};
use tauri_plugin_shell::ShellExt;
use reqwest;

#[derive(Deserialize)]
struct IncomingData {
    lat: f64,
    lng: f64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ExampleResult {
    pub status: String,
    pub address: String,
    pub province: String,
    pub district: String,
    pub ward: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_vn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_en: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_ex: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poi_st_sd: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_num: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buaname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub st_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_com: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fax: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub brandname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gen_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub perform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classify: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dtrend: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub be_id: Option<String>,
}

// Struct để parse response từ Goong API
#[derive(Debug, Deserialize)]
struct GoongResponse {
    results: Vec<GoongResult>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct GoongResult {
    formatted_address: Option<String>,
    address_components: Option<Vec<AddressComponent>>,
    name: Option<String>,
    types: Option<Vec<String>>,
    place_id: Option<String>,
    compound: Option<Compound>,
    address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddressComponent {
    long_name: String,
    short_name: String,
}

#[derive(Debug, Deserialize)]
struct Compound {
    district: Option<String>,
    commune: Option<String>,
    province: Option<String>,
}

// Cấu hình API
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiConfig {
    provider: String, // "goong" hoặc "google"
    goong_api_key: String,
    google_api_key: String,
}

// State để lưu trữ window và dữ liệu mới nhất
struct AppState {
    window: Arc<Mutex<Option<tauri::WebviewWindow>>>,
    latest_data: Arc<Mutex<Option<ExampleResult>>>,
    pending_requests: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<ExampleResult>>>>,
    api_config: Arc<Mutex<ApiConfig>>,
}

// Hàm lưu cấu hình vào file
fn save_config(config: &ApiConfig) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    let config_dir = config_path.parent().unwrap();
    
    // Tạo thư mục nếu chưa tồn tại
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    
    let config_json = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, config_json)?;
    println!("Configuration saved to: {:?}", config_path);
    Ok(())
}

// Hàm load cấu hình từ file
fn load_config() -> ApiConfig {
    match get_config_path() {
        Ok(config_path) => {
            if config_path.exists() {
                match fs::read_to_string(&config_path) {
                    Ok(content) => {
                        match serde_json::from_str::<ApiConfig>(&content) {
                            Ok(config) => {
                                println!("Configuration loaded from: {:?}", config_path);
                                return config;
                            }
                            Err(e) => {
                                println!("Error parsing config file: {}, using default", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading config file: {}, using default", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Error getting config path: {}, using default", e);
        }
    }
    
    // Trả về config mặc định nếu không load được
    ApiConfig::default()
}

// Hàm lấy đường dẫn file config
fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir().ok_or("Cannot find config directory")?;
    path.push("GeocoderApp");
    path.push("config.json");
    Ok(path)
}

// Hàm gọi API Goong.io
async fn call_goong_api(lat: f64, lng: f64, api_key: &str) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let url = format!(
        "https://rsapi.goong.io/v2/geocode?latlng={},{}&limit=5&api_key={}&has_deprecated_administrative_unit=true",
        lat, lng, api_key
    );

    println!("Calling Goong API: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    println!("Goong API response: {}", response_text);

    let goong_response: GoongResponse = serde_json::from_str(&response_text)?;
    
    if goong_response.status != "OK" {
        return Err(format!("Goong API returned status: {}", goong_response.status).into());
    }

    // Lấy kết quả đầu tiên
    if let Some(first_result) = goong_response.results.get(0) {
        println!("Processing first result: {:?}", first_result);

        // Sử dụng compound field để lấy thông tin hành chính
        let (province, district, ward) = if let Some(compound) = &first_result.compound {
            (
                compound.province.clone().unwrap_or_default(),
                compound.district.clone().unwrap_or_default(),
                compound.commune.clone().unwrap_or_default(),
            )
        } else {
            // Fallback: parse từ address_components nếu không có compound
            let mut province = String::new();
            let mut district = String::new();
            let mut ward = String::new();

            if let Some(components) = &first_result.address_components {
                // Dựa vào thứ tự trong mảng address_components
                match components.len() {
                    4 => {
                        // Format: [số nhà, phường, quận, thành phố]
                        ward = components.get(1).map(|c| c.long_name.clone()).unwrap_or_default();
                        district = components.get(2).map(|c| c.long_name.clone()).unwrap_or_default();
                        province = components.get(3).map(|c| c.long_name.clone()).unwrap_or_default();
                    }
                    3 => {
                        // Format: [phường, quận, thành phố]
                        ward = components.get(0).map(|c| c.long_name.clone()).unwrap_or_default();
                        district = components.get(1).map(|c| c.long_name.clone()).unwrap_or_default();
                        province = components.get(2).map(|c| c.long_name.clone()).unwrap_or_default();
                    }
                    _ => {
                        // Try to find by name pattern
                        for component in components {
                            let name = &component.long_name;
                            if name.contains("Hà Nội") || name.contains("Hồ Chí Minh") || name.contains("Đà Nẵng") {
                                province = name.clone();
                            } else if name.contains("Quận") || name.contains("Huyện") {
                                district = name.clone();
                            } else if name.contains("Phường") || name.contains("Xã") {
                                ward = name.clone();
                            }
                        }
                    }
                }
            }

            (province, district, ward)
        };

        // Extract house number and street name from name or formatted_address
        let (house_num, st_name) = extract_address_parts(
            first_result.name.as_deref().unwrap_or(""),
            first_result.formatted_address.as_deref().unwrap_or("")
        );

        let result = ExampleResult {
            status: "ok".into(),
            address: first_result.formatted_address.clone().unwrap_or_else(|| "Unknown address".into()),
            province: clean_province_name(&province),
            district: clean_district_name(&district),
            ward: clean_ward_name(&ward),

            poi_vn: first_result.name.clone(),
            poi_en: None,
            poi_ex: None,

            r#type: first_result.types.as_ref().and_then(|types| {
                if types.is_empty() {
                    Some("address".into())
                } else {
                    types.get(0).cloned()
                }
            }),
            sub_type: None,
            poi_st_sd: Some("Standard POI".into()),

            room: None,
            house_num: if house_num.is_empty() { None } else { Some(house_num) },
            buaname: None,
            st_name: if st_name.is_empty() { None } else { Some(st_name) },
            sub_com: None,

            phone: None,
            fax: None,
            web: None,
            mail: None,

            brandname: None,
            import: None,
            status_detail: Some("active".into()),
            note: None,
            dine: None,
            update_: Some("2025-01-01".into()),
            source: Some("goong".into()),
            gen_type: Some("public".into()),
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,

            google_id: first_result.place_id.clone(),
            be_id: None,
        };

        println!("Processed result: {:?}", result);
        Ok(result)
    } else {
        println!("No results found in Goong API response");
        Ok(ExampleResult {
            status: "error".into(),
            address: "No results found".into(),
            province: "".into(),
            district: "".into(),
            ward: "".into(),

            poi_vn: None,
            poi_en: None,
            poi_ex: None,

            r#type: None,
            sub_type: None,
            poi_st_sd: None,

            room: None,
            house_num: None,
            buaname: None,
            st_name: None,
            sub_com: None,

            phone: None,
            fax: None,
            web: None,
            mail: None,

            brandname: None,
            import: None,
            status_detail: Some("no_results".into()),
            note: None,
            dine: None,
            update_: None,
            source: None,
            gen_type: None,
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,

            google_id: None,
            be_id: None,
        })
    
    }
}

// Hàm gọi API Google Geocoding
async fn call_google_api(lat: f64, lng: f64, api_key: &str) -> Result<ExampleResult, Box<dyn std::error::Error>> {
    let url = format!(
        "https://maps.googleapis.com/maps/api/geocode/json?latlng={},{}&key={}",
        lat, lng, api_key
    );

    println!("Calling Google API: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("API request failed with status: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    println!("Google API response: {}", response_text);

    #[derive(Debug, Deserialize)]
    struct GoogleResponse {
        results: Vec<GoogleResult>,
        status: String,
    }

    #[derive(Debug, Deserialize)]
    struct GoogleResult {
        formatted_address: String,
        address_components: Vec<GoogleAddressComponent>,
        types: Vec<String>,
        place_id: String,
    }

    #[derive(Debug, Deserialize)]
    struct GoogleAddressComponent {
        long_name: String,
        short_name: String,
        types: Vec<String>,
    }

    let google_response: GoogleResponse = serde_json::from_str(&response_text)?;
    
    if google_response.status != "OK" {
        return Err(format!("Google API returned status: {}", google_response.status).into());
    }

    if let Some(first_result) = google_response.results.get(0) {
        let mut province = String::new();
        let mut district = String::new();
        let mut ward = String::new();

        for component in &first_result.address_components {
            if component.types.contains(&"administrative_area_level_1".to_string()) {
                province = component.long_name.clone();
            } else if component.types.contains(&"administrative_area_level_2".to_string()) {
                district = component.long_name.clone();
            } else if component.types.contains(&"administrative_area_level_3".to_string()) ||
                      component.types.contains(&"sublocality".to_string()) {
                ward = component.long_name.clone();
            }
        }

        let (house_num, st_name) = extract_address_parts(
            "",
            &first_result.formatted_address
        );

        let result = ExampleResult {
            status: "ok".into(),
            address: first_result.formatted_address.clone(),
            province: clean_province_name(&province),
            district: clean_district_name(&district),
            ward: clean_ward_name(&ward),
            poi_vn: None,
            poi_en: None,
            poi_ex: None,
            r#type: first_result.types.get(0).cloned(),
            sub_type: None,
            poi_st_sd: Some("Standard POI".into()),
            room: None,
            house_num: if house_num.is_empty() { None } else { Some(house_num) },
            buaname: None,
            st_name: if st_name.is_empty() { None } else { Some(st_name) },
            sub_com: None,
            phone: None,
            fax: None,
            web: None,
            mail: None,
            brandname: None,
            import: None,
            status_detail: Some("active".into()),
            note: None,
            dine: None,
            update_: Some("2025-01-01".into()),
            source: Some("google".into()),
            gen_type: Some("public".into()),
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,
            google_id: Some(first_result.place_id.clone()),
            be_id: None,
        };

        println!("Processed Google result: {:?}", result);
        Ok(result)
    } else {
                Ok(ExampleResult {
            status: "error".into(),
            address: "No results found".into(),
            province: "".into(),
            district: "".into(),
            ward: "".into(),

            poi_vn: None,
            poi_en: None,
            poi_ex: None,

            r#type: None,
            sub_type: None,
            poi_st_sd: None,

            room: None,
            house_num: None,
            buaname: None,
            st_name: None,
            sub_com: None,

            phone: None,
            fax: None,
            web: None,
            mail: None,

            brandname: None,
            import: None,
            status_detail: Some("no_results".into()),
            note: None,
            dine: None,
            update_: None,
            source: None,
            gen_type: None,
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,

            google_id: None,
            be_id: None,
        })
    }
}



// Hàm trích xuất số nhà và tên đường từ name và formatted_address
fn extract_address_parts(name: &str, formatted_address: &str) -> (String, String) {
    let mut house_num = String::new();
    let mut st_name = String::new();

    // Ưu tiên phân tích từ name trước
    if !name.is_empty() {
        let parts: Vec<&str> = name.split_whitespace().collect();
        if !parts.is_empty() {
            if let Some(first_part) = parts.get(0) {
                if first_part.chars().all(|c| c.is_ascii_digit()) {
                    house_num = first_part.to_string();
                    st_name = parts[1..].join(" ");
                } else {
                    st_name = name.to_string();
                }
            }
        }
    }

    // Nếu không tìm thấy từ name, thử từ formatted_address
    if house_num.is_empty() && !formatted_address.is_empty() {
        let parts: Vec<&str> = formatted_address.split(',').collect();
        if let Some(first_part) = parts.get(0) {
            let first_part = first_part.trim();
            let sub_parts: Vec<&str> = first_part.split_whitespace().collect();
            if !sub_parts.is_empty() && sub_parts[0].chars().all(|c| c.is_ascii_digit()) {
                house_num = sub_parts[0].to_string();
                if sub_parts.len() > 1 {
                    st_name = sub_parts[1..].join(" ");
                }
            }
        }
    }

    (house_num, st_name)
}

// Hàm làm sạch tên tỉnh/thành phố
fn clean_province_name(province: &str) -> String {
    province
        .replace("Thành phố ", "")
        .replace("Tỉnh ", "")
        .trim()
        .to_string()
}

// Hàm làm sạch tên quận/huyện
fn clean_district_name(district: &str) -> String {
    district
        .replace("Quận ", "")
        .replace("Huyện ", "")
        .trim()
        .to_string()
}

// Hàm làm sạch tên phường/xã
fn clean_ward_name(ward: &str) -> String {
    ward
        .replace("Phường ", "")
        .replace("Xã ", "")
        .trim()
        .to_string()
}

fn start_local_server(app_state: Arc<AppState>) {
    thread::spawn(move || {
        let server = Server::http("127.0.0.1:31203").unwrap();
        println!("Tauri server listening on http://127.0.0.1:31203");

        loop {
            let mut request = match server.recv() {
                Ok(rq) => rq,
                Err(e) => {
                    println!("Server error: {}", e);
                    continue;
                }
            };

            if request.method() == &Method::Post && request.url() == "/process" {
                println!("Received request from Addin!");

                // Đọc body
                let mut content = String::new();
                if let Err(e) = request.as_reader().read_to_string(&mut content) {
                    println!("Error reading request body: {}", e);
                    let response = Response::from_string(format!("Error reading body: {}", e));
                    request.respond(response).unwrap();
                    continue;
                }
                println!("Raw data = {}", content);

                let parsed: IncomingData = match serde_json::from_str(&content) {
                    Ok(data) => data,
                    Err(e) => {
                        println!("Error parsing JSON: {}", e);
                        let response = Response::from_string(format!("Error parsing JSON: {}", e));
                        request.respond(response).unwrap();
                        continue;
                    }
                };
                println!("Lat = {}, Lon = {}", parsed.lat, parsed.lng);

                // Clone app_state để sử dụng trong async block
                let state_clone = Arc::clone(&app_state);
                
                // Tạo channel để đợi confirm từ UI
                let (tx, rx) = tokio::sync::oneshot::channel();
                
                // Thêm request vào pending
                {
                    if let Ok(mut pending) = state_clone.pending_requests.lock() {
                        pending.push(tx);
                    }
                }
                
                // Tạo runtime cho async function
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                let response_json = rt.block_on(async {
                    // Lấy cấu hình API hiện tại
                    let config = {
                        if let Ok(config_lock) = state_clone.api_config.lock() {
                            config_lock.clone()
                        } else {
                            ApiConfig::default()
                        }
                    };

                    let result = match config.provider.as_str() {
                        "google" => {
                            if !config.google_api_key.is_empty() {
                                call_google_api(parsed.lat, parsed.lng, &config.google_api_key).await
                            } else {
                                Err("Google API key not configured".into())
                            }
                        }
                        _ => {
                            call_goong_api(parsed.lat, parsed.lng, &config.goong_api_key).await
                        }
                    };

                    match result {
                        Ok(result) => {
                            // Lưu dữ liệu mới nhất vào state
                            if let Ok(mut latest_data) = state_clone.latest_data.lock() {
                                *latest_data = Some(result.clone());
                            }
                            
                            // Gửi sự kiện đến frontend để hiển thị popup
                            if let Ok(window_lock) = state_clone.window.lock() {
                                if let Some(window) = &*window_lock {
                                    let _ = window.emit("show-confirm-dialog", &result); 
                                    println!("Emitted show-confirm-dialog event to frontend");
                                }
                            }
                            
                            // Đợi confirm từ UI (timeout sau 30 giây)
                            match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
                                Ok(Ok(confirmed_result)) => {
                                    println!("Request confirmed by user");
                                    confirmed_result
                                }
                                Ok(Err(_)) => {
                                    println!("Confirm channel error");
                                    ExampleResult {
                                        status: "error".into(),
                                        address: "User confirmation failed".into(),
                                        ..Default::default()
                                    }
                                }
                                Err(_) => {
                                    println!("Confirm timeout");
                                    ExampleResult {
                                        status: "error".into(),
                                        address: "Confirmation timeout".into(),
                                        ..Default::default()
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            println!("Error calling API: {}", e);
                            ExampleResult {
                                status: "error".into(),
                                address: format!("API Error: {}", e),
                                ..Default::default()
                            }
                        }
                    }
                });

                let resp_text = match serde_json::to_string(&response_json) {
                    Ok(text) => text,
                    Err(e) => {
                        println!("Error serializing response: {}", e);
                        format!("{{\"status\":\"error\",\"address\":\"Serialization error: {}\"}}", e)
                    }
                };

                let response = Response::from_string(resp_text)
                    .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap());

                if let Err(e) = request.respond(response) {
                    println!("Error sending response: {}", e);
                }
                continue;
            }

            // Handle other routes
            request
                .respond(Response::from_string("Invalid route"))
                .unwrap();
        }
    });
}

// ==================== TAURI COMMANDS ====================

// Command để confirm kết quả
#[tauri::command]
fn confirm_result(result: ExampleResult, state: tauri::State<Arc<AppState>>) -> bool {
    println!("Confirming result: {:?}", result);
    
    if let Ok(mut pending) = state.pending_requests.lock() {
        if let Some(tx) = pending.pop() {
            let _ = tx.send(result);
            return true;
        }
    }
    false
}

// Command để reject kết quả
#[tauri::command]
fn reject_result(state: tauri::State<Arc<AppState>>) -> bool {
    println!("Rejecting result");
    
    if let Ok(mut pending) = state.pending_requests.lock() {
        if let Some(tx) = pending.pop() {
            let error_result = ExampleResult {
                status: "rejected".into(),
                address: "User rejected the result".into(),
                ..Default::default()
            };
            let _ = tx.send(error_result);
            return true;
        }
    }
    false
}

// Command để mở rộng cửa sổ
#[tauri::command]
async fn expand_window(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("Expanding window");
    
    window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
        width: 800.0, 
        height: 600.0 
    })).map_err(|e| e.to_string())?;
    
    window.center().map_err(|e| e.to_string())?;
    window.set_always_on_top(true).map_err(|e| e.to_string())?;
    
    Ok(())
}

// Command để thu nhỏ cửa sổ
#[tauri::command]
async fn collapse_window(window: tauri::WebviewWindow) -> Result<(), String> {
    println!("Collapsing window");
    
    window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
        width: 60.0, 
        height: 60.0 
    })).map_err(|e| e.to_string())?;
    
    // Đặt vị trí góc trên bên phải
    if let Ok(monitor) = window.primary_monitor() {
        if let Some(monitor) = monitor {
            let screen_size = monitor.size();
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: screen_size.width as f64 - 80.0,
                y: 20.0,
            }));
        }
    }
    
    Ok(())
}

// Command để lấy vị trí cửa sổ
#[tauri::command]
async fn get_window_position(window: tauri::WebviewWindow) -> Result<(f64, f64), String> {
    let position = window
        .inner_position()
        .map_err(|e| e.to_string())?;
    
    Ok((position.x as f64, position.y as f64))
}

// Command để đặt vị trí cửa sổ
#[tauri::command]
async fn set_window_position(window: tauri::WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// Command để lấy dữ liệu mới nhất
#[tauri::command]
fn get_latest_data(state: tauri::State<Arc<AppState>>) -> Option<ExampleResult> {
    if let Ok(data) = state.latest_data.lock() {
        data.clone()
    } else {
        None
    }
}

// Command để lấy cấu hình API
#[tauri::command]
fn get_api_config(state: tauri::State<Arc<AppState>>) -> Result<ApiConfig, String> {
    if let Ok(config) = state.api_config.lock() {
        Ok(config.clone())
    } else {
        Err("Failed to get API config".to_string())
    }
}

// Command để cập nhật cấu hình API
#[tauri::command]
fn update_api_config(new_config: ApiConfig, state: tauri::State<Arc<AppState>>) -> Result<(), String> {
    if let Ok(mut config) = state.api_config.lock() {
        *config = new_config.clone();
        
        // Lưu cấu hình vào file
        if let Err(e) = save_config(&new_config) {
            println!("Error saving config: {}", e);
            return Err(format!("Failed to save config: {}", e));
        }
        
        Ok(())
    } else {
        Err("Failed to update API config".to_string())
    }
}

// Implement Default cho ExampleResult
impl Default for ExampleResult {
    fn default() -> Self {
        Self {
            status: "error".into(),
            address: "".into(),
            province: "".into(),
            district: "".into(),
            ward: "".into(),
            poi_vn: None,
            poi_en: None,
            poi_ex: None,
            r#type: None,
            sub_type: None,
            poi_st_sd: None,
            room: None,
            house_num: None,
            buaname: None,
            st_name: None,
            sub_com: None,
            phone: None,
            fax: None,
            web: None,
            mail: None,
            brandname: None,
            import: None,
            status_detail: None,
            note: None,
            dine: None,
            update_: None,
            source: None,
            gen_type: None,
            perform: None,
            dup: None,
            explain: None,
            classify: None,
            dtrend: None,
            google_id: None,
            be_id: None,
        }
    }
}

// Implement Default cho ApiConfig
impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider: "goong".to_string(),
            goong_api_key: "T4B6StzJYTsTEyxA0u9I01593mA1yclUffVMODpx".to_string(),
            google_api_key: "".to_string(),
        }
    }
}

fn main() {
    // Load cấu hình từ file khi khởi động
    let initial_config = load_config();

    let app_state = Arc::new(AppState {
        window: Arc::new(Mutex::new(None)),
        latest_data: Arc::new(Mutex::new(None)),
        pending_requests: Arc::new(Mutex::new(Vec::new())),
        api_config: Arc::new(Mutex::new(initial_config)), 
    });

    let state_clone = Arc::clone(&app_state);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // Tạo system tray menu
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
            let provider_item = MenuItem::with_id(app, "provider", "Select Provider", true, None::<&str>)?;
            let goong_key_item = MenuItem::with_id(app, "goong_key", "Set Goong API Key", true, None::<&str>)?;
            let google_key_item = MenuItem::with_id(app, "google_key", "Set Google API Key", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            // Tạo separator items
            let separator1 = MenuItem::with_id(app, "sep1", "---", false, None::<&str>)?;
            let separator2 = MenuItem::with_id(app, "sep2", "---", false, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[
                &show_item,
                &hide_item,
                &separator1,
                &provider_item,
                &goong_key_item,
                &google_key_item,
                &separator2,
                &quit_item,
            ])?;

            // Tạo system tray
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false) // Chỉ hiện menu khi click phải
                .on_menu_event(move |app, event| {
                    let window = app.get_webview_window("main").unwrap();
                    match event.id.as_ref() {
                        "show" => {
                            println!("Show window menu item clicked");
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "hide" => {
                            println!("Hide window menu item clicked");
                            let _ = window.hide();
                        }
                        "provider" => {
                            println!("Provider menu item clicked");
                            let _ = window.emit("open-provider-selector", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "goong_key" => {
                            println!("Goong API key menu item clicked");
                            let _ = window.emit("open-goong-key-input", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "google_key" => {
                            println!("Google API key menu item clicked");
                            let _ = window.emit("open-google-key-input", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "quit" => {
                            println!("Quit menu item clicked");
                            app.exit(0);
                        }
                        "sep1" | "sep2" => {
                            // Bỏ qua separator items
                        }
                        _ => {
                            println!("Unknown menu item: {:?}", event.id);
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } => {
                            println!("Left click on tray icon");
                            let app = tray.app_handle();
                            let window = app.get_webview_window("main").unwrap();
                            if window.is_visible().unwrap() {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let main_window = app.get_webview_window("main").unwrap();
            
            // Cấu hình window floating - Ẩn khỏi taskbar
            main_window.set_always_on_top(true).unwrap();
            main_window.set_decorations(false).unwrap();
            main_window.set_skip_taskbar(true).unwrap(); // Ẩn khỏi taskbar
            
            // Đặt kích thước nhỏ (icon)
            main_window.set_size(tauri::Size::Logical(tauri::LogicalSize { 
                width: 60.0, 
                height: 60.0 
            })).unwrap();
            
            // Đặt vị trí góc trên bên phải
            if let Ok(monitor) = main_window.primary_monitor() {
                if let Some(monitor) = monitor {
                    let screen_size = monitor.size();
                    let _ = main_window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                        x: screen_size.width as f64 - 80.0,
                        y: 20.0,
                    }));
                }
            }

            // Ẩn window khi khởi động, chỉ hiện system tray
            let _ = main_window.hide();

            // Lưu window reference vào state
            if let Ok(mut window_lock) = state_clone.window.lock() {
                *window_lock = Some(main_window);
            }
            
            start_local_server(state_clone);
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_latest_data, 
            confirm_result, 
            reject_result,
            expand_window,
            collapse_window,
            get_window_position,  
            set_window_position,
            get_api_config,
            update_api_config
        ])
        .run(generate_context!())
        .expect("error while running Tauri application");
}